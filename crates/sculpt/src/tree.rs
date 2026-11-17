//! A tree of boxes round a shape's polygons that follows the points as
//! they move (decision 0304): built once when sculpting starts, then only
//! the boxes round moved polygons are made again ("refitted", never
//! rebuilt), so finding the surface under the mouse and the points in a
//! brush's reach stay quick while a stroke reshapes millions of polygons.
//!
//! Built from the polygons' middles in Morton order (a curve through
//! space that keeps neighbours together): sorting is the only work that
//! isn't a straight pass, so even three million polygons take a fraction
//! of a second.

use glam::Vec3;
use mesh::{FaceId, Mesh, VertexId};

/// The most polygons a leaf holds.
const LEAF: usize = 8;
/// A node no polygon reaches (the root's parent).
const NONE: u32 = u32::MAX;

/// A box, as its smallest and largest corner.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    const EMPTY: Self = Self {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };

    fn grow(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    fn distance_squared(&self, p: Vec3) -> f32 {
        let d = (self.min - p).max(p - self.max).max(Vec3::ZERO);
        d.length_squared()
    }

    /// Where the line `origin + t · way` enters the box (t ≥ 0), if it
    /// meets it before `limit`.
    fn entry(&self, origin: Vec3, inverse: Vec3, limit: f32) -> Option<f32> {
        let a = (self.min - origin) * inverse;
        let b = (self.max - origin) * inverse;
        let near = a.min(b).max_element().max(0.0);
        let far = a.max(b).min_element().min(limit);
        (near <= far).then_some(near)
    }
}

#[derive(Debug, Clone, Copy)]
struct Node {
    bounds: Aabb,
    /// A leaf's first polygon in `order`; otherwise its first child (the
    /// second follows it).
    first: u32,
    /// How many polygons a leaf holds; 0 for a node with children.
    count: u32,
    parent: u32,
}

/// Where a line met the surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    /// How far along the line.
    pub distance: f32,
    /// The place.
    pub point: Vec3,
    /// The polygon.
    pub face: FaceId,
    /// Which way the polygon faces there.
    pub normal: Vec3,
}

/// The tree (see the module notes).
#[derive(Debug, Clone, Default)]
pub struct FaceTree {
    nodes: Vec<Node>,
    /// The polygons' slots, in the order the leaves hold them.
    order: Vec<u32>,
    /// Each polygon slot's leaf ([`NONE`] for a slot with no polygon).
    leaf_of: Vec<u32>,
    /// Each node's mark from the last refit that reached it.
    marks: Vec<u32>,
    mark: u32,
}

impl FaceTree {
    /// The tree round `mesh`'s polygons.
    pub fn new(mesh: &Mesh) -> Self {
        let slots = mesh.face_slots().count();
        let mut middles: Vec<(u64, u32)> = Vec::with_capacity(mesh.face_count());
        let mut whole = Aabb::EMPTY;
        let mut centres = Vec::with_capacity(mesh.face_count());
        for (face, corners) in mesh.faces() {
            let mut sum = Vec3::ZERO;
            let mut n = 0.0;
            for v in corners {
                if let Some(p) = mesh.position(*v) {
                    sum += p;
                    n += 1.0;
                }
            }
            if n == 0.0 || !sum.is_finite() {
                continue;
            }
            let c = sum / n;
            whole.grow(c);
            centres.push((face.get(), c));
        }
        let size = (whole.max - whole.min).max(Vec3::splat(1e-12));
        for (face, c) in centres {
            let t = ((c - whole.min) / size).clamp(Vec3::ZERO, Vec3::ONE);
            middles.push((morton(t), face));
        }
        middles.sort_unstable();
        let mut tree = Self {
            nodes: Vec::with_capacity(middles.len() / LEAF * 2 + 1),
            order: middles.iter().map(|(_, f)| *f).collect(),
            leaf_of: vec![NONE; slots],
            marks: Vec::new(),
            mark: 0,
        };
        if middles.is_empty() {
            return tree;
        }
        let codes: Vec<u64> = middles.iter().map(|(c, _)| *c).collect();
        tree.build(&codes);
        tree.fit_all(mesh);
        tree
    }

