//! A tree of boxes round a shape's triangles, for finding quickly the
//! nearest place on a surface, or where a line meets it
//! ([decision 0147](../../../docs/decisions/0147-curve-deform-and-shrinkwrap.md)).
//!
//! Each node holds a box round its triangles; a node with more than a few
//! is split in two at the middle of its triangles along its longest side,
//! so a search skips every box farther than the best place found so far.
//! Finding the nearest place on 100 000 triangles takes a few
//! microseconds instead of a pass over them all. Shrinkwrap uses it; the
//! collision hulls and anything else asking "where is the surface?" can
//! too.
//!
//! The tree is built the same way every time from the same triangles, and
//! searched in the same order, so a tie always goes the same way.
//! Triangles with a corner that isn't a real number, or with no area, are
//! left out: they have no surface to find.

use glam::Vec3;

/// The most triangles a leaf holds.
const LEAF: usize = 4;

/// A place found on the surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceHit {
    /// Where it is.
    pub point: Vec3,
    /// Which way the surface faces there (its triangle's, of unit length,
    /// following the order of the triangle's corners).
    pub normal: Vec3,
    /// The triangle's place in the list the tree was built from.
    pub triangle: usize,
    /// How far it is: from the point asked about for
    /// [`TriangleTree::nearest`], along the line (negative behind its
    /// start) for [`TriangleTree::along_line`].
    pub distance: f32,
}

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

    /// The square of the distance from `p` to the box (0 inside).
    fn distance_squared(&self, p: Vec3) -> f32 {
        let d = (self.min - p).max(p - self.max).max(Vec3::ZERO);
        d.length_squared()
    }

    /// The smallest |t| at which the line `origin + t · way` is in the
    /// box, or `None` if it misses it.
    fn line_reach(&self, origin: Vec3, way: Vec3) -> Option<f32> {
        let mut low = f32::NEG_INFINITY;
        let mut high = f32::INFINITY;
        for axis in 0..3 {
            let (o, w) = (origin[axis], way[axis]);
            let (lo, hi) = (self.min[axis], self.max[axis]);
            if w.abs() < 1e-20 {
                if o < lo || o > hi {
                    return None;
                }
                continue;
            }
            let (a, b) = ((lo - o) / w, (hi - o) / w);
            low = low.max(a.min(b));
            high = high.min(a.max(b));
            if low > high {
                return None;
            }
        }
        Some(if low <= 0.0 && high >= 0.0 {
            0.0
        } else {
            low.abs().min(high.abs())
        })
    }
}

/// A node: a box, and either two children or a run of triangles.
#[derive(Debug, Clone)]
struct Node {
    bounds: Aabb,
    /// For a leaf, where its triangles start in `order`; otherwise its
    /// first child (the second follows it).
    first: usize,
    /// How many triangles a leaf holds; 0 for a node with children.
    count: usize,
}

/// A tree of boxes round triangles (see the module notes).
#[derive(Debug, Clone, Default)]
pub struct TriangleTree {
    triangles: Vec<[Vec3; 3]>,
    /// Each kept triangle's place in the list given.
    places: Vec<usize>,
    /// The kept triangles in the order the leaves hold them.
    order: Vec<usize>,
    nodes: Vec<Node>,
}

impl TriangleTree {
    /// The tree round `triangles` (each its three corners). Triangles that
    /// aren't real numbers or have no area are left out.
    pub fn new(triangles: &[[Vec3; 3]]) -> Self {
        let mut kept = Vec::with_capacity(triangles.len());
        let mut places = Vec::with_capacity(triangles.len());
        for (i, t) in triangles.iter().enumerate() {
            let real = t.iter().all(|p| p.is_finite());
            let area = (t[1] - t[0]).cross(t[2] - t[0]).length_squared();
            if real && area > 1e-24 && area.is_finite() {
                kept.push(*t);
                places.push(i);
            }
        }
        let mut tree = Self {
            order: (0..kept.len()).collect(),
            triangles: kept,
            places,
            nodes: Vec::new(),
        };
        if !tree.triangles.is_empty() {
            tree.build();
        }
        tree
    }

    /// The triangles of `mesh`'s polygons (cut as drawing cuts them),
    /// each moved by `matrix`.
    pub fn of_mesh(mesh: &crate::Mesh, matrix: glam::Mat4) -> Self {
        let mut triangles = Vec::with_capacity(mesh.face_count() * 2);
        for (_, corners) in mesh.faces() {
            let points: Vec<Vec3> = corners
                .iter()
                .map(|v| {
                    matrix.transform_point3(mesh.position(*v).unwrap_or(Vec3::splat(f32::NAN)))
                })
                .collect();
            crate::face_triangles(mesh, corners, |[a, b, c]| {
                triangles.push([points[a], points[b], points[c]]);
            });
        }
        Self::new(&triangles)
    }

    /// Whether it holds no triangle.
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// How many triangles it holds (those with a surface).
    pub fn len(&self) -> usize {
        self.triangles.len()
    }

    fn centre(&self, i: usize) -> Vec3 {
        let t = &self.triangles[i];
        (t[0] + t[1] + t[2]) / 3.0
    }

    fn bounds_of(&self, run: &[usize]) -> Aabb {
        let mut b = Aabb::EMPTY;
        for &i in run {
            for p in self.triangles[i] {
                b.grow(p);
            }
        }
        b
    }

