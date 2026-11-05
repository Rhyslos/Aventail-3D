//! Projections: UVs laid on by a flat, box, cylinder or ball shape placed
//! around the model, and world-aligned UVs for modular kits
//! ([decision 0235](../../../docs/decisions/0235-projections.md)).
//!
//! A projection is placed by `placement`, a matrix from the shape's own
//! space to the projection's: there, the projection shape is the box from
//! −1 to 1 on each axis (a flat projection's square is its X and Y, looked
//! at along Z; a cylinder and a ball stand along Z). Each chosen polygon's
//! corners are carried there and laid on:
//!
//! - **Flat** ([`Projection::Planar`]): X and Y.
//! - **Box**: each polygon flat along the box side it faces most (its
//!   normal's biggest part), turned so the texture reads the right way up
//!   and not mirrored on any side.
//! - **Cylinder**: the angle round Z (0 to 1 once round) and the height.
//!   Where a polygon crosses the line behind the cylinder (−Y), its
//!   corners are kept together on one side.
//! - **Ball** ([`Projection::Sphere`]): the angle round as for the
//!   cylinder, and the angle up from the bottom (0) to the top (1).
//!   Corners on a pole take their polygon's middle angle, so the polygons
//!   round a pole don't fold.
//!
//! Box, cylinder and ball projections mark seams on the edges where the
//! projected polygons' UVs part (between box sides, along the back line).
//!
//! The box from −1 to 1 becomes the texture square (0 to 1), so making the
//! projection bigger makes the texture bigger on the model. With a world
//! scale ([`ProjectOptions::world`]), no square is fitted: the projected
//! coordinates are multiplied by that scale (texture widths per unit), so
//! neighbouring kit pieces line up wherever they are.
//!
//! Locked pieces (decision 0112) are skipped. Nothing here is random: the
//! same shape and options give the same UVs.

use glam::{Mat4, Vec2, Vec3};
use mesh::edit::Outcome;
use mesh::uvs::Uvs;
use mesh::{EdgeKey, FaceId, Mesh};
use std::collections::{BTreeMap, BTreeSet};
use std::f32::consts::{PI, TAU};

/// The shape a projection lays UVs on with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Projection {
    /// Flat, along the projection's Z.
    #[default]
    Planar,
    /// Each polygon flat along the box side it faces most.
    Box,
    /// Round the projection's Z, and along it.
    Cylinder,
    /// Round the projection's Z, and up it by angle.
    Sphere,
}

impl Projection {
    /// Every kind, in menu order.
    pub const ALL: [Self; 4] = [Self::Planar, Self::Box, Self::Cylinder, Self::Sphere];
}

/// Everything a projection needs, recorded with it so it replays the same.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectOptions {
    /// Which shape.
    pub kind: Projection,
    /// From the shape's own space to the projection's (where the
    /// projection shape is the box from −1 to 1).
    pub placement: Mat4,
    /// With a scale (texture widths per unit of the projection's space),
    /// the coordinates are scaled by it instead of the box being fitted to
    /// the square: world-aligned UVs.
    pub world: Option<f32>,
}

impl Default for ProjectOptions {
    fn default() -> Self {
        Self {
            kind: Projection::Planar,
            placement: Mat4::IDENTITY,
            world: None,
        }
    }
}

impl ProjectOptions {
    /// Whether every number is real and the placement can be undone (a
    /// projection squashed flat has no inside).
    pub fn is_real(&self) -> bool {
        self.placement.is_finite()
            && self.placement.determinant().abs() > 1e-12
            && self.world.is_none_or(|w| w.is_finite() && w > 0.0)
    }
}

/// A placement that fits the box from −1 to 1 round `min` to `max` (in the
/// shape's own space), along the shape's own axes, a little bigger so
/// nothing sits on its edge. Flat sides get some thickness.
pub fn fit_box(min: Vec3, max: Vec3) -> Mat4 {
    let centre = (min + max) / 2.0;
    let half = ((max - min) / 2.0).max(Vec3::splat(1e-3));
    // The square stays square: the texture isn't stretched one way.
    let size = half.max_element();
    Mat4::from_scale_rotation_translation(
        Vec3::splat(size),
        glam::Quat::IDENTITY,
        centre,
    )
    .inverse()
}