    /// Splits runs of polygons at the highest bit their codes differ in
    /// until each leaf holds at most [`LEAF`]; children always come after
    /// their parent.
    fn build(&mut self, codes: &[u64]) {
        self.nodes.push(Node {
            bounds: Aabb::EMPTY,
            first: 0,
            count: codes.len() as u32,
            parent: NONE,
        });
        let mut waiting = vec![0usize];
        while let Some(at) = waiting.pop() {
            let (start, count) = (self.nodes[at].first as usize, self.nodes[at].count as usize);
            if count <= LEAF {
                for &f in &self.order[start..start + count] {
                    self.leaf_of[f as usize] = at as u32;
                }
                continue;
            }
            let run = &codes[start..start + count];
            let (a, b) = (run[0], run[count - 1]);
            let half = if a == b {
                count / 2
            } else {
                let bit = 63 - (a ^ b).leading_zeros();
                let split = run.partition_point(|c| (c >> bit) & 1 == 0);
                if split == 0 || split == count {
                    count / 2
                } else {
                    split
                }
            };
            let left = self.nodes.len();
            for (first, n) in [(start, half), (start + half, count - half)] {
                self.nodes.push(Node {
                    bounds: Aabb::EMPTY,
                    first: first as u32,
                    count: n as u32,
                    parent: at as u32,
                });
            }
            self.nodes[at].first = left as u32;
            self.nodes[at].count = 0;
            waiting.push(left);
            waiting.push(left + 1);
        }
    }

    /// Works out every box (children after parents: so backwards).
    fn fit_all(&mut self, mesh: &Mesh) {
        for at in (0..self.nodes.len()).rev() {
            self.fit_node(mesh, at);
        }
    }

    fn fit_node(&mut self, mesh: &Mesh, at: usize) {
        let node = self.nodes[at];
        let bounds = if node.count > 0 {
            self.leaf_bounds(mesh, at)
        } else {
            let c = node.first as usize;
            self.nodes[c].bounds.union(self.nodes[c + 1].bounds)
        };
        self.nodes[at].bounds = bounds;
    }

    /// Fits the boxes again round the polygons around `points`, which
    /// moved, and every box above them.
    pub fn refit_points(&mut self, mesh: &Mesh, points: &[VertexId]) {
        let topology = mesh.topology();
        self.refit(
            mesh,
            points
                .iter()
                .flat_map(|v| topology.vertex_faces(*v).iter().copied()),
        );
    }

    /// Fits the boxes again round `faces`, whose points moved, and every
    /// box above them.
    pub fn refit(&mut self, mesh: &Mesh, faces: impl IntoIterator<Item = FaceId>) {
        if self.nodes.is_empty() {
            return;
        }
        if self.marks.len() != self.nodes.len() {
            self.marks = vec![0; self.nodes.len()];
        }
        self.mark = self.mark.wrapping_add(1);
        if self.mark == 0 {
            self.marks.fill(0);
            self.mark = 1;
        }
        let mark = self.mark;
        let mut leaves: Vec<u32> = Vec::new();
        for f in faces {
            let Some(&leaf) = self.leaf_of.get(f.get() as usize) else {
                continue;
            };
            if leaf != NONE && self.marks[leaf as usize] != mark {
                self.marks[leaf as usize] = mark;
                leaves.push(leaf);
            }
        }
        // The leaves' boxes, on every core when there are many.
        let boxes = mesh::parallel::in_parallel(leaves.len() * LEAF, |range| {
            // Whole leaves each: the count is a multiple of LEAF.
            let (from, to) = (range.start / LEAF, (range.end / LEAF).min(leaves.len()));
            leaves[from..to]
                .iter()
                .map(|l| self.leaf_bounds(mesh, *l as usize))
                .collect::<Vec<_>>()
        });
        let mut at = 0;
        for part in boxes {
            for b in part {
                if let Some(&leaf) = leaves.get(at) {
                    self.nodes[leaf as usize].bounds = b;
                }
                at += 1;
            }
        }
        // The boxes above them: all of them when many changed (one pass,
        // children after parents, so backwards), else just those above.
        if leaves.len() * 8 > self.nodes.len() {
            for n in (0..self.nodes.len()).rev() {
                if self.nodes[n].count == 0 {
                    self.fit_node(mesh, n);
                }
            }
            return;
        }
        let mut above: Vec<u32> = Vec::new();
        for leaf in leaves {
            let mut up = self.nodes[leaf as usize].parent;
            while up != NONE && self.marks[up as usize] != mark {
                self.marks[up as usize] = mark;
                above.push(up);
                up = self.nodes[up as usize].parent;
            }
        }
        above.sort_unstable_by(|a, b| b.cmp(a));
        for n in above {
            self.fit_node(mesh, n as usize);
        }
    }

