//! Convex hulls ([decision 0151](../../../docs/decisions/0151-collision-hulls.md)):
//! the tightest shape with no dents that holds a set of points, as if the
//! points were wrapped in cling film.
//!
//! Game engines collide with convex shapes cheaply, so a hull is what an
//! object's *Wrapped* collision is, and each part of *Wrapped in parts* is
//! one. Built by **quickhull**: a first four-cornered solid from points
//! far apart, then, again and again, the point farthest outside one of its
//! triangles is added, every triangle it can see is taken away, and the
//! hole is closed with new triangles to it. Points never outside any
//! triangle are inside and forgotten.
//!
//! Worked in double precision, with a small distance (relative to the
//! points' extent, or to their rounding when they are far from the
//! origin) under which a point counts as on a triangle, so flat sides stay
//! flat and shallow edges stay edges wherever the shape is. Always the same result for the same points, in the
//! same order. A limited hull ([`convex_hull_limited`]) stops once it has
//! a number of corners: as each step adds the farthest point, it is the
//! best few-cornered hull of that kind, a little inside the full one.

use glam::{DVec3, Vec3};
use std::collections::HashMap;
use words::tr;

/// A convex hull: its corners, and its triangles, each running
/// counter-clockwise seen from outside.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Hull {
    /// Its corners.
    pub points: Vec<Vec3>,
    /// Its triangles, as places in `points`, facing out.
    pub triangles: Vec<[u32; 3]>,
}

/// Why points have no hull.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HullProblem {
    /// There are no points at real places.
    Empty,
    /// The points all lie on one plane (or line, or spot): a hull would
    /// have no inside.
    Flat,
}

impl std::fmt::Display for HullProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Empty => tr("the shape has no points"),
            Self::Flat => tr("the shape is flat, so a wrapping round it would have no inside"),
        })
    }
}

impl std::error::Error for HullProblem {}

impl Hull {
    /// How much it holds (cubic units of its points).
    pub fn volume(&self) -> f64 {
        self.triangles
            .iter()
            .filter_map(|t| self.corners(*t))
            .map(|[a, b, c]| a.dot(b.cross(c)) / 6.0)
            .sum()
    }

    /// Whether `p` is inside it, or outside by no more than `tolerance`.
    pub fn contains(&self, p: Vec3, tolerance: f32) -> bool {
        let p = p.as_dvec3();
        self.triangles
            .iter()
            .filter_map(|t| self.corners(*t))
            .all(|[a, b, c]| {
                let n = (b - a).cross(c - a).normalize_or_zero();
                n.dot(p - a) <= f64::from(tolerance)
            })
    }

    /// The lines to draw it with: each edge between two triangles that
    /// don't lie flat together, so a box shows its twelve edges, not the
    /// diagonals across its sides. Lower place first, in order.
    pub fn outline(&self) -> Vec<[u32; 2]> {
        let mut sides: HashMap<(u32, u32), Vec<DVec3>> = HashMap::new();
        for t in &self.triangles {
            let Some([a, b, c]) = self.corners(*t) else {
                continue;
            };
            let n = (b - a).cross(c - a).normalize_or_zero();
            for k in 0..3 {
                let (p, q) = (t[k], t[(k + 1) % 3]);
                sides.entry((p.min(q), p.max(q))).or_default().push(n);
            }
        }
        let mut out: Vec<[u32; 2]> = sides
            .into_iter()
            .filter(|(_, normals)| match normals.as_slice() {
                [a, b] => a.dot(*b) < 1.0 - 1e-5,
                _ => true,
            })
            .map(|((p, q), _)| [p, q])
            .collect();
        out.sort_unstable();
        out
    }

    /// Its corners where at least three sides of different directions
    /// meet: the points of a convex solid that are really corners, not
    /// on a flat side or a straight edge. Two triangles lie on the same
    /// side when each one's corners are within [`same_side_tolerance`] of
    /// the other's plane.
    fn true_corners(&self) -> Vec<Vec3> {
        let points: Vec<DVec3> = self.points.iter().map(|p| p.as_dvec3()).collect();
        let tolerance = same_side_tolerance(&points);
        // Each triangle's corners and plane.
        let sides: Vec<([DVec3; 3], DVec3)> = self
            .triangles
            .iter()
            .filter_map(|t| self.corners(*t))
            .map(|[a, b, c]| ([a, b, c], (b - a).cross(c - a).normalize_or_zero()))
            .collect();
        let same = |i: usize, j: usize| {
            let ((pi, ni), (pj, nj)) = (sides[i], sides[j]);
            ni.dot(nj) > 0.0
                && pj.iter().all(|q| ni.dot(*q - pi[0]).abs() <= tolerance)
                && pi.iter().all(|q| nj.dot(*q - pj[0]).abs() <= tolerance)
        };
        let mut planes: Vec<Vec<usize>> = vec![Vec::new(); self.points.len()];
        for (index, t) in self.triangles.iter().enumerate().take(sides.len()) {
            for i in t {
                let list = &mut planes[*i as usize];
                if !list.iter().any(|j| same(*j, index)) {
                    list.push(index);
                }
            }
        }
        self.points
            .iter()
            .zip(&planes)
            .filter(|(_, p)| p.len() >= 3)
            .map(|(p, _)| *p)
            .collect()
    }

