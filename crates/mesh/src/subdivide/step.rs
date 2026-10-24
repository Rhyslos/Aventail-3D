//! One step of Catmull-Clark subdivision on one channel: the points'
//! positions, or the UVs (each UV corner welded to its neighbours where
//! they match). Both channels share the polygons' layout, so the new
//! polygons line up corner for corner.

use std::ops::{Add, Mul, Sub};

/// A sharpness that never wears off: a full crease, a hard edge kept
/// sharp, or an open border.
pub(crate) const ALWAYS: f32 = f32::INFINITY;

/// What a channel's values must do: add, scale and blend.
pub(crate) trait Blend:
    Copy + Default + Add<Output = Self> + Sub<Output = Self> + Mul<f32, Output = Self>
{
}

impl<T> Blend for T where
    T: Copy + Default + Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T>
{
}

/// The polygons, as one list of corners (indices into a channel's values)
/// and where each polygon starts in it (one more start than polygons).
#[derive(Debug, Clone, Default)]
pub(crate) struct Layout {
    pub(crate) starts: Vec<u32>,
    pub(crate) corners: Vec<u32>,
}

impl Layout {
    pub(crate) fn face_count(&self) -> usize {
        self.starts.len().saturating_sub(1)
    }

    /// The corners of polygon `f`, as a range in the corner list.
    pub(crate) fn range(&self, f: usize) -> std::ops::Range<usize> {
        self.starts[f] as usize..self.starts[f + 1] as usize
    }

    /// The corner after `c` in its polygon `f`.
    fn next(&self, f: usize, c: usize) -> usize {
        let r = self.range(f);
        if c + 1 == r.end { r.start } else { c + 1 }
    }

    /// The corner before `c` in its polygon `f`.
    fn prev(&self, f: usize, c: usize) -> usize {
        let r = self.range(f);
        if c == r.start { r.end - 1 } else { c - 1 }
    }

    /// The layout after one step: each polygon of `n` corners becomes `n`
    /// quads, one per corner: the corner's point, the point on the edge
    /// after it, the polygon's middle, the point on the edge before it.
    /// `edge_of` gives each corner's edge (to the next corner); points are
    /// numbered the old points first, then one per edge, then one per
    /// polygon.
    fn refined(&self, edge_of: &[u32], points: usize, edges: usize) -> Self {
        let n = self.corners.len();
        let mut starts = Vec::with_capacity(n + 1);
        let mut corners = Vec::with_capacity(n * 4);
        let edge_base = points as u32;
        let face_base = (points + edges) as u32;
        for f in 0..self.face_count() {
            for c in self.range(f) {
                starts.push(corners.len() as u32);
                corners.push(self.corners[c]);
                corners.push(edge_base + edge_of[c]);
                corners.push(face_base + f as u32);
                corners.push(edge_base + edge_of[self.prev(f, c)]);
            }
        }
        starts.push(corners.len() as u32);
        Self { starts, corners }
    }
}

/// A channel's edges: each corner's edge to the next corner, and for each
/// edge its two ends, how many polygons use it, two of them, and its
/// sharpness (the most any of its corners gives it).
struct Edges {
    of_corner: Vec<u32>,
    ends: Vec<[u32; 2]>,
    uses: Vec<u32>,
    faces: Vec<[u32; 2]>,
    sharpness: Vec<f32>,
}

