//! Shrinkwrap: moving a shape's points onto another shape's surface
//! ([decision 0147](../../../docs/decisions/0147-curve-deform-and-shrinkwrap.md)),
//! the work of the *Shrinkwrap* modifier.
//!
//! Each point (or each chosen point) goes to a place on the target's
//! surface:
//!
//! - **Nearest surface point** ([`WrapMode::Nearest`]): straight to the
//!   nearest place, wherever it is. Snug, but points far round a corner
//!   can bunch up on it.
//! - **Along the shape's normals** ([`WrapMode::Normals`]): along the way
//!   the shape faces at the point (its polygons' facings round it,
//!   weighted by their areas), forwards or backwards, to whichever
//!   meeting is nearer. Keeps the points spread as they were on a shape
//!   much like the target, such as a decal sheet laid over a body.
//! - **Along an axis** ([`WrapMode::AxisX`] and the like): straight along
//!   one of the shape's own axes, either way: a road dropped onto
//!   terrain along Z.
//!
//! A point whose line meets nothing stays where it is. **Offset** then
//! lifts each moved point off the surface along the way the target faces
//! there (negative sinks it in), so a wrapped sheet can float just above
//! the surface it hugs.
//!
//! The target's nearest places and meetings are found with a tree of
//! boxes ([`crate::bvh`]). Only points move: polygons, ids, UVs, seams and
//! materials stay, as for every deformer.

use crate::bvh::TriangleTree;
use crate::{Mesh, VertexId};
use glam::Vec3;
use std::collections::BTreeSet;
use words::tr;

/// How each point finds its place on the target's surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapMode {
    /// The nearest place on the surface.
    #[default]
    Nearest,
    /// Along the shape's own normals at the point, either way.
    Normals,
    /// Along the shape's X, either way.
    AxisX,
    /// Along the shape's Y, either way.
    AxisY,
    /// Along the shape's Z, either way.
    AxisZ,
}

impl WrapMode {
    /// Every mode, in the order the interface lists them.
    pub const ALL: [Self; 5] = [
        Self::Nearest,
        Self::Normals,
        Self::AxisX,
        Self::AxisY,
        Self::AxisZ,
    ];

    /// Its name, as the interface shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Nearest => tr("Nearest surface point"),
            Self::Normals => tr("Along the shape's normals"),
            Self::AxisX => tr("Along X"),
            Self::AxisY => tr("Along Y"),
            Self::AxisZ => tr("Along Z"),
        }
    }
}

/// How a shape is wrapped onto a target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShrinkwrapOptions {
    /// How each point finds its place.
    pub mode: WrapMode,
    /// How far off the surface the points end up, along the way the
    /// surface faces, in centimetres (negative is inside).
    pub offset: f32,
}

impl Default for ShrinkwrapOptions {
    fn default() -> Self {
        Self {
            mode: WrapMode::Nearest,
            offset: 0.0,
        }
    }
}

impl ShrinkwrapOptions {
    /// The furthest offset either way, in centimetres (1 km).
    pub const MAX_OFFSET: f32 = 100_000.0;

    /// The same options within limits; an offset that isn't a real number
    /// becomes 0.
    pub fn clamped(self) -> Self {
        Self {
            mode: self.mode,
            offset: if self.offset.is_finite() {
                self.offset.clamp(-Self::MAX_OFFSET, Self::MAX_OFFSET)
            } else {
                0.0
            },
        }
    }
}

/// Why a shape couldn't be wrapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShrinkwrapProblem {
    /// The target has no polygons with a surface.
    NoSurface,
}

impl std::fmt::Display for ShrinkwrapProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSurface => f.write_str(tr(
                "the target has no surface to wrap onto: give it polygons",
            )),
        }
    }
}

impl std::error::Error for ShrinkwrapProblem {}

/// The way the shape faces at each point (by id number): the sum of its
/// polygons' facings round it, each as long as twice the polygon's area
/// (Newell's), so big polygons count for more. Of unit length, or nothing
/// for a point with no polygons, or whose polygons cancel out.
pub fn point_normals(mesh: &Mesh) -> Vec<Vec3> {
    let mut sums = vec![Vec3::ZERO; mesh.point_slot_count()];
    for (_, corners) in mesh.faces() {
        let points: Vec<Vec3> = corners
            .iter()
            .map(|v| mesh.position(*v).unwrap_or(Vec3::ZERO))
            .collect();
        let mut n = Vec3::ZERO;
        for i in 0..points.len() {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            n += Vec3::new(
                (a.y - b.y) * (a.z + b.z),
                (a.z - b.z) * (a.x + b.x),
                (a.x - b.x) * (a.y + b.y),
            );
        }
        if !n.is_finite() {
            continue;
        }
        for v in corners {
            if let Some(sum) = sums.get_mut(v.get() as usize) {
                *sum += n;
            }
        }
    }
    sums.into_iter()
        .map(|n| n.try_normalize().unwrap_or(Vec3::ZERO))
        .collect()
}

/// `mesh` with its points (those in `only`, or all) moved onto `target`'s
/// surface (in the mesh's space) by `options`. Only the points move. A
/// target with no surface is refused.
pub fn shrinkwrap(
    mesh: &Mesh,
    target: &TriangleTree,
    only: Option<&BTreeSet<VertexId>>,
    options: &ShrinkwrapOptions,
) -> Result<Mesh, ShrinkwrapProblem> {
    if target.is_empty() {
        return Err(ShrinkwrapProblem::NoSurface);
    }
    let options = options.clamped();
    let normals = match options.mode {
        WrapMode::Normals => point_normals(mesh),
        _ => Vec::new(),
    };
    let points: Vec<(VertexId, Vec3)> = mesh
        .vertices()
        .filter(|(id, _)| only.is_none_or(|o| o.contains(id)))
        .collect();
    let parts = crate::parallel::in_parallel(points.len(), |range| {
        points[range]
            .iter()
            .filter_map(|&(id, p)| {
                let hit = match options.mode {
                    WrapMode::Nearest => target.nearest(p),
                    WrapMode::Normals => {
                        let way = normals.get(id.get() as usize).copied()?;
                        target.along_line(p, way)
                    }
                    WrapMode::AxisX => target.along_line(p, Vec3::X),
                    WrapMode::AxisY => target.along_line(p, Vec3::Y),
                    WrapMode::AxisZ => target.along_line(p, Vec3::Z),
                }?;
                let moved = hit.point + hit.normal * options.offset;
                moved.is_finite().then_some((id, moved))
            })
            .collect::<Vec<_>>()
    });
    let mut out = mesh.clone();
    for (id, p) in parts.into_iter().flatten() {
        out.set_position(id, p);
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