    fn corners(&self, t: [u32; 3]) -> Option<[DVec3; 3]> {
        let at = |i: u32| self.points.get(i as usize).map(|p| p.as_dvec3());
        Some([at(t[0])?, at(t[1])?, at(t[2])?])
    }
}

/// How close to a plane a point counts as on it, for `points`: a
/// millionth of their extent (their size, wherever they are), or, if more,
/// a few times the rounding of their places as single-precision numbers,
/// which grows with their distance from the origin (a flat side 100 m
/// away has its points off its plane by about a hundredth of a
/// millimetre).
fn same_side_tolerance(points: &[DVec3]) -> f64 {
    let (low, high) = points.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(l, h), p| (l.min(*p), h.max(*p)),
    );
    if points.is_empty() {
        return 0.0;
    }
    let extent = (high - low).max_element();
    let farthest = high.abs().max(low.abs()).max_element();
    (extent * 1e-6).max(farthest * 4.0 * f64::from(f32::EPSILON))
}

/// The convex hull of `points` (those at real places), or why there is
/// none.
pub fn convex_hull(points: &[Vec3]) -> Result<Hull, HullProblem> {
    convex_hull_limited(points, usize::MAX)
}

/// The convex hull of `points`, stopped once it has `max_corners` corners
/// (at least 4): the farthest points first, so a sphere's limited hull is
/// an even, rounded solid a little inside it.
pub fn convex_hull_limited(points: &[Vec3], max_corners: usize) -> Result<Hull, HullProblem> {
    // Each place once (a shape's points are often listed once per
    // polygon), in an order of their own.
    let mut unique: Vec<[u32; 3]> = points
        .iter()
        .filter(|p| p.is_finite())
        .map(|p| (*p + Vec3::ZERO).to_array().map(f32::to_bits))
        .collect();
    unique.sort_unstable();
    unique.dedup();
    let points: Vec<DVec3> = unique
        .iter()
        .map(|bits| Vec3::from_array(bits.map(f32::from_bits)).as_dvec3())
        .collect();
    if points.is_empty() {
        return Err(HullProblem::Empty);
    }
    let mut builder = Builder::start(&points)?;
    builder.grow(max_corners.max(4));
    let mut hull = builder.finish();
    // A point in the middle of a flat side, or along a straight edge, can
    // become a corner while the hull grows (it was outside an earlier,
    // rougher hull). Such points are dropped and the hull built again from
    // the true corners, which are then all it has.
    for _ in 0..4 {
        let corners = hull.true_corners();
        if corners.len() == hull.points.len() {
            break;
        }
        let points: Vec<DVec3> = corners.iter().map(|p| p.as_dvec3()).collect();
        let mut builder = Builder::start(&points)?;
        builder.grow(usize::MAX);
        hull = builder.finish();
    }
    Ok(hull)
}

/// One triangle of a hull being built.
struct Face {
    corners: [usize; 3],
    normal: DVec3,
    offset: f64,
    /// The points outside it (each point belongs to one face at most).
    outside: Vec<usize>,
    alive: bool,
}

/// A hull being built.
struct Builder<'a> {
    points: &'a [DVec3],
    /// How far outside a triangle a point must be to count.
    epsilon: f64,
    faces: Vec<Face>,
    /// Each directed edge (from, to) and the face it runs round.
    edges: HashMap<(usize, usize), usize>,
    /// How many live faces use each point.
    uses: Vec<u32>,
    corners: usize,
}