impl Edges {
    fn of(layout: &Layout, sharp: &[f32], points: usize) -> Self {
        let n = layout.corners.len();
        // (low end, high end, corner, polygon), grouped by the low end
        // (a counting sort, as every low end is a point number) and then
        // sorted within each point's few, so each edge's corners sit
        // together: much quicker than a hash map or a full sort.
        let mut first = vec![0u32; points + 1];
        for f in 0..layout.face_count() {
            for c in layout.range(f) {
                let a = layout.corners[c];
                let b = layout.corners[layout.next(f, c)];
                first[a.min(b) as usize + 1] += 1;
            }
        }
        for i in 1..first.len() {
            first[i] += first[i - 1];
        }
        let mut fill = first.clone();
        let mut keyed: Vec<(u32, u32, u32, u32)> = vec![(0, 0, 0, 0); n];
        for f in 0..layout.face_count() {
            for c in layout.range(f) {
                let a = layout.corners[c];
                let b = layout.corners[layout.next(f, c)];
                let low = a.min(b) as usize;
                keyed[fill[low] as usize] = (a.min(b), a.max(b), c as u32, f as u32);
                fill[low] += 1;
            }
        }
        for p in 0..points {
            keyed[first[p] as usize..first[p + 1] as usize].sort_unstable();
        }
        let mut of_corner = vec![0u32; n];
        let mut ends = Vec::with_capacity(n / 2 + 1);
        let mut uses = Vec::with_capacity(n / 2 + 1);
        let mut faces = Vec::with_capacity(n / 2 + 1);
        let mut sharpness = Vec::with_capacity(n / 2 + 1);
        let mut i = 0;
        while i < keyed.len() {
            let (a, b, _, _) = keyed[i];
            let id = ends.len() as u32;
            let mut j = i;
            let mut s = 0.0f32;
            let mut two = [keyed[i].3, keyed[i].3];
            while j < keyed.len() && keyed[j].0 == a && keyed[j].1 == b {
                let (_, _, c, f) = keyed[j];
                of_corner[c as usize] = id;
                s = s.max(sharp.get(c as usize).copied().unwrap_or(0.0));
                if j == i + 1 {
                    two[1] = f;
                }
                j += 1;
            }
            ends.push([a, b]);
            uses.push((j - i) as u32);
            faces.push(two);
            sharpness.push(s);
            i = j;
        }
        Self {
            of_corner,
            ends,
            uses,
            faces,
            sharpness,
        }
    }

    /// How sharp edge `e` is: always for an open border or an edge shared
    /// by three or more polygons.
    fn sharpness(&self, e: usize) -> f32 {
        if self.uses[e] == 2 {
            self.sharpness[e]
        } else {
            ALWAYS
        }
    }
}

/// What a point knows of its neighbourhood, gathered edge by edge and
/// polygon by polygon.
#[derive(Clone, Copy, Default)]
struct Around<T> {
    edges: u32,
    edge_mids: T,
    faces: u32,
    face_points: T,
    /// Sharp edges: how many, their other ends (the first two) and their
    /// sharpness added up (finite ones; any that never wears off makes
    /// `always` true).
    sharp: u32,
    sharp_ends: [T; 2],
    sharp_sum: f32,
    always: bool,
}

/// The result of one step on one channel.
pub(crate) struct Stepped<T> {
    pub(crate) values: Vec<T>,
    pub(crate) layout: Layout,
    /// Each new corner's edge sharpness (to the next corner).
    pub(crate) sharp: Vec<f32>,
    /// For each new corner, the old corner whose edge its edge is half of
    /// (`None` for the new edges inside a polygon).
    pub(crate) edge_from: Vec<Option<u32>>,
}