    /// The box round leaf `at`'s polygons.
    fn leaf_bounds(&self, mesh: &Mesh, at: usize) -> Aabb {
        let node = self.nodes[at];
        let mut b = Aabb::EMPTY;
        let (start, end) = (node.first as usize, (node.first + node.count) as usize);
        for &f in &self.order[start..end] {
            for v in mesh.face(FaceId::from_raw(f)).unwrap_or(&[]) {
                if let Some(p) = mesh.position(*v) {
                    b.grow(p);
                }
            }
        }
        b
    }

    /// Whether it holds no polygon.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// The nearest place where the line from `origin` along `way` meets a
    /// polygon of `mesh` (seen from either side).
    pub fn ray_hit(&self, mesh: &Mesh, origin: Vec3, way: Vec3) -> Option<RayHit> {
        if self.nodes.is_empty() || !origin.is_finite() || !way.is_finite() {
            return None;
        }
        let inverse = Vec3::ONE / way;
        let mut best: Option<RayHit> = None;
        let mut waiting = vec![0usize];
        while let Some(at) = waiting.pop() {
            let limit = best.map_or(f32::INFINITY, |b| b.distance);
            let node = self.nodes[at];
            if node.bounds.entry(origin, inverse, limit).is_none() {
                continue;
            }
            if node.count == 0 {
                waiting.push(node.first as usize);
                waiting.push(node.first as usize + 1);
                continue;
            }
            let (start, end) = (node.first as usize, (node.first + node.count) as usize);
            for &f in &self.order[start..end] {
                let face = FaceId::from_raw(f);
                let Some(corners) = mesh.face(face) else {
                    continue;
                };
                let points: Vec<Vec3> = corners.iter().filter_map(|v| mesh.position(*v)).collect();
                for k in 1..points.len().saturating_sub(1) {
                    let [a, b, c] = [points[0], points[k], points[k + 1]];
                    if let Some(t) = line_meets_triangle(origin, way, a, b, c)
                        && t < best.map_or(f32::INFINITY, |h| h.distance)
                    {
                        best = Some(RayHit {
                            distance: t,
                            point: origin + way * t,
                            face,
                            normal: (b - a).cross(c - a).normalize_or(Vec3::Z),
                        });
                    }
                }
            }
        }
        best
    }

    /// Calls `found` with each polygon whose box comes within `radius` of
    /// `centre` (and so may have points in reach).
    pub fn near(&self, centre: Vec3, radius: f32, mut found: impl FnMut(FaceId)) {
        if self.nodes.is_empty() || !centre.is_finite() || !(radius >= 0.0) {
            return;
        }
        let reach = radius * radius;
        let mut waiting = vec![0usize];
        while let Some(at) = waiting.pop() {
            let node = self.nodes[at];
            if node.bounds.distance_squared(centre) > reach {
                continue;
            }
            if node.count == 0 {
                waiting.push(node.first as usize);
                waiting.push(node.first as usize + 1);
                continue;
            }
            for &f in &self.order[node.first as usize..(node.first + node.count) as usize] {
                found(FaceId::from_raw(f));
            }
        }
    }

    /// Bytes it holds.
    pub fn memory_size(&self) -> usize {
        self.nodes.capacity() * std::mem::size_of::<Node>()
            + (self.order.capacity() + self.leaf_of.capacity() + self.marks.capacity()) * 4
    }
}

