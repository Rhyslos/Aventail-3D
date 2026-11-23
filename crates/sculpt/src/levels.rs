//! Detail levels ([decision 0315](../../../docs/decisions/0315-detail-levels.md)):
//! a sculpt's shape subdivided up to [`MAX_LEVELS`] times, each level
//! sculptable, the lower levels kept.
//!
//! Level 0 is the shape coming into the Sculpt layer with its moves. Each
//! level above is the level below, sculpted, subdivided once
//! ([`mesh::subdivide::refine`]: every point keeps its number, so a
//! level's first points are the level below's), with moves of its own
//! measured along the surface there. A stroke on a low level therefore
//! carries every finer level's detail along with it.
//!
//! Each level also keeps where its points are with nothing sculpted (the
//! shape coming in, subdivided plainly), so that when that shape changes
//! its points the level finds its place again by position, as level 0
//! does.

use crate::frames::frame;
use crate::moves::{CHUNK, Moves};
use glam::Vec3;
use mesh::subdivide::Refinement;
use mesh::{Mesh, VertexId};
use std::sync::Arc;

/// The most detail levels above the shape coming in.
pub const MAX_LEVELS: usize = 6;
/// The most polygons a detail level may have: adding one past this is
/// refused, as Remesh for sculpting refuses it.
pub const MAX_POLYGONS: u64 = mesh::remesh::MAX_SCULPT_POLYGONS;
/// From this many polygons a new level is heavy: slow to sculpt and to
/// bake, so the interface asks first.
pub const HEAVY_POLYGONS: u64 = mesh::remesh::HEAVY_POLYGONS;

/// Which detail level an export or a bake takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExportLevel {
    /// The level shown.
    #[default]
    Shown,
    /// The finest level there is.
    Finest,
    /// This level (the finest when there are fewer).
    Level(u8),
}

/// Why a detail level couldn't be added or changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelProblem {
    /// There are [`MAX_LEVELS`] already.
    TooManyLevels,
    /// The new level would have `polygons` polygons, more than
    /// [`MAX_POLYGONS`].
    TooHeavy {
        /// The polygons it would have.
        polygons: u64,
    },
    /// A point of the shape isn't at a real position, or the levels under
    /// it couldn't be worked out.
    NoShape,
}

/// One detail level's own data.
#[derive(Clone)]
pub(crate) struct Level {
    /// The structure of the shape coming into the layer that this level
    /// was made for: with another, its points are found again by
    /// position.
    pub(crate) structure: u64,
    /// Where its points are with nothing sculpted, by slot (not a number
    /// for a slot with no point).
    pub(crate) places: Arc<[Vec3]>,
    /// Each point's move, along its frame on the level below, sculpted
    /// and subdivided.
    pub(crate) moves: Moves,
}

impl Level {
    /// Whether two levels hold the same thing (shared parts aren't
    /// compared).
    pub(crate) fn same(&self, other: &Self) -> bool {
        self.structure == other.structure
            && (Arc::ptr_eq(&self.places, &other.places) || same_places(&self.places, &other.places))
            && self.moves == other.moves
    }
}

/// Places compared bit for bit (a slot with no point is not a number,
/// and equal to itself here).
pub(crate) fn same_places(a: &[Vec3], b: &[Vec3]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.to_array()
                .iter()
                .zip(y.to_array())
                .all(|(p, q)| p.to_bits() == q.to_bits())
        })
}

/// One level up from a sculpted shape: the finer shape before its own
/// moves, and how its points hang on the shape below.
pub(crate) struct Step {
    pub(crate) under: Mesh,
    pub(crate) refinement: Refinement,
}

/// How many polygons the level above `shape` has.
pub fn polygons_above(shape: &Mesh) -> u64 {
    mesh::subdivide::polygon_count(shape, 1)
}

/// The level above `shape` (a level as sculpted). `None` when it would
/// pass [`MAX_POLYGONS`] or a point isn't at a real position.
pub(crate) fn step_up(shape: &Mesh) -> Option<Step> {
    if shape.face_count() == 0 || polygons_above(shape) > MAX_POLYGONS {
        return None;
    }
    let (under, refinement) = mesh::subdivide::refine(shape).ok()?;
    Some(Step { under, refinement })
}

/// Moves `mesh`'s points by `moves`, each along its frame on `mesh` as it
/// comes in. Points in chunks where nothing moved aren't looked at.
pub(crate) fn move_points(mesh: &mut Mesh, moves: &Moves) {
    let len = moves.len().min(mesh.point_slot_count());
    let moved: Vec<usize> = moves
        .chunks()
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_some())
        .flat_map(|(c, _)| c * CHUNK..((c + 1) * CHUNK).min(len))
        .collect();
    if moved.is_empty() {
        return;
    }
    let new = sculpted_places(mesh, &moved, |i| moves.get(i));
    let placed: Vec<(VertexId, Vec3)> = moved
        .into_iter()
        .zip(new)
        .filter_map(|(i, p)| Some((VertexId::from_raw(i as u32), p?)))
        .collect();
    mesh.set_positions(&placed);
}

/// Where points `which` (slots of `mesh`) go when each moves by `move_of`
/// along its frame on `mesh`; `None` for a slot with no point. Worked out
/// on every core for many points, all from `mesh` as it is.
pub fn sculpted_places(
    mesh: &Mesh,
    which: &[usize],
    move_of: impl Fn(usize) -> Vec3 + Sync,
) -> Vec<Option<Vec3>> {
    let topology = mesh.topology();
    mesh::parallel::in_parallel(which.len(), |range| {
        which[range]
            .iter()
            .map(|&i| {
                let v = VertexId::from_raw(i as u32);
                let p = mesh.position(v)?;
                let m = move_of(i);
                if m == Vec3::ZERO {
                    return Some(p);
                }
                Some(p + frame(mesh, topology, v).to_object(m))
            })
            .collect::<Vec<_>>()
    })
    .into_iter()
    .flatten()
    .collect()
}

/// `mesh` with every point that has a place in `places` (by slot) put
/// there: a level's shape with its points worked out again.
pub(crate) fn with_places(mesh: &Mesh, places: &[Vec3]) -> Mesh {
    let placed: Vec<(VertexId, Vec3)> = places
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_finite())
        .map(|(i, p)| (VertexId::from_raw(i as u32), *p))
        .collect();
    let mut out = mesh.clone();
    out.set_positions(&placed);
    out
}
