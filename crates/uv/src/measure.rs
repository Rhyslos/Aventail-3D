//! Measuring in the UV Editor ([decision 0115](../../../docs/decisions/0115-seeing-and-checking.md)):
//! how long a line drawn on the texture is on the surface of the shapes
//! under it.
//!
//! A [`Surface`] holds a shape's triangles twice, as they lie in the
//! texture and where they are in the world, sorted into a grid so the
//! triangle under a spot is found quickly. [`surface_length`] walks the
//! line in [`MEASURE_STEPS`] steps and adds up the real distance between
//! steps that land on the same piece; the parts between pieces (empty
//! texture, or a jump across a seam) count nothing.

use glam::{Mat4, Vec2, Vec3};
use mesh::Mesh;
use mesh::uvs::Uvs;

/// How many steps a measured line is walked in.
pub const MEASURE_STEPS: usize = 256;

/// The grid the triangles are sorted into, in cells across.
const GRID: usize = 64;

/// One triangle: its UVs, its world positions and its piece.
#[derive(Debug, Clone)]
struct Triangle {
    uv: [Vec2; 3],
    world: [Vec3; 3],
    piece: usize,
}

/// A shape's triangles in the texture and in the world.
#[derive(Debug, Clone)]
pub struct Surface {
    triangles: Vec<Triangle>,
    /// The triangles over each grid cell.
    cells: Vec<Vec<u32>>,
    min: Vec2,
    span: Vec2,
}

impl Surface {
    /// `mesh`'s surface, placed in the world by `to_world`; `None`
    /// without UVs or polygons.
    pub fn new(mesh: &Mesh, to_world: Mat4) -> Option<Self> {
        let uvs = Uvs::of(mesh)?;
        let (starts, _) = mesh.face_layout();
        let (island_of, _) = mesh::uvs::island_of_each(mesh);
        let mut triangles = Vec::new();
        for (f, list) in mesh.faces() {
            let start = starts[f.get() as usize] as usize;
            let piece = island_of.get(f.get() as usize).copied().unwrap_or(0);
            mesh::triangulate::face_triangles(mesh, list, |[a, b, c]| {
                let p = |i: usize| {
                    to_world.transform_point3(mesh.position(list[i]).unwrap_or_default())
                };
                triangles.push(Triangle {
                    uv: [uvs.at(start + a), uvs.at(start + b), uvs.at(start + c)],
                    world: [p(a), p(b), p(c)],
                    piece,
                });
            });
        }
        triangles.retain(|t| {
            t.uv.iter().all(|v| v.is_finite()) && t.world.iter().all(|p| p.is_finite())
        });
        if triangles.is_empty() {
            return None;
        }
        let mut min = Vec2::INFINITY;
        let mut max = Vec2::NEG_INFINITY;
        for t in &triangles {
            for v in t.uv {
                min = min.min(v);
                max = max.max(v);
            }
        }
        let span = (max - min).max(Vec2::splat(1e-6));
        let mut cells = vec![Vec::new(); GRID * GRID];
        for (i, t) in triangles.iter().enumerate() {
            let lo = t.uv[0].min(t.uv[1]).min(t.uv[2]);
            let hi = t.uv[0].max(t.uv[1]).max(t.uv[2]);
            let (x0, y0) = cell_of(lo, min, span);
            let (x1, y1) = cell_of(hi, min, span);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    cells[y * GRID + x].push(i as u32);
                }
            }
        }
        Some(Self {
            triangles,
            cells,
            min,
            span,
        })
    }

    /// The piece and world position under texture spot `uv`, if any
    /// polygon lies there (the first found, where pieces overlap).
    pub fn at(&self, uv: Vec2) -> Option<(usize, Vec3)> {
        let rel = (uv - self.min) / self.span;
        if !(0.0..=1.0).contains(&rel.x) || !(0.0..=1.0).contains(&rel.y) {
            return None;
        }
        let (x, y) = cell_of(uv, self.min, self.span);
        self.cells[y * GRID + x].iter().find_map(|i| {
            let t = &self.triangles[*i as usize];
            let w = weights(t.uv, uv)?;
            Some((
                t.piece,
                t.world[0] * w.x + t.world[1] * w.y + t.world[2] * w.z,
            ))
        })
    }
}

/// The grid cell holding `uv`.
fn cell_of(uv: Vec2, min: Vec2, span: Vec2) -> (usize, usize) {
    let rel = ((uv - min) / span * GRID as f32).floor();
    let clamp = |v: f32| (v.max(0.0) as usize).min(GRID - 1);
    (clamp(rel.x), clamp(rel.y))
}

/// `p`'s weights in triangle `t` (each 0 to 1, adding up to 1), if it
/// lies inside (a hair outside counts, so spots on an edge are found).
fn weights(t: [Vec2; 3], p: Vec2) -> Option<Vec3> {
    let twice = (t[1] - t[0]).perp_dot(t[2] - t[0]);
    if twice.abs() < 1e-12 {
        return None;
    }
    let a = (t[1] - p).perp_dot(t[2] - p) / twice;
    let b = (t[2] - p).perp_dot(t[0] - p) / twice;
    let c = 1.0 - a - b;
    const SLACK: f32 = -1e-4;
    (a >= SLACK && b >= SLACK && c >= SLACK).then_some(Vec3::new(a, b, c))
}

/// How long the line from `a` to `b` (in the texture) is on the surfaces
/// under it, in world units: the distance between steps on the same
/// piece of the same shape, added up.
pub fn surface_length(surfaces: &[Surface], a: Vec2, b: Vec2) -> f32 {
    let spot = |uv: Vec2| {
        surfaces
            .iter()
            .enumerate()
            .find_map(|(s, surface)| surface.at(uv).map(|(piece, p)| ((s, piece), p)))
    };
    let mut total = 0.0;
    let mut last = spot(a);
    for k in 1..=MEASURE_STEPS {
        let now = spot(a.lerp(b, k as f32 / MEASURE_STEPS as f32));
        if let (Some((was, p)), Some((is, q))) = (last, now)
            && was == is
        {
            total += p.distance(q);
        }
        last = now;
    }
    total
}

#[cfg(test)]
mod tests;