/// Where the line `origin + t · way` meets triangle `a b c` (either side),
/// for t ≥ 0 (Möller and Trumbore's method).
fn line_meets_triangle(origin: Vec3, way: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let (e1, e2) = (b - a, c - a);
    let p = way.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-20 {
        return None;
    }
    let inv = 1.0 / det;
    let s = origin - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = way.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t >= 0.0 && t.is_finite()).then_some(t)
}

/// The Morton code of `t` (each of x, y, z from 0 to 1): 21 bits each,
/// interleaved.
fn morton(t: Vec3) -> u64 {
    let scale = |x: f32| ((x * 2_097_151.0) as u64).min(2_097_151);
    spread(scale(t.x)) | spread(scale(t.y)) << 1 | spread(scale(t.z)) << 2
}

/// The 21 low bits of `x`, two zero bits after each.
fn spread(mut x: u64) -> u64 {
    x &= 0x1f_ffff;
    x = (x | x << 32) & 0x001f_0000_0000_ffff;
    x = (x | x << 16) & 0x001f_0000_ff00_00ff;
    x = (x | x << 8) & 0x100f_00f0_0f00_f00f;
    x = (x | x << 4) & 0x10c3_0c30_c30c_30c3;
    x = (x | x << 2) & 0x1249_2492_4924_9249;
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesh::PrimitiveKind;

    fn ball() -> Mesh {
        mesh::samples::heavy_ball(2_000, 1.0)
    }

    #[test]
    fn a_line_through_the_ball_hits_its_near_side() {
        let m = ball();
        let tree = FaceTree::new(&m);
        let hit = tree
            .ray_hit(&m, Vec3::new(0.0, -10.0, 1.0), Vec3::Y)
            .unwrap();
        assert!((hit.point.y + 1.0).abs() < 0.1, "{hit:?}");
        assert!(
            tree.ray_hit(&m, Vec3::new(5.0, -10.0, 1.0), Vec3::Y)
                .is_none()
        );
    }

    #[test]
    fn near_finds_every_polygon_with_a_point_in_reach() {
        let m = ball();
        let tree = FaceTree::new(&m);
        let centre = Vec3::new(0.0, -1.0, 1.0);
        let mut found = std::collections::BTreeSet::new();
        tree.near(centre, 0.3, |f| {
            found.insert(f);
        });
        for (f, corners) in m.faces() {
            let close = corners
                .iter()
                .any(|v| m.position(*v).unwrap().distance(centre) <= 0.3);
            if close {
                assert!(found.contains(&f));
            }
        }
    }

    #[test]
    fn refitting_follows_moved_points() {
        let mut m = ball();
        let mut tree = FaceTree::new(&m);
        // Pull one point far out: the line through its new place hits.
        let v = VertexId::from_raw(m.vertex_count() as u32 / 2);
        let p = m.position(v).unwrap();
        let out = p + Vec3::new(0.0, 0.0, 0.0) + (p - Vec3::Z).normalize() * 3.0;
        m.set_position(v, out);
        let faces: Vec<FaceId> = m.topology().vertex_faces(v).to_vec();
        tree.refit(&m, faces);
        let from = out + (out - Vec3::Z).normalize() * 5.0;
        let hit = tree.ray_hit(&m, from, (out - from).normalize());
        assert!(hit.is_some_and(|h| h.point.distance(out) < 0.5), "{hit:?}");
    }

    #[test]
    fn an_empty_shape_and_one_polygon_work() {
        let empty = Mesh::new();
        let tree = FaceTree::new(&empty);
        assert!(tree.is_empty());
        assert!(tree.ray_hit(&empty, Vec3::ZERO, Vec3::X).is_none());
        let plane = PrimitiveKind::Plane.standard().mesh();
        let tree = FaceTree::new(&plane);
        assert!(!tree.is_empty());
        let hit = tree.ray_hit(&plane, Vec3::new(0.0, 0.0, 10.0), Vec3::NEG_Z);
        assert!(hit.is_some());
    }
}
