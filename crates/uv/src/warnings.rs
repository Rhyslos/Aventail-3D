//! The UV Editor's always-on warnings ([decision 0115](../../../docs/decisions/0115-seeing-and-checking.md)):
//! pieces lying over other pieces, polygons folded over within their own
//! piece, pieces lying mirrored, so the texture would read backwards on
//! them, and pieces sticking out of the texture square
//! ([decision 0118](../../../docs/decisions/0118-select-connected-and-outside-warnings.md)).
//!
//! Worked out whenever the shown shapes change, so it must be quick: the
//! overlap test draws the pieces into a grid of [`WARNING_CELLS`] cells
//! across (see [`crate::check::check_sheet`]), coarser than the bake
//! check's. Pieces stacked exactly on an identical piece (to share
//! pixels) and pieces on a trim sheet's rows overlap on purpose, and
//! aren't warned about.

use crate::check::{SheetPiece, check_sheet};
use mesh::{FaceId, Mesh};
use std::collections::{BTreeMap, BTreeSet};

/// The grid the overlap warnings draw pieces into, in cells across the
/// texture square.
pub const WARNING_CELLS: u32 = 256;

/// What is wrong in one shape's UVs, polygon by polygon.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Warnings {
    /// Polygons of pieces lying over another piece, and polygons folded
    /// over within their own piece.
    pub overlapping: BTreeSet<FaceId>,
    /// Polygons of pieces lying mirrored.
    pub mirrored: BTreeSet<FaceId>,
    /// How many pieces overlap (another, or themselves).
    pub overlapping_pieces: usize,
    /// How many pieces lie mirrored.
    pub mirrored_pieces: usize,
    /// Polygons of pieces sticking out of the texture square (decision
    /// 0118): the part outside gets no texture of its own.
    pub outside: BTreeSet<FaceId>,
    /// How many pieces stick out of the square.
    pub outside_pieces: usize,
}

impl Warnings {
    /// Whether nothing is wrong.
    pub fn is_clean(&self) -> bool {
        self.overlapping.is_empty() && self.mirrored.is_empty() && self.outside.is_empty()
    }
}

/// How far past the texture square's sides a UV may lie and still count
/// as inside (the bake check's slack).
pub const OUTSIDE_SLACK: f32 = 1e-4;

/// The pieces of `mesh` and, for each, the polygons lying mirrored on
/// the texture: all of a piece mostly mirrored (by area), or those
/// folded against the rest of a piece mostly the right way round.
fn sides(mesh: &Mesh) -> Vec<(Vec<FaceId>, bool, Vec<FaceId>)> {
    let Some(uvs) = mesh::uvs::Uvs::of(mesh) else {
        return Vec::new();
    };
    let stretch: BTreeMap<FaceId, crate::stretch::Stretch> = crate::stretch::stretch_of(mesh)
        .unwrap_or_default()
        .into_iter()
        .collect();
    mesh::uvs::islands(mesh)
        .into_iter()
        .map(|island| {
            let area = |f: &FaceId| crate::density::areas(mesh, &[*f], &uvs).1;
            let flipped = |f: &FaceId| stretch.get(f).is_some_and(|s| s.flipped && !s.empty);
            let unflipped = |f: &FaceId| stretch.get(f).is_some_and(|s| !s.flipped && !s.empty);
            let back: f32 = island.iter().filter(|f| flipped(f)).map(area).sum();
            let front: f32 = island.iter().filter(|f| unflipped(f)).map(area).sum();
            let mirrored = back > front;
            let folded = island
                .iter()
                .copied()
                .filter(|f| if mirrored { unflipped(f) } else { flipped(f) })
                .collect();
            (island, mirrored, folded)
        })
        .collect()
}

/// The pieces of `mesh` lying mirrored on the texture (mostly, by area).
pub fn mirrored_pieces(mesh: &Mesh) -> Vec<Vec<FaceId>> {
    sides(mesh)
        .into_iter()
        .filter(|(_, mirrored, _)| *mirrored)
        .map(|(island, _, _)| island)
        .collect()
}

/// The warnings for `meshes` shown together. Pieces are checked for
/// overlaps against the others in the same texture: `group` gives each
/// piece's (mesh index, polygons) texture. Pieces on one of the trim
/// sheet's `rows` (bottom, top) aren't checked for overlaps.
pub fn warnings<G: Ord>(
    meshes: &[&Mesh],
    group: impl Fn(usize, &[FaceId]) -> G,
    rows: &[(f32, f32)],
) -> Vec<Warnings> {
    let mut out = vec![Warnings::default(); meshes.len()];
    let mut groups: BTreeMap<G, Vec<(usize, Vec<FaceId>)>> = BTreeMap::new();
    for (m, mesh) in meshes.iter().enumerate() {
        for (island, mirrored, folded) in sides(mesh) {
            if mirrored {
                out[m].mirrored.extend(island.iter().copied());
                out[m].mirrored_pieces += 1;
            }
            if !folded.is_empty() {
                out[m].overlapping.extend(folded);
                out[m].overlapping_pieces += 1;
            }
            let on_trim = rows
                .iter()
                .any(|(bottom, top)| crate::trim::on_row(mesh, &island, *bottom, *top));
            // Trim pieces run past the square's sides on purpose.
            if !on_trim
                && mesh::uvs::uv_bounds(mesh, &island, &[]).is_some_and(|(min, max)| {
                    min.min_element() < -OUTSIDE_SLACK || max.max_element() > 1.0 + OUTSIDE_SLACK
                })
            {
                out[m].outside.extend(island.iter().copied());
                out[m].outside_pieces += 1;
            }
            if !on_trim {
                groups
                    .entry(group(m, &island))
                    .or_default()
                    .push((m, island));
            }
        }
    }
    for members in groups.into_values() {
        let pieces: Vec<SheetPiece> = members
            .iter()
            .map(|(m, faces)| SheetPiece {
                mesh: meshes[*m],
                faces: faces.clone(),
            })
            .collect();
        let report = check_sheet(&pieces, WARNING_CELLS, 0);
        for p in report.overlapping {
            let (m, faces) = &members[p];
            let new = faces
                .iter()
                .filter(|f| out[*m].overlapping.insert(**f))
                .count();
            // A piece folded over and lying over another counts once.
            if new == faces.len() {
                out[*m].overlapping_pieces += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