/// One Catmull-Clark step on `values` laid out by `layout`, with each
/// corner's edge sharpness in `sharp` (0 smooth; from 1, or [`ALWAYS`],
/// sharp; in between a blend). `keep_corners`: a point on an open border
/// with only one polygon (the corner of a flat sheet) stays where it is.
pub(crate) fn step<T: Blend>(
    values: &[T],
    layout: &Layout,
    sharp: &[f32],
    keep_corners: bool,
) -> Stepped<T> {
    let edges = Edges::of(layout, sharp, values.len());
    let nf = layout.face_count();
    let ne = edges.ends.len();
    let nv = values.len();

    // The middle of each polygon.
    let face_points: Vec<T> = (0..nf)
        .map(|f| {
            let r = layout.range(f);
            let n = r.len().max(1) as f32;
            let mut sum = T::default();
            for c in r {
                sum = sum + values[layout.corners[c] as usize];
            }
            sum * (1.0 / n)
        })
        .collect();

    // A point on each edge: its middle where it is sharp, the middle of
    // its ends and its two polygons' middles where it is smooth.
    let edge_points: Vec<T> = (0..ne)
        .map(|e| {
            let [a, b] = edges.ends[e];
            let mid = (values[a as usize] + values[b as usize]) * 0.5;
            let s = edges.sharpness(e);
            if s >= 1.0 {
                return mid;
            }
            let [f0, f1] = edges.faces[e];
            let smooth = (values[a as usize]
                + values[b as usize]
                + face_points[f0 as usize]
                + face_points[f1 as usize])
                * 0.25;
            if s > 0.0 {
                smooth + (mid - smooth) * s
            } else {
                smooth
            }
        })
        .collect();

    // Each old point's neighbourhood.
    let mut around: Vec<Around<T>> = vec![Around::default(); nv];
    for e in 0..ne {
        let [a, b] = edges.ends[e];
        let mid = (values[a as usize] + values[b as usize]) * 0.5;
        let s = edges.sharpness(e);
        for (p, other) in [(a, b), (b, a)] {
            let at = &mut around[p as usize];
            at.edges += 1;
            at.edge_mids = at.edge_mids + mid;
            if s > 0.0 {
                if (at.sharp as usize) < 2 {
                    at.sharp_ends[at.sharp as usize] = values[other as usize];
                }
                at.sharp += 1;
                if s.is_finite() {
                    at.sharp_sum += s;
                } else {
                    at.always = true;
                }
            }
        }
    }
    for (f, middle) in face_points.iter().enumerate() {
        for c in layout.range(f) {
            let at = &mut around[layout.corners[c] as usize];
            at.faces += 1;
            at.face_points = at.face_points + *middle;
        }
    }

    // Each old point moved: the smooth rule, a crease (two sharp edges),
    // or a corner (more), blended for edges only partly sharp.
    let point_values: Vec<T> = (0..nv)
        .map(|p| {
            let v = values[p];
            let at = &around[p];
            if at.faces == 0 || at.edges == 0 {
                return v;
            }
            let smooth = || {
                let n = at.edges as f32;
                let q = at.face_points * (1.0 / at.faces as f32);
                let r = at.edge_mids * (1.0 / n);
                (q + r * 2.0 + v * (n - 3.0)) * (1.0 / n)
            };
            if at.sharp < 2 {
                return smooth();
            }
            let sharp = if at.sharp == 2 {
                if keep_corners && at.faces == 1 && at.always {
                    v
                } else {
                    (v * 6.0 + at.sharp_ends[0] + at.sharp_ends[1]) * 0.125
                }
            } else {
                v
            };
            let strength = if at.always {
                1.0
            } else {
                at.sharp_sum / at.sharp as f32
            };
            if strength >= 1.0 {
                sharp
            } else {
                let s = smooth();
                s + (sharp - s) * strength
            }
        })
        .collect();

    let mut new_values = point_values;
    new_values.reserve(ne + nf);
    new_values.extend(edge_points);
    new_values.extend(face_points);

    let new_layout = layout.refined(&edges.of_corner, nv, ne);
    let wear = |s: f32| if s.is_finite() { (s - 1.0).max(0.0) } else { s };
    let mut new_sharp = Vec::with_capacity(new_layout.corners.len());
    let mut edge_from = Vec::with_capacity(new_layout.corners.len());
    for f in 0..nf {
        for c in layout.range(f) {
            let before = layout.prev(f, c);
            let s = |c: usize| wear(sharp.get(c).copied().unwrap_or(0.0));
            new_sharp.extend([s(c), 0.0, 0.0, s(before)]);
            edge_from.extend([Some(c as u32), None, None, Some(before as u32)]);
        }
    }
    Stepped {
        values: new_values,
        layout: new_layout,
        sharp: new_sharp,
        edge_from,
    }
}