impl<'a> Builder<'a> {
    /// The first four-cornered solid, with every other point given to a
    /// face it is outside of.
    fn start(points: &'a [DVec3]) -> Result<Self, HullProblem> {
        // Flat below the same tolerance the hull's sides are found with:
        // its extent's, or its points' rounding far from the origin. A
        // point counts as outside a triangle a thousand times closer.
        let flat = same_side_tolerance(points).max(f64::MIN_POSITIVE);
        let epsilon = (flat * 1e-3).max(f64::MIN_POSITIVE);
        // The two farthest apart of the points lowest and highest along
        // each axis.
        let mut extremes = Vec::with_capacity(6);
        for axis in 0..3 {
            let pick = |better: fn(f64, f64) -> bool| {
                (0..points.len()).fold(0, |best, i| {
                    if better(points[i][axis], points[best][axis]) {
                        i
                    } else {
                        best
                    }
                })
            };
            extremes.push(pick(|a, b| a < b));
            extremes.push(pick(|a, b| a > b));
        }
        let mut pair = (extremes[0], extremes[1]);
        let mut far = 0.0;
        for &a in &extremes {
            for &b in &extremes {
                let d = points[a].distance_squared(points[b]);
                if d > far {
                    far = d;
                    pair = (a, b);
                }
            }
        }
        if far.sqrt() <= flat {
            return Err(HullProblem::Flat);
        }
        let (a, b) = pair;
        let line = (points[b] - points[a]).normalize();
        let off_line = |p: DVec3| {
            let d = p - points[a];
            (d - line * d.dot(line)).length()
        };
        let c = farthest(points, off_line);
        if off_line(points[c]) <= flat {
            return Err(HullProblem::Flat);
        }
        let normal = (points[b] - points[a])
            .cross(points[c] - points[a])
            .normalize();
        let off_plane = |p: DVec3| normal.dot(p - points[a]).abs();
        let d = farthest(points, off_plane);
        if off_plane(points[d]) <= flat {
            return Err(HullProblem::Flat);
        }
        let mut builder = Self {
            points,
            epsilon,
            faces: Vec::new(),
            edges: HashMap::new(),
            uses: vec![0; points.len()],
            corners: 0,
        };
        let middle = (points[a] + points[b] + points[c] + points[d]) / 4.0;
        for [p, q, r] in [[a, b, c], [a, d, b], [b, d, c], [c, d, a]] {
            let (n, _) = builder.plane([p, q, r]);
            let corners = if n.dot(middle - points[p]) > 0.0 {
                [p, r, q]
            } else {
                [p, q, r]
            };
            builder.add_face(corners);
        }
        let first: Vec<usize> = (0..points.len())
            .filter(|i| ![a, b, c, d].contains(i))
            .collect();
        builder.share_out(&first, &[0, 1, 2, 3]);
        Ok(builder)
    }

    /// The unit normal and offset of the plane through `corners`.
    fn plane(&self, [a, b, c]: [usize; 3]) -> (DVec3, f64) {
        let (pa, pb, pc) = (self.points[a], self.points[b], self.points[c]);
        let n = (pb - pa).cross(pc - pa).normalize_or_zero();
        (n, n.dot(pa))
    }

    fn distance(&self, face: usize, p: usize) -> f64 {
        let f = &self.faces[face];
        f.normal.dot(self.points[p]) - f.offset
    }

    fn add_face(&mut self, corners: [usize; 3]) -> usize {
        let (normal, offset) = self.plane(corners);
        let index = self.faces.len();
        for k in 0..3 {
            self.edges.insert((corners[k], corners[(k + 1) % 3]), index);
            if self.uses[corners[k]] == 0 {
                self.corners += 1;
            }
            self.uses[corners[k]] += 1;
        }
        self.faces.push(Face {
            corners,
            normal,
            offset,
            outside: Vec::new(),
            alive: true,
        });
        index
    }

    fn remove_face(&mut self, face: usize) {
        let corners = self.faces[face].corners;
        self.faces[face].alive = false;
        for k in 0..3 {
            let edge = (corners[k], corners[(k + 1) % 3]);
            if self.edges.get(&edge) == Some(&face) {
                self.edges.remove(&edge);
            }
            self.uses[corners[k]] = self.uses[corners[k]].saturating_sub(1);
            if self.uses[corners[k]] == 0 {
                self.corners = self.corners.saturating_sub(1);
            }
        }
    }

    /// Gives each of `points` to the first of `faces` it is outside of;
    /// points outside none are inside the hull and dropped.
    fn share_out(&mut self, points: &[usize], faces: &[usize]) {
        for &p in points {
            if let Some(&f) = faces.iter().find(|f| self.distance(**f, p) > self.epsilon) {
                self.faces[f].outside.push(p);
            }
        }
    }