    /// Builds the nodes, splitting runs of triangles until each leaf holds
    /// at most [`LEAF`]. Without recursion: a list of runs still to split.
    fn build(&mut self) {
        let mut order = std::mem::take(&mut self.order);
        self.nodes.push(Node {
            bounds: self.bounds_of(&order),
            first: 0,
            count: order.len(),
        });
        let mut waiting = vec![0usize];
        while let Some(at) = waiting.pop() {
            let (start, count) = (self.nodes[at].first, self.nodes[at].count);
            if count <= LEAF {
                continue;
            }
            let run = &mut order[start..start + count];
            let mut centres = Aabb::EMPTY;
            for &i in run.iter() {
                centres.grow(self.centre(i));
            }
            let size = centres.max - centres.min;
            let axis = if size.x >= size.y && size.x >= size.z {
                0
            } else if size.y >= size.z {
                1
            } else {
                2
            };
            let half = count / 2;
            run.select_nth_unstable_by(half, |a, b| {
                let (ca, cb) = (self.centre(*a)[axis], self.centre(*b)[axis]);
                ca.total_cmp(&cb).then(a.cmp(b))
            });
            let left = self.nodes.len();
            let (low, high) = run.split_at(half);
            let (low_bounds, high_bounds) = (self.bounds_of(low), self.bounds_of(high));
            self.nodes.push(Node {
                bounds: low_bounds,
                first: start,
                count: half,
            });
            self.nodes.push(Node {
                bounds: high_bounds,
                first: start + half,
                count: count - half,
            });
            self.nodes[at].first = left;
            self.nodes[at].count = 0;
            waiting.push(left);
            waiting.push(left + 1);
        }
        self.order = order;
    }

    fn hit(&self, i: usize, point: Vec3, distance: f32) -> SurfaceHit {
        let t = &self.triangles[i];
        SurfaceHit {
            point,
            normal: (t[1] - t[0]).cross(t[2] - t[0]).normalize_or(Vec3::Z),
            triangle: self.places[i],
            distance,
        }
    }

    /// The place on the surface nearest `p`, or `None` for an empty tree
    /// or a `p` that isn't a real number.
    pub fn nearest(&self, p: Vec3) -> Option<SurfaceHit> {
        if self.nodes.is_empty() || !p.is_finite() {
            return None;
        }
        let mut best: Option<(f32, usize, Vec3)> = None;
        let mut waiting = vec![0usize];
        while let Some(at) = waiting.pop() {
            let node = &self.nodes[at];
            let reach = node.bounds.distance_squared(p);
            if best.is_some_and(|(d, _, _)| reach >= d) {
                continue;
            }
            if node.count == 0 {
                // The nearer child last, so it is looked at first.
                let (a, b) = (node.first, node.first + 1);
                let (da, db) = (
                    self.nodes[a].bounds.distance_squared(p),
                    self.nodes[b].bounds.distance_squared(p),
                );
                if da <= db {
                    waiting.extend([b, a]);
                } else {
                    waiting.extend([a, b]);
                }
                continue;
            }
            for &i in &self.order[node.first..node.first + node.count] {
                let [a, b, c] = self.triangles[i];
                let q = closest_on_triangle(p, a, b, c);
                let d = q.distance_squared(p);
                if best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, i, q));
                }
            }
        }
        best.map(|(d, i, q)| self.hit(i, q, d.sqrt()))
    }

    /// Where the line through `origin` along `way` (either direction)
    /// first meets the surface: the meeting nearest `origin`, with its
    /// distance along `way` (negative behind `origin`). `None` when it
    /// meets nothing, or `way` has no length.
    pub fn along_line(&self, origin: Vec3, way: Vec3) -> Option<SurfaceHit> {
        let way = way.try_normalize()?;
        if self.nodes.is_empty() || !origin.is_finite() {
            return None;
        }
        let mut best: Option<(f32, usize)> = None;
        let mut waiting = vec![0usize];
        while let Some(at) = waiting.pop() {
            let node = &self.nodes[at];
            let Some(reach) = node.bounds.line_reach(origin, way) else {
                continue;
            };
            if best.is_some_and(|(t, _)| reach >= t.abs()) {
                continue;
            }
            if node.count == 0 {
                waiting.extend([node.first + 1, node.first]);
                continue;
            }
            for &i in &self.order[node.first..node.first + node.count] {
                if let Some(t) = line_meets_triangle(origin, way, &self.triangles[i])
                    && best.is_none_or(|(bt, _)| t.abs() < bt.abs())
                {
                    best = Some((t, i));
                }
            }
        }
        best.map(|(t, i)| self.hit(i, origin + way * t, t))
    }
}

/// Where the line `origin + t · way` meets triangle `tri`, as `t` (either
/// sign), or `None` if it misses it or runs along it (Möller and
/// Trumbore, 1997).
fn line_meets_triangle(origin: Vec3, way: Vec3, tri: &[Vec3; 3]) -> Option<f32> {
    let e1 = tri[1] - tri[0];
    let e2 = tri[2] - tri[0];
    let p = way.cross(e2);
    let det = e1.dot(p);
    // Scaled by the triangle's size, so tiny and huge shapes are alike.
    if det.abs() <= 1e-12 * e1.length() * e2.length() {
        return None;
    }
    let inv = 1.0 / det;
    let s = origin - tri[0];
    let u = s.dot(p) * inv;
    const EDGE: f32 = 1e-6;
    if !(-EDGE..=1.0 + EDGE).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = way.dot(q) * inv;
    if v < -EDGE || u + v > 1.0 + EDGE {
        return None;
    }
    let t = e2.dot(q) * inv;
    t.is_finite().then_some(t)
}

/// The point of triangle `abc` nearest `p` (Ericson, *Real-Time Collision
/// Detection*, 5.1.5).
pub(crate) fn closest_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let sum = va + vb + vc;
    if sum.abs() <= f32::MIN_POSITIVE {
        return a;
    }
    a + ab * (vb / sum) + ac * (vc / sum)
}

#[cfg(test)]
mod tests;