/// Projects UVs onto `faces` of `mesh` (see the module's notes). Locked
/// pieces, polygons not in the mesh and polygons with no area are left
/// out. Returns what was done; nothing changes when the options aren't
/// real.
pub fn project(mesh: &mut Mesh, faces: &[FaceId], options: &ProjectOptions) -> Outcome {
    let wanted: BTreeSet<FaceId> = faces.iter().copied().collect();
    let found: Vec<FaceId> = wanted
        .iter()
        .copied()
        .filter(|f| mesh.face(*f).is_some())
        .collect();
    let missing = wanted.len() - found.len();
    if !options.is_real() {
        return Outcome {
            matched: 0,
            missing,
            skipped: found.len(),
        };
    }
    let locked = mesh::uvs::locked(mesh);
    let is_locked =
        |f: &FaceId| locked.as_ref().is_some_and(|l| l.get(f.get() as usize).copied().unwrap_or(false));
    let chosen: Vec<FaceId> = found.iter().copied().filter(|f| !is_locked(f)).collect();
    let skipped = found.len() - chosen.len();
    let (starts, _) = mesh.face_layout();
    let corners = mesh.face_layout().1.len();
    let mut values: Vec<Vec2> = match Uvs::of(mesh) {
        Some(uvs) => (0..corners).map(|i| uvs.at(i)).collect(),
        None => vec![Vec2::ZERO; corners],
    };
    let to = options.placement;
    // Which side of the cylinder's or ball's back line each polygon was
    // put on, to mark seams where neighbours differ.
    let mut wrapped: BTreeMap<FaceId, i32> = BTreeMap::new();
    let mut matched = 0;
    for f in &chosen {
        let Some(list) = mesh.face(*f) else {
            continue;
        };
        let at: Vec<Vec3> = list
            .iter()
            .map(|v| to.transform_point3(mesh.position(*v).unwrap_or(Vec3::ZERO)))
            .collect();
        let laid = match options.kind {
            Projection::Planar => at.iter().map(|p| Vec2::new(p.x, p.y)).collect(),
            Projection::Box => {
                // Box sides meet with the texture cut: seams there too.
                wrapped.insert(*f, 0);
                box_side(&at)
            }
            Projection::Cylinder | Projection::Sphere => {
                let (laid, side) = round(&at, options.kind == Projection::Sphere);
                wrapped.insert(*f, side);
                laid
            }
        };
        let start = starts[f.get() as usize] as usize;
        for (k, uv) in laid.into_iter().enumerate() {
            let uv = match options.world {
                Some(scale) => uv * scale,
                None => match options.kind {
                    // Round projections are already 0 to 1 round; their
                    // height (or angle up) is too.
                    Projection::Cylinder => Vec2::new(uv.x, uv.y * 0.5 + 0.5),
                    Projection::Sphere => uv,
                    _ => uv * 0.5 + Vec2::splat(0.5),
                },
            };
            if let Some(slot) = values.get_mut(start + k)
                && uv.is_finite()
            {
                *slot = uv;
            }
        }
        matched += 1;
    }
    if matched > 0 {
        mesh::uvs::set_uvs(mesh, values);
        let cuts = wrap_seams(mesh, &wrapped);
        if !cuts.is_empty() {
            mesh::uvs::set_seams(mesh, &cuts, true);
        }
    }
    Outcome {
        matched,
        missing,
        skipped,
    }
}

/// A polygon's corners laid flat along the box side its normal faces
/// most, read so that up is +Z on the sides, and not mirrored.
fn box_side(at: &[Vec3]) -> Vec<Vec2> {
    let normal = newell_normal(at);
    let a = normal.abs();
    let flat = |p: &Vec3| -> Vec2 {
        if a.z >= a.x && a.z >= a.y {
            // Top or bottom: X across, Y up (bottom mirrored back).
            if normal.z >= 0.0 {
                Vec2::new(p.x, p.y)
            } else {
                Vec2::new(-p.x, p.y)
            }
        } else if a.x >= a.y {
            // Right or left side.
            if normal.x >= 0.0 {
                Vec2::new(p.y, p.z)
            } else {
                Vec2::new(-p.y, p.z)
            }
        } else if normal.y >= 0.0 {
            // Back or front.
            Vec2::new(-p.x, p.z)
        } else {
            Vec2::new(p.x, p.z)
        }
    };
    at.iter().map(flat).collect()
}