    /// Adds the farthest outside points until none is left or the hull
    /// has `max_corners` corners.
    fn grow(&mut self, max_corners: usize) {
        let mut next = 0;
        while self.corners < max_corners {
            // The first live face with points outside it.
            while next < self.faces.len()
                && (!self.faces[next].alive || self.faces[next].outside.is_empty())
            {
                next += 1;
            }
            if next == self.faces.len() {
                break;
            }
            let face = next;
            let eye = self.faces[face]
                .outside
                .iter()
                .copied()
                .fold(None, |best: Option<(usize, f64)>, p| {
                    let d = self.distance(face, p);
                    match best {
                        Some((_, bd)) if bd >= d => best,
                        _ => Some((p, d)),
                    }
                })
                .map(|(p, _)| p);
            let Some(eye) = eye else {
                break;
            };
            if !self.add_point(face, eye) {
                // Rounding made the step impossible: the point is left
                // out (it lies within rounding of the hull).
                self.faces[face].outside.retain(|p| *p != eye);
            }
            // New faces come after, so faces before `face` stay done;
            // start again from it.
            next = face;
        }
    }

    /// Adds `eye` (outside `face`): takes away every face it sees and
    /// closes the hole with new faces to it. False, changing nothing, when
    /// the faces it sees don't make one hole with a single rim.
    fn add_point(&mut self, face: usize, eye: usize) -> bool {
        let mut seen = vec![face];
        let mut visible = std::collections::HashSet::from([face]);
        let mut looked = std::collections::HashSet::from([face]);
        let mut i = 0;
        while i < seen.len() {
            let corners = self.faces[seen[i]].corners;
            for k in 0..3 {
                let (p, q) = (corners[k], corners[(k + 1) % 3]);
                let Some(&other) = self.edges.get(&(q, p)) else {
                    return false;
                };
                if looked.insert(other) && self.distance(other, eye) > self.epsilon {
                    visible.insert(other);
                    seen.push(other);
                }
            }
            i += 1;
        }
        // The rim: edges of seen faces whose neighbour isn't seen.
        let mut rim: Vec<(usize, usize)> = Vec::new();
        for &f in &seen {
            let corners = self.faces[f].corners;
            for k in 0..3 {
                let (p, q) = (corners[k], corners[(k + 1) % 3]);
                let other = self.edges.get(&(q, p)).copied();
                if other.is_none_or(|o| !visible.contains(&o)) {
                    rim.push((p, q));
                }
            }
        }
        // A single loop: every rim point starts one edge and ends one,
        // and the eye isn't on it.
        let mut starts: HashMap<usize, u32> = HashMap::new();
        let mut ends: HashMap<usize, u32> = HashMap::new();
        for &(p, q) in &rim {
            *starts.entry(p).or_default() += 1;
            *ends.entry(q).or_default() += 1;
        }
        if rim.len() < 3
            || starts.contains_key(&eye)
            || starts.values().any(|n| *n != 1)
            || ends.values().any(|n| *n != 1)
            || starts.len() != ends.len()
        {
            return false;
        }
        let mut orphans = Vec::new();
        for &f in &seen {
            orphans.append(&mut self.faces[f].outside);
            self.remove_face(f);
        }
        orphans.retain(|p| *p != eye);
        let new: Vec<usize> = rim
            .iter()
            .map(|&(p, q)| self.add_face([p, q, eye]))
            .collect();
        self.share_out(&orphans, &new);
        true
    }

    /// The hull: its live faces, their corners numbered in the order they
    /// first appear.
    fn finish(self) -> Hull {
        let mut number: HashMap<usize, u32> = HashMap::new();
        let mut hull = Hull::default();
        for face in self.faces.iter().filter(|f| f.alive) {
            let mut t = [0u32; 3];
            for (slot, &c) in t.iter_mut().zip(&face.corners) {
                let next = hull.points.len() as u32;
                *slot = *number.entry(c).or_insert_with(|| {
                    hull.points.push(self.points[c].as_vec3());
                    next
                });
            }
            hull.triangles.push(t);
        }
        hull
    }
}

/// The point with the largest `measure` (the first of equals).
fn farthest(points: &[DVec3], measure: impl Fn(DVec3) -> f64) -> usize {
    let mut best = (0, f64::MIN);
    for (i, p) in points.iter().enumerate() {
        let m = measure(*p);
        if m > best.1 {
            best = (i, m);
        }
    }
    best.0
}

#[cfg(test)]
mod tests;
