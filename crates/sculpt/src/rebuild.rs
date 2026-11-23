//! Rebuild lower levels ([decision 0315](../../../docs/decisions/0315-detail-levels.md)):
//! detail levels worked out for a shape that has none under it, such as
//! one that came from Remesh for sculpting.
//!
//! A remeshed shape's polygons weren't made by subdividing anything, so
//! there is no coarser shape to find inside it. Instead a coarser shape is
//! made for it: the shape is remeshed again with voxels `2^levels` times
//! the size (about `4^levels` times fewer polygons), and that coarse shape
//! is subdivided back up, each level's points laid on the original
//! surface and kept as that level's moves. The finest level then has
//! about as many polygons as the shape had, and lies on it.

use crate::frames::frame;
use crate::layer::Sculpt;
use crate::levels::{self, MAX_LEVELS};
use crate::mask::Mask;
use crate::moves::{CHUNK, Moves};
use crate::nearest::Grid;
use glam::Vec3;
use mesh::bvh::TriangleTree;
use mesh::remesh::{Progress, RemeshProblem, SculptRemeshOptions};
use mesh::{Mesh, VertexId};
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

/// The fewest polygons the coarsest level may have: with fewer, fewer
/// levels are made.
pub const FEWEST_POLYGONS: u64 = 200;
/// The share of the work the remesh is; the rest lays the levels on the
/// surface.
const REMESH_SHARE: f32 = 0.4;

/// How far a rebuild has got, and a way to stop it, shared with another
/// thread.
#[derive(Debug, Default)]
pub struct RebuildProgress {
    remesh: Progress,
    /// How far the levels are laid, in thousandths.
    laid: AtomicU32,
}

impl RebuildProgress {
    /// How far it is, 0 to 1.
    pub fn share(&self) -> f32 {
        self.remesh.share() * REMESH_SHARE
            + self.laid.load(Relaxed) as f32 / 1000.0 * (1.0 - REMESH_SHARE)
    }

    /// Asks it to stop.
    pub fn stop(&self) {
        self.remesh.stop();
    }

    /// Whether it was asked to stop.
    pub fn cancelled(&self) -> bool {
        self.remesh.cancelled()
    }

    fn set_laid(&self, share: f32) {
        self.laid
            .fetch_max((share.clamp(0.0, 1.0) * 1000.0) as u32, Relaxed);
    }
}

/// Why lower levels couldn't be rebuilt.
#[derive(Debug, Clone, PartialEq)]
pub enum RebuildProblem {
    /// The shape has too few polygons for even one lower level of
    /// [`FEWEST_POLYGONS`].
    TooFew,
    /// The coarser shape couldn't be made.
    Remesh(RemeshProblem),
    /// A level couldn't be subdivided (a point that isn't a real number,
    /// or too many polygons).
    Level,
    /// It was stopped.
    Cancelled,
}

/// A shape with its lower levels rebuilt.
#[derive(Debug, Clone)]
pub struct Rebuilt {
    /// The coarsest level: the new shape for the sculpt to sit on.
    pub base: Mesh,
    /// The sculpt made for `base`: a level for each level rebuilt, the
    /// finest shown.
    pub sculpt: Sculpt,
}

/// How many lower levels a shape of `polygons` polygons can be given, at
/// most `wanted`: each takes a quarter of the polygons, and the coarsest
/// keeps at least [`FEWEST_POLYGONS`].
pub fn levels_for(polygons: u64, wanted: usize) -> usize {
    let mut levels = 0;
    let mut left = polygons;
    while levels < wanted.min(MAX_LEVELS) && left / 4 >= FEWEST_POLYGONS {
        left /= 4;
        levels += 1;
    }
    levels
}

