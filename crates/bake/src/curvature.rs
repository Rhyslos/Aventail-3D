//! Curvature from the shape's edges: where two polygons meet at an
//! angle, a band along the edge is light if the edge sticks out
//! (convex) and dark if it goes in (concave), fading over the edge
//! width. Flat and gently curved surfaces stay mid-grey.

use glam::Vec3;
use paint::PaintSurface;
use std::collections::HashMap;

/// Folds below this (radians) are left out: the slight bends of curved
/// surfaces.
const LEAST_FOLD: f32 = 0.15;

/// Edges the shape is shaded smoothly across are left out below this
/// fold (radians, 60°): they are the facets of a round shape (a
/// cylinder's sides, a sphere), not edges a painter would highlight.
const LEAST_SOFT_FOLD: f32 = std::f32::consts::FRAC_PI_3;

/// How close two shading normals are to count as the same (shading
/// flows across the edge).
const SAME_SHADING: f32 = 0.999;

/// A folded edge: its ends, and how strongly it is folded (out positive,
/// in negative, 1 at a right angle).
#[derive(Debug, Clone, Copy)]
struct Fold {
    a: Vec3,
    b: Vec3,
    strength: f32,
}

/// The shape's folded edges, with a grid to find those near a point.
#[derive(Debug, Clone)]
pub(crate) struct Edges {
    folds: Vec<Fold>,
    width: f32,
    min: Vec3,
    cell: f32,
    grid: HashMap<(i32, i32, i32), Vec<u32>>,
}

impl Edges {
    /// The folded edges of `surface`'s triangles in `slots`, with bands
    /// `width` wide.
    pub(crate) fn of(surface: &PaintSurface, slots: &[u32], width: f32) -> Self {
        let (triangles, shading): (Vec<_>, Vec<_>) = surface
            .triangles()
            .iter()
            .enumerate()
            .filter(|(_, t)| slots.contains(&t.slot) && t.normal != Vec3::ZERO)
            .map(|(i, t)| (t, surface.shading(i)))
            .unzip();
        // Corners meet by place (the polygons' own corners differ).
        let snap = width.max(1e-6) * 1e-3;
        let key = |p: Vec3| {
            let q = (p / snap).round();
            (q.x as i64, q.y as i64, q.z as i64)
        };
        let mut sides: HashMap<_, Vec<(usize, Vec3)>> = HashMap::new();
        for (n, t) in triangles.iter().enumerate() {
            for (i, j, k) in [(0, 1, 2), (1, 2, 0), (2, 0, 1)] {
                let (a, b) = (key(t.points[i]), key(t.points[j]));
                let edge = if a <= b { (a, b) } else { (b, a) };
                sides.entry(edge).or_default().push((n, t.points[k]));
            }
        }
        let mut folds = Vec::new();
        for (edge, list) in &sides {
            let [(first, _), (second, across)] = list.as_slice() else {
                continue;
            };
            let (t1, t2) = (triangles[*first], triangles[*second]);
            let angle = t1.normal.dot(t2.normal).clamp(-1.0, 1.0).acos();
            if angle < LEAST_FOLD || first == second {
                continue;
            }
            // Shaded smoothly across (the same normal at both ends, seen
            // from either side) and not too sharp: a facet, not an edge.
            let normal_at = |n: usize, end| {
                (0..3)
                    .find(|&i| key(triangles[n].points[i]) == end)
                    .map(|i| shading[n][i])
            };
            let soft = [edge.0, edge.1].into_iter().all(|end| {
                match (normal_at(*first, end), normal_at(*second, end)) {
                    (Some(p), Some(q)) => p.dot(q) > SAME_SHADING,
                    _ => false,
                }
            });
            if soft && angle < LEAST_SOFT_FOLD {
                continue;
            }
            // Out if the other triangle bends away behind this one.
            let a = t1
                .points
                .iter()
                .copied()
                .find(|p| key(*p) == edge.0)
                .unwrap_or(t1.points[0]);
            let b = t1
                .points
                .iter()
                .copied()
                .find(|p| key(*p) == edge.1)
                .unwrap_or(t1.points[1]);
            let out = t1.normal.dot(*across - a) < 0.0;
            let strength = (angle / std::f32::consts::FRAC_PI_2).min(1.0);
            folds.push(Fold {
                a,
                b,
                strength: if out { strength } else { -strength },
            });
        }
        let cell = width.max(1e-6) * 2.0;
        let mut min = Vec3::splat(f32::MAX);
        for f in &folds {
            min = min.min(f.a).min(f.b);
        }
        let mut edges = Self {
            folds,
            width: width.max(1e-6),
            min,
            cell,
            grid: HashMap::new(),
        };
        for (i, f) in edges.folds.iter().enumerate() {
            let lo = edges.cell_of(f.a.min(f.b) - Vec3::splat(edges.width));
            let hi = edges.cell_of(f.a.max(f.b) + Vec3::splat(edges.width));
            // Very long edges in very small cells: cap the cells walked.
            let span =
                (hi.0 - lo.0 + 1) as i64 * (hi.1 - lo.1 + 1) as i64 * (hi.2 - lo.2 + 1) as i64;
            if span > 1 << 20 {
                continue;
            }
            for z in lo.2..=hi.2 {
                for y in lo.1..=hi.1 {
                    for x in lo.0..=hi.0 {
                        edges.grid.entry((x, y, z)).or_default().push(i as u32);
                    }
                }
            }
        }
        edges
    }

    fn cell_of(&self, p: Vec3) -> (i32, i32, i32) {
        let q = ((p - self.min) / self.cell).floor();
        (q.x as i32, q.y as i32, q.z as i32)
    }

    /// The curvature at `p`: 0.5 flat, toward 1 near outer edges and 0
    /// near inner ones.
    pub(crate) fn at(&self, p: Vec3) -> f32 {
        let Some(list) = self.grid.get(&self.cell_of(p)) else {
            return 0.5;
        };
        let mut sum = 0.0f32;
        for &i in list {
            let f = self.folds[i as usize];
            let d = distance_to_segment(p, f.a, f.b);
            if d < self.width {
                let k = 1.0 - d / self.width;
                sum += f.strength * k * k;
            }
        }
        0.5 + 0.5 * sum.clamp(-1.0, 1.0)
    }
}

fn distance_to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}