/// The polygon's facing by Newell's method (fine for bent n-gons).
fn newell_normal(at: &[Vec3]) -> Vec3 {
    let mut n = Vec3::ZERO;
    for (i, p) in at.iter().enumerate() {
        let q = at[(i + 1) % at.len()];
        n += Vec3::new(
            (p.y - q.y) * (p.z + q.z),
            (p.z - q.z) * (p.x + q.x),
            (p.x - q.x) * (p.y + q.y),
        );
    }
    n
}

/// How close to the middle line (in the projection's units) a corner
/// counts as on a pole.
const POLE: f32 = 1e-4;

/// A polygon's corners round the Z axis: U the angle round (0 at the back,
/// −Y, going round through +X), V the height (`ball`: the angle up, 0 to
/// 1). The corners are kept on one side of the back line: returns which
/// turn they were put on (0, or 1 when moved one turn on).
fn round(at: &[Vec3], ball: bool) -> (Vec<Vec2>, i32) {
    let angle = |p: &Vec3| -> Option<f32> {
        (Vec2::new(p.x, p.y).length() > POLE).then(|| {
            // 0 at −Y, a quarter at +X, a half at +Y.
            ((p.x.atan2(-p.y) / TAU) + 1.0).rem_euclid(1.0)
        })
    };
    let angles: Vec<Option<f32>> = at.iter().map(angle).collect();
    // Keep the corners within half a turn of the first real one.
    let first = angles.iter().flatten().copied().next().unwrap_or(0.0);
    let mut kept: Vec<Option<f32>> = angles
        .iter()
        .map(|a| {
            a.map(|a| {
                let d = a - first;
                if d > 0.5 {
                    a - 1.0
                } else if d < -0.5 {
                    a + 1.0
                } else {
                    a
                }
            })
        })
        .collect();
    // Below 0 anywhere: the whole polygon moves one turn on, so it stays
    // in the square from 0.
    let lowest = kept.iter().flatten().copied().fold(f32::INFINITY, f32::min);
    let side = i32::from(lowest < 0.0);
    if side == 1 {
        for a in kept.iter_mut().flatten() {
            *a += 1.0;
        }
    }
    // A pole's corner takes the middle angle of the rest.
    let real: Vec<f32> = kept.iter().flatten().copied().collect();
    let middle = if real.is_empty() {
        0.5
    } else {
        real.iter().sum::<f32>() / real.len() as f32
    };
    let laid = at
        .iter()
        .zip(kept)
        .map(|(p, a)| {
            let u = a.unwrap_or(middle);
            let v = if ball {
                let r = p.length().max(1e-12);
                (p.z / r).clamp(-1.0, 1.0).asin() / PI + 0.5
            } else {
                p.z
            };
            Vec2::new(u, v)
        })
        .collect();
    (laid, side)
}

/// The edges between projected polygons whose corners there differ in
/// the texture (across a round projection's back line, between a box's
/// sides): they're cut in the texture, so they become seams.
fn wrap_seams(mesh: &Mesh, wrapped: &BTreeMap<FaceId, i32>) -> Vec<EdgeKey> {
    if wrapped.is_empty() {
        return Vec::new();
    }
    let topology = mesh.topology();
    let mut out = BTreeSet::new();
    let (starts, _) = mesh.face_layout();
    let Some(uvs) = Uvs::of(mesh) else {
        return Vec::new();
    };
    for f in wrapped.keys() {
        for e in topology.face_edges(*f) {
            let others: Vec<FaceId> = topology.edge_faces(*e).filter(|g| g != f).collect();
            for g in others {
                if !wrapped.contains_key(&g) {
                    continue;
                }
                // Cut wherever the two polygons' corners at the edge's
                // ends differ in the texture.
                let (a, b) = e.ends();
                let corner = |face: FaceId, v: mesh::VertexId| -> Option<Vec2> {
                    let list = mesh.face(face)?;
                    let k = list.iter().position(|w| *w == v)?;
                    Some(uvs.at(starts[face.get() as usize] as usize + k))
                };
                let apart = [a, b].iter().any(|v| match (corner(*f, *v), corner(g, *v)) {
                    (Some(p), Some(q)) => p.distance(q) > 1e-4,
                    _ => false,
                });
                if apart {
                    out.insert(*e);
                }
            }
        }
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests;
