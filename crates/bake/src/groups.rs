//! Splitting a shape's triangles into the groups a gradient runs across
//! ([decision 0199](../../../docs/decisions/0199-kit-finishing-touches.md)):
//! the whole object, its separate parts (triangles joined by shared
//! corners) or its UV pieces (triangles joined by shared UVs).

use glam::Vec3;
use paint::{GradientSpan, PaintSurface};
use std::collections::HashMap;

/// Each triangle's group, and how many groups there are.
pub(crate) struct Groups {
    /// The group of each triangle of the surface (`u32::MAX` for those
    /// not in the slots asked for).
    pub(crate) of_triangle: Vec<u32>,
    /// How many groups.
    pub(crate) count: usize,
}

/// `surface`'s triangles in `slots`, grouped by `span`.
pub(crate) fn of(surface: &PaintSurface, slots: &[u32], span: GradientSpan) -> Groups {
    let triangles = surface.triangles();
    let wanted: Vec<bool> = triangles.iter().map(|t| slots.contains(&t.slot)).collect();
    let mut parent: Vec<usize> = (0..triangles.len()).collect();
    if span != GradientSpan::Object {
        // Triangles sharing a corner (a place, or a UV) are joined.
        let mut first: HashMap<[i64; 3], usize> = HashMap::new();
        for (i, t) in triangles.iter().enumerate() {
            if !wanted[i] {
                continue;
            }
            for k in 0..3 {
                let key = match span {
                    GradientSpan::UvPiece => {
                        let uv = t.uvs[k];
                        [quantise(uv.x * 1e5), quantise(uv.y * 1e5), 0]
                    }
                    _ => {
                        let p = t.points[k];
                        [
                            quantise(p.x * 1e3),
                            quantise(p.y * 1e3),
                            quantise(p.z * 1e3),
                        ]
                    }
                };
                match first.get(&key) {
                    Some(&j) => join(&mut parent, i, j),
                    None => {
                        first.insert(key, i);
                    }
                }
            }
        }
    }
    let mut number: HashMap<usize, u32> = HashMap::new();
    let mut of_triangle = vec![u32::MAX; triangles.len()];
    for i in 0..triangles.len() {
        if !wanted[i] {
            continue;
        }
        let root = if span == GradientSpan::Object {
            0
        } else {
            find(&mut parent, i)
        };
        let next = number.len() as u32;
        of_triangle[i] = *number.entry(root).or_insert(next);
    }
    Groups {
        of_triangle,
        count: number.len(),
    }
}

/// The box around each group's triangles.
pub(crate) fn bounds(surface: &PaintSurface, groups: &Groups) -> Vec<(Vec3, Vec3)> {
    let mut boxes = vec![(Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)); groups.count.max(1)];
    for (t, &g) in surface.triangles().iter().zip(&groups.of_triangle) {
        if g == u32::MAX {
            continue;
        }
        let b = &mut boxes[g as usize];
        for p in t.points {
            b.0 = b.0.min(p);
            b.1 = b.1.max(p);
        }
    }
    for b in &mut boxes {
        if b.0.x > b.1.x {
            *b = (Vec3::ZERO, Vec3::ONE);
        }
    }
    boxes
}

fn quantise(v: f32) -> i64 {
    if v.is_finite() {
        v.round() as i64
    } else {
        i64::MIN
    }
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

fn join(parent: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    if ra != rb {
        parent[ra.max(rb)] = ra.min(rb);
    }
}