/// `shape` (a sculpt as it looks) with up to `wanted` lower levels
/// rebuilt (see the module notes). `mask` is the shape's mask, a value
/// for each of its points, carried to the new finest level by position.
/// Progress goes to `progress`, which can stop it.
pub fn rebuild_lower(
    shape: &Mesh,
    wanted: usize,
    mask: Option<&Mask>,
    progress: &RebuildProgress,
) -> Result<Rebuilt, RebuildProblem> {
    let polygons = shape.face_count() as u64;
    let count = levels_for(polygons, wanted);
    if count == 0 {
        return Err(RebuildProblem::TooFew);
    }
    let area = mesh::remesh::projected_area(shape);
    let options = SculptRemeshOptions {
        voxel_size: mesh::remesh::voxel_for(area, polygons >> (2 * count)),
        ..Default::default()
    };
    let remeshed = mesh::remesh::remesh_for_sculpting(shape, &options, None, &progress.remesh)
        .map_err(|why| match why {
            RemeshProblem::Cancelled => RebuildProblem::Cancelled,
            why => RebuildProblem::Remesh(why),
        })?;
    let base = remeshed.mesh;
    let surface = TriangleTree::of_mesh(shape, glam::Mat4::IDENTITY);
    let mut sculpt = Sculpt::new(&base);
    let structure = sculpt.structure();
    let mut places: Vec<Vec3> = base.point_slots().map(|p| p.unwrap_or(Vec3::ZERO)).collect();
    let mut level = base.clone();
    for k in 1..=count {
        if progress.cancelled() {
            return Err(RebuildProblem::Cancelled);
        }
        let step = levels::step_up(&level).ok_or(RebuildProblem::Level)?;
        places = step
            .refinement
            .places(&places)
            .ok_or(RebuildProblem::Level)?;
        let moves = laid_on(&step.under, &surface);
        sculpt = sculpt
            .with_level(structure, places.clone(), moves)
            .ok_or(RebuildProblem::Level)?;
        level = step.under;
        if let Some(moves) = sculpt.level_moves(k) {
            levels::move_points(&mut level, moves);
        }
        progress.set_laid(k as f32 / count as f32);
    }
    let mut sculpt = sculpt.showing(count).ok_or(RebuildProblem::Level)?;
    if let Some(mask) = mask.filter(|m| !m.is_zero() && m.len() == shape.point_slot_count())
        && let Some(carried) = mask_by_position(shape, mask, &level)
        && let Some(with) = sculpt.with_mask_at(count, carried)
    {
        sculpt = with;
    }
    Ok(Rebuilt { base, sculpt })
}

/// The moves that lay each point of `under` on `surface` at the place
/// nearest it, each along its frame on `under`. Worked out on every
/// core.
fn laid_on(under: &Mesh, surface: &TriangleTree) -> Moves {
    let len = under.point_slot_count();
    let topology = under.topology();
    let found: Vec<Vec3> = mesh::parallel::in_parallel(len, |range| {
        range
            .map(|i| {
                let v = VertexId::from_raw(i as u32);
                let Some(p) = under.position(v) else {
                    return Vec3::ZERO;
                };
                let Some(hit) = surface.nearest(p) else {
                    return Vec3::ZERO;
                };
                let m = frame(under, topology, v).from_object(hit.point - p);
                if m.is_finite() { m } else { Vec3::ZERO }
            })
            .collect::<Vec<_>>()
    })
    .into_iter()
    .flatten()
    .collect();
    let mut moves = Moves::zero(len);
    for (c, chunk) in found.chunks(CHUNK).enumerate() {
        moves.set_chunk(c, chunk.to_vec());
    }
    moves
}

/// `mask` (a value for each point of `old`) for the points of `new`:
/// each takes the value of the nearest point of `old`.
fn mask_by_position(old: &Mesh, mask: &Mask, new: &Mesh) -> Option<Mask> {
    let places: Vec<Vec3> = old.point_slots().map(|p| p.unwrap_or(Vec3::NAN)).collect();
    let grid = Grid::new(&places)?;
    let len = new.point_slot_count();
    let values: Vec<u8> = mesh::parallel::in_parallel(len, |range| {
        range
            .map(|i| {
                new.position(VertexId::from_raw(i as u32))
                    .and_then(|p| grid.nearest(&places, p))
                    .map_or(0, |j| mask.get(j as usize))
            })
            .collect::<Vec<_>>()
    })
    .into_iter()
    .flatten()
    .collect();
    Some(Mask::from_values(&values))
}
