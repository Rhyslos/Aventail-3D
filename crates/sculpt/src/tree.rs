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

use crate::frames::{AT_HAND, at_hand, newell, newell_at, newell_side};
use glam::Vec3;
use mesh::{FaceId, Mesh, VertexId};

/// The most polygons a leaf holds.
const LEAF: usize = 8;
/// A node no polygon reaches (the root's parent).
const NONE: u32 = u32::MAX;
/// The fewest leaves a core refits when the work is shared: a big dab
/// moves the polygons of tens of thousands of leaves.
const LEAVES_PER_CORE: usize = 2048;
/// The most points a leaf's polygons have between them for the leaf to
/// keep them listed ([`FaceTree::corners`]): [`LEAF`] polygons of up to
/// [`AT_HAND`] corners. A leaf with a polygon of more corners is fitted
/// polygon by polygon.
const MOST_CORNERS: usize = LEAF * AT_HAND;
/// How much of a brush's reach a leaf's box must stay within for all its
/// points to count as in reach without asking each ([`FaceTree::leaf_within`]):
/// a hair less than all of it, so rounding can't put one outside.
const WITHIN: f32 = 1.0 - 1e-4;

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

    /// How far its farthest corner is from `p`, squared.
    fn farthest_squared(&self, p: Vec3) -> f32 {
        (p - self.min).abs().max((self.max - p).abs()).length_squared()
    }

    /// Where the line `origin + t · way` enters the box (t ≥ 0), if it
    /// meets it before `limit`.
    fn entry(&self, origin: Vec3, inverse: Vec3, limit: f32) -> Option<f32> {
        let a = (self.min - origin) * inverse;
        let b = (self.max - origin) * inverse;
        let near = a.min(b).max_element().max(0.0);
        let far = a.max(b).min_element();
        // A little slack: a line through a box's edge or corner (a point
        // of the shape seen straight on, say) must not slip past it by
        // rounding.
        let scale = if near.is_finite() && far.is_finite() {
            near.abs().max(far.abs()).max(1.0)
        } else {
            1.0
        };
        let slack = 1e-5 * scale;
        (near <= far + slack && near <= limit).then_some(near)
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
    /// The points each leaf answers for, leaf after leaf: a point is with
    /// the first of its polygons that the tree holds, so each point of a
    /// polygon is with one leaf only, and a dab that goes through the
    /// leaves in its reach meets it once.
    points: Vec<u32>,
    /// Where each node's points start in `points` (one more entry than
    /// there are nodes; a node that isn't a leaf has none).
    point_starts: Vec<u32>,
    /// Each leaf's polygons' area-weighted facings added together, by
    /// node (nothing for a node that isn't a leaf): kept with its box, so
    /// a dab needn't look at every polygon well inside its reach to know
    /// which way they face.
    facings: Vec<Vec3>,
    /// Every point of each leaf's polygons, once each, leaf after leaf:
    /// what its box goes round, looked up once a point, not once for each
    /// polygon that uses it. Nothing for a leaf with a polygon of more
    /// than [`AT_HAND`] corners.
    corners: Vec<u32>,
    /// Where each node's points start in `corners` (one more entry than
    /// there are nodes).
    corner_starts: Vec<u32>,
    /// The outline of each leaf's patch of polygons, leaf after leaf: the
    /// sides of its polygons that no other of its polygons runs back
    /// along, each as where its two ends are among the leaf's `corners`.
    /// The facings of the leaf's polygons add up to what these sides
    /// alone give ([`newell_side`]).
    sides: Vec<[u8; 2]>,
    /// Where each node's sides start in `sides` (one more entry than
    /// there are nodes).
    side_starts: Vec<u32>,
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
            points: Vec::new(),
            point_starts: Vec::new(),
            facings: Vec::new(),
            corners: Vec::new(),
            corner_starts: Vec::new(),
            sides: Vec::new(),
            side_starts: Vec::new(),
        };
        if middles.is_empty() {
            return tree;
        }
        let codes: Vec<u64> = middles.iter().map(|(c, _)| *c).collect();
        tree.build(&codes);
        tree.facings = vec![Vec3::ZERO; tree.nodes.len()];
        tree.list_corners(mesh);
        tree.refit_all(mesh);
        tree.share_points(mesh);
        tree
    }

    /// Lists each leaf's points and outline (see `corners` and `sides`),
    /// on every core.
    fn list_corners(&mut self, mesh: &Mesh) {
        let (nodes, order) = (&self.nodes, &self.order);
        let parts = mesh::parallel::in_parallel(nodes.len(), |range| {
            let mut corners: Vec<u32> = Vec::new();
            let mut sides: Vec<[u8; 2]> = Vec::new();
            // How many of each every node of the range has.
            let mut counts: Vec<(u32, u32)> = Vec::with_capacity(range.len());
            for node in &nodes[range] {
                let (from, sides_from) = (corners.len(), sides.len());
                let faces = &order[node.first as usize..][..node.count as usize];
                if node.count == 0 || !outline(mesh, faces, &mut corners, &mut sides) {
                    corners.truncate(from);
                    sides.truncate(sides_from);
                }
                counts.push(((corners.len() - from) as u32, (sides.len() - sides_from) as u32));
            }
            (corners, sides, counts)
        });
        let (mut corner_starts, mut side_starts) = (vec![0u32], vec![0u32]);
        for (corners, sides, counts) in parts {
            self.corners.extend(corners);
            self.sides.extend(sides);
            for (c, s) in counts {
                corner_starts.push(corner_starts.last().copied().unwrap_or(0) + c);
                side_starts.push(side_starts.last().copied().unwrap_or(0) + s);
            }
        }
        self.corner_starts = corner_starts;
        self.side_starts = side_starts;
    }

    /// Gives each point of a polygon to the leaf of the first of its
    /// polygons that the tree holds (see `points`).
    fn share_points(&mut self, mesh: &Mesh) {
        let topology = mesh.topology();
        let leaf_of = &self.leaf_of;
        let leaves: Vec<u32> = mesh::parallel::in_parallel(mesh.point_slot_count(), |range| {
            range
                .map(|i| {
                    topology
                        .vertex_faces(VertexId::from_raw(i as u32))
                        .iter()
                        .filter_map(|f| leaf_of.get(f.get() as usize).copied())
                        .find(|leaf| *leaf != NONE)
                        .unwrap_or(NONE)
                })
                .collect::<Vec<u32>>()
        })
        .concat();
        // Gathered by leaf (a counting sort: each leaf's points in order).
        let mut starts = vec![0u32; self.nodes.len() + 1];
        for leaf in &leaves {
            if let Some(n) = starts.get_mut(*leaf as usize + 1) {
                *n += 1;
            }
        }
        for n in 1..starts.len() {
            starts[n] += starts[n - 1];
        }
        let mut filled = starts.clone();
        let mut points = vec![0u32; starts.last().copied().unwrap_or(0) as usize];
        for (point, leaf) in leaves.iter().enumerate() {
            if let Some(at) = filled.get_mut(*leaf as usize) {
                points[*at as usize] = point as u32;
                *at += 1;
            }
        }
        self.points = points;
        self.point_starts = starts;
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

    /// Works out the box of node `at`, which isn't a leaf, from its
    /// children's.
    fn fit_node(&mut self, at: usize) {
        let c = self.nodes[at].first as usize;
        self.nodes[at].bounds = self.nodes[c].bounds.union(self.nodes[c + 1].bounds);
    }

    /// Fits the boxes again round the polygons around `points`, which
    /// moved, and every box above them.
    pub fn refit_points(&mut self, mesh: &Mesh, points: &[VertexId]) {
        if points.len() * 2 >= mesh.vertex_count() {
            return self.refit_all(mesh);
        }
        let topology = mesh.topology();
        self.refit(
            mesh,
            points
                .iter()
                .flat_map(|v| topology.vertex_faces(*v).iter().copied()),
        );
    }

    /// Fits the boxes again round `faces`, whose points moved, and every
    /// box above them; every box, on every core, when they are half the
    /// polygons or more (decision 0313).
    pub fn refit_faces(&mut self, mesh: &Mesh, faces: &[FaceId]) {
        if faces.len() * 2 >= self.order.len() {
            self.refit_all(mesh);
        } else {
            self.refit(mesh, faces.iter().copied());
        }
    }

    /// [`Self::refit_leaves`], or every box on every core when `leaves`
    /// hold half the polygons or more.
    pub fn refit_near(&mut self, mesh: &Mesh, leaves: &[u32]) {
        if leaves.len() * LEAF >= self.order.len() {
            self.refit_all(mesh);
        } else {
            self.refit_leaves(mesh, leaves);
        }
    }

    /// Fits every box again: the leaves' on every core, then those above.
    pub fn refit_all(&mut self, mesh: &Mesh) {
        let nodes = &self.nodes;
        let fits = mesh::parallel::in_parallel(nodes.len(), |range| {
            range
                .map(|n| (nodes[n].count > 0).then(|| self.leaf_fit(mesh, n)))
                .collect::<Vec<_>>()
        });
        for (n, fit) in fits.into_iter().flatten().enumerate() {
            if let Some((bounds, facing)) = fit {
                self.nodes[n].bounds = bounds;
                self.facings[n] = facing;
            }
        }
        // Children come after their parents: so backwards.
        for n in (0..self.nodes.len()).rev() {
            if self.nodes[n].count == 0 {
                self.fit_node(n);
            }
        }
    }

    /// Fits the boxes again round `faces`, whose points moved, and every
    /// box above them.
    pub fn refit(&mut self, mesh: &Mesh, faces: impl IntoIterator<Item = FaceId>) {
        let leaves: Vec<u32> = faces
            .into_iter()
            .filter_map(|f| self.leaf_of.get(f.get() as usize).copied())
            .filter(|leaf| *leaf != NONE)
            .collect();
        self.refit_leaves(mesh, &leaves);
    }

    /// Fits the boxes of `leaves` again (as [`Self::near_leaves`] names
    /// them; any of them more than once is fine), whose polygons' points
    /// moved, and every box above them. A dab hands back the leaves it
    /// found, so none is looked up from its polygons again.
    pub fn refit_leaves(&mut self, mesh: &Mesh, leaves: &[u32]) {
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
        // Each leaf once.
        let mut once: Vec<u32> = Vec::with_capacity(leaves.len());
        for &leaf in leaves {
            let Some(seen) = self.marks.get_mut(leaf as usize) else {
                continue;
            };
            if *seen != mark && self.nodes[leaf as usize].count > 0 {
                *seen = mark;
                once.push(leaf);
            }
        }
        let leaves = once;
        // The leaves' boxes and facings, on every core when there are many.
        let fits = mesh::parallel::in_parallel_from(leaves.len(), LEAVES_PER_CORE, |range| {
            leaves[range]
                .iter()
                .map(|l| self.leaf_fit(mesh, *l as usize))
                .collect::<Vec<_>>()
        });
        for (leaf, (bounds, facing)) in leaves.iter().zip(fits.into_iter().flatten()) {
            self.nodes[*leaf as usize].bounds = bounds;
            self.facings[*leaf as usize] = facing;
        }
        // The boxes above them: all of them when many changed (one pass,
        // children after parents, so backwards), else just those above.
        if leaves.len() * 8 > self.nodes.len() {
            for n in (0..self.nodes.len()).rev() {
                if self.nodes[n].count == 0 {
                    self.fit_node(n);
                }
            }
            return;
        }
        for leaf in &leaves {
            let mut up = self.nodes[*leaf as usize].parent;
            while up != NONE && self.marks[up as usize] != mark {
                self.marks[up as usize] = mark;
                up = self.nodes[up as usize].parent;
            }
        }
        // Each marked node after its marked children: down from the top,
        // and fitted on the way back up (no sorting them by depth).
        let mut waiting: Vec<(u32, bool)> = vec![(0, false)];
        while let Some((n, children_done)) = waiting.pop() {
            let node = self.nodes[n as usize];
            if node.count > 0 || self.marks[n as usize] != mark {
                continue;
            }
            if children_done {
                self.fit_node(n as usize);
            } else {
                waiting.push((n, true));
                waiting.push((node.first, false));
                waiting.push((node.first + 1, false));
            }
        }
    }

    /// The box round leaf `at`'s polygons, and their area-weighted facings
    /// added together: from the leaf's listed points and outline, each
    /// point looked up once.
    fn leaf_fit(&self, mesh: &Mesh, at: usize) -> (Aabb, Vec3) {
        let listed = |starts: &[u32]| Some(*starts.get(at)? as usize..*starts.get(at + 1)? as usize);
        let lists = listed(&self.corner_starts)
            .and_then(|range| self.corners.get(range))
            .zip(listed(&self.side_starts).and_then(|range| self.sides.get(range)));
        let Some((corners, sides)) = lists.filter(|(c, _)| !c.is_empty() && c.len() <= MOST_CORNERS)
        else {
            return self.leaf_fit_by_polygon(mesh, at);
        };
        let mut places = [Vec3::ZERO; MOST_CORNERS];
        let mut bounds = Aabb::EMPTY;
        for (slot, v) in places.iter_mut().zip(corners) {
            let Some(p) = mesh.position(VertexId::from_raw(*v)) else {
                return self.leaf_fit_by_polygon(mesh, at);
            };
            *slot = p;
            bounds.grow(p);
        }
        let mut facing = Vec3::ZERO;
        for [a, b] in sides {
            facing += newell_side(places[*a as usize], places[*b as usize]);
        }
        (bounds, facing)
    }

    /// [`Self::leaf_fit`] polygon by polygon: for a leaf with a polygon of
    /// many corners or a point without a place, and what the listed way
    /// is checked against.
    fn leaf_fit_by_polygon(&self, mesh: &Mesh, at: usize) -> (Aabb, Vec3) {
        let node = self.nodes[at];
        let (mut bounds, mut facing) = (Aabb::EMPTY, Vec3::ZERO);
        let (start, end) = (node.first as usize, (node.first + node.count) as usize);
        for &f in &self.order[start..end] {
            let Some(corners) = mesh.face(FaceId::from_raw(f)) else {
                continue;
            };
            let mut places = [Vec3::ZERO; AT_HAND];
            if at_hand(mesh, corners, &mut places) {
                let places = &places[..corners.len()];
                for p in places {
                    bounds.grow(*p);
                }
                facing += newell_at(places);
            } else {
                for v in corners {
                    if let Some(p) = mesh.position(*v) {
                        bounds.grow(p);
                    }
                }
                facing += newell(mesh, corners);
            }
        }
        (bounds, facing)
    }

    /// Whether it holds no polygon.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// How many polygons it holds (one whose points have no real place is
    /// left out).
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Whether it holds polygon `face`.
    pub fn holds(&self, face: FaceId) -> bool {
        self.leaf_of
            .get(face.get() as usize)
            .is_some_and(|leaf| *leaf != NONE)
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
        self.near_leaves(centre, radius, |leaf| {
            for &f in self.leaf_faces(leaf) {
                found(FaceId::from_raw(f));
            }
        });
    }

    /// Calls `found` with each leaf (a handful of polygons kept together)
    /// whose box comes within `radius` of `centre`, in the tree's own
    /// order: with [`Self::leaf_faces`], what [`Self::near`] gives, a
    /// leaf at a time.
    pub fn near_leaves(&self, centre: Vec3, radius: f32, mut found: impl FnMut(u32)) {
        if self.nodes.is_empty() || !centre.is_finite() || radius.is_nan() || radius < 0.0 {
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
            found(at as u32);
        }
    }

    /// The polygons leaf `leaf` holds (their slots); none for anything
    /// that isn't a leaf.
    pub fn leaf_faces(&self, leaf: u32) -> &[u32] {
        match self.nodes.get(leaf as usize) {
            Some(node) if node.count > 0 => self
                .order
                .get(node.first as usize..(node.first + node.count) as usize)
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    /// The points leaf `leaf` answers for: each point of a polygon is
    /// with one leaf only (the leaf of the first of its polygons the tree
    /// holds), in id order.
    pub fn leaf_points(&self, leaf: u32) -> &[u32] {
        let leaf = leaf as usize;
        match (self.point_starts.get(leaf), self.point_starts.get(leaf + 1)) {
            (Some(start), Some(end)) => self
                .points
                .get(*start as usize..*end as usize)
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    /// The area-weighted facings of leaf `leaf`'s polygons added together
    /// in order (not of unit length), as of its last fit.
    pub fn leaf_facing(&self, leaf: u32) -> Vec3 {
        self.facings.get(leaf as usize).copied().unwrap_or(Vec3::ZERO)
    }

    /// Whether every point of leaf `leaf`'s polygons is surely within
    /// `radius` of `centre`: its whole box is, with a hair to spare.
    pub fn leaf_within(&self, leaf: u32, centre: Vec3, radius: f32) -> bool {
        let reach = radius * WITHIN;
        self.nodes
            .get(leaf as usize)
            .is_some_and(|node| node.bounds.farthest_squared(centre) <= reach * reach)
    }

    /// Bytes it holds.
    pub fn memory_size(&self) -> usize {
        self.nodes.capacity() * std::mem::size_of::<Node>()
            + (self.order.capacity()
                + self.leaf_of.capacity()
                + self.marks.capacity()
                + self.points.capacity()
                + self.point_starts.capacity()
                + self.corners.capacity()
                + self.corner_starts.capacity()
                + self.side_starts.capacity())
                * 4
            + self.sides.capacity() * 2
            + self.facings.capacity() * std::mem::size_of::<Vec3>()
    }

    /// Whether every box and facing is what working it out afresh from
    /// `mesh` gives: to the last bit as they are kept, and (the facings
    /// within rounding) as looking at each polygon gives.
    #[cfg(test)]
    pub(crate) fn is_exact(&self, mesh: &Mesh) -> bool {
        let bits = |v: Vec3| v.to_array().map(f32::to_bits);
        (0..self.nodes.len()).all(|n| {
            let node = self.nodes[n];
            if node.count == 0 {
                let c = node.first as usize;
                return node.bounds == self.nodes[c].bounds.union(self.nodes[c + 1].bounds);
            }
            let (bounds, facing) = self.leaf_fit(mesh, n);
            let (plain_bounds, plain_facing) = self.leaf_fit_by_polygon(mesh, n);
            // Twice the area of its polygons: what rounding is against.
            let area: f32 = self
                .leaf_faces(n as u32)
                .iter()
                .filter_map(|f| mesh.face(FaceId::from_raw(*f)))
                .map(|corners| newell(mesh, corners).length())
                .sum();
            bounds == node.bounds
                && bounds == plain_bounds
                && bits(facing) == bits(self.facings[n])
                && facing.distance(plain_facing) <= area.max(1e-9) * 1e-3
        })
    }
}

/// Lists the points of the polygons `faces` (their slots), once each,
/// onto `corners`, and the sides none of them runs back along onto
/// `sides` (each as where its ends are among those points). Whether it
/// could: not with a polygon of more than [`AT_HAND`] corners.
fn outline(mesh: &Mesh, faces: &[u32], corners: &mut Vec<u32>, sides: &mut Vec<[u8; 2]>) -> bool {
    let (from, sides_from) = (corners.len(), sides.len());
    for &f in faces {
        let Some(list) = mesh.face(FaceId::from_raw(f)) else {
            continue;
        };
        if list.len() > AT_HAND {
            return false;
        }
        let mut local = [0u8; AT_HAND];
        for (slot, v) in local.iter_mut().zip(list) {
            let at = match corners[from..].iter().position(|c| *c == v.get()) {
                Some(at) => at,
                None => {
                    corners.push(v.get());
                    corners.len() - from - 1
                }
            };
            *slot = at as u8;
        }
        for i in 0..list.len() {
            let (a, b) = (local[i], local[(i + 1) % list.len()]);
            // The same side the other way: the two drop out.
            match sides[sides_from..].iter().position(|s| *s == [b, a]) {
                Some(back) => {
                    sides.swap_remove(sides_from + back);
                }
                None => sides.push([a, b]),
            }
        }
    }
    true
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
    // A little slack, so a line through a polygon's edge or corner can't
    // slip between it and its neighbours by rounding.
    const SLACK: f32 = 1e-5;
    let u = s.dot(p) * inv;
    if !(-SLACK..=1.0 + SLACK).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = way.dot(q) * inv;
    if v < -SLACK || u + v > 1.0 + SLACK {
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
    fn a_line_through_a_point_of_the_shape_finds_the_nearest_side() {
        // Regression: a line straight through one of a sphere's points
        // grazes the boxes' edges there, and rounding let it slip past
        // the front to the back of the ball.
        let m = mesh::Primitive::Sphere(mesh::primitives::SphereParams {
            radius: 50.0,
            sides: 160,
            rings: 80,
        })
        .mesh();
        let tree = FaceTree::new(&m);
        let front = Vec3::new(0.0, -50.0, 50.0);
        let way = Vec3::new(-0.576_948_4, 0.781_652_1, -0.236_960_96);
        for along in [219.29, 300.0, 1000.0] {
            let origin = front - way * along;
            let hit = tree.ray_hit(&m, origin, way).expect("a hit");
            assert!(hit.point.distance(front) < 1e-2, "{along}: {:?}", hit.point);
        }
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
    fn refitting_everything_gives_the_boxes_a_fresh_tree_has() {
        let mut m = ball();
        let mut tree = FaceTree::new(&m);
        let moves: Vec<(VertexId, Vec3)> = m
            .vertices()
            .map(|(v, p)| (v, p * (1.5 + p.x.sin() * 0.2)))
            .collect();
        m.set_positions(&moves);
        let faces: Vec<FaceId> = m.faces().map(|(f, _)| f).collect();
        tree.refit_faces(&m, &faces);
        let fresh = FaceTree::new(&m);
        // Box for box what refitting polygon by polygon gives, and the
        // whole shape's box a fresh tree has.
        let mut by_hand = tree.clone();
        by_hand.refit(&m, faces.iter().copied());
        assert_eq!(
            tree.nodes.iter().map(|n| n.bounds).collect::<Vec<_>>(),
            by_hand.nodes.iter().map(|n| n.bounds).collect::<Vec<_>>()
        );
        let root = |t: &FaceTree| t.nodes[0].bounds;
        assert_eq!(root(&tree), root(&fresh));
        let points: Vec<VertexId> = m.vertices().map(|(v, _)| v).collect();
        let mut by_points = FaceTree::new(&ball());
        by_points.refit_points(&m, &points);
        assert_eq!(root(&by_points), root(&fresh));
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
