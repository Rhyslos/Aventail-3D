//! Laying strips onto a trim sheet's rows
//! ([decision 0106](../../../docs/decisions/0106-trim-sheets.md)).
//!
//! A trim sheet is a shared texture of horizontal rows that repeat left to
//! right. [`place`] lays each piece holding the chosen polygons onto a
//! row: straight as a grid when it is a strip of square quads (else as it
//! lay, or unwrapped if it had no usable UVs), the right way round, its
//! long side running along the row, scaled so its height fills the row
//! exactly, and set from the row's left end, one piece after another.
//! Pieces may run past the square's right side: the row repeats there.
//!
//! [`on_row`] tells whether a piece lies on a row, so packing and the bake
//! check leave trim pieces where they are.

use crate::chart::Chart;
use crate::unwrap::{
    chosen_pieces, conformal, cuts_of, grid, old_spots, outcome, projected, signed_area,
    smallest_box_turn, usable_guess, write,
};
use glam::Vec2;
use mesh::edit::Outcome;
use mesh::uvs::Uvs;
use mesh::{FaceId, Mesh};

/// How far (as a share of the square) a piece's top and bottom may be
/// from a row's and still count as lying on it.
pub const ON_ROW: f32 = 1e-3;

/// Lays the pieces holding `faces` onto the row from `bottom` to `top`
/// (shares of the square's height). Rows that aren't real or have no
/// height change nothing.
pub fn place(mesh: &mut Mesh, faces: &[FaceId], bottom: f32, top: f32) -> Outcome {
    let cuts = cuts_of(mesh);
    let (pieces, found) = chosen_pieces(mesh, faces, &cuts);
    let height = top - bottom;
    if pieces.is_empty() || !bottom.is_finite() || !top.is_finite() || height <= 0.0 {
        return outcome(faces.len(), found, false);
    }
    let uvs = Uvs::of(mesh);
    let mut laid = Vec::new();
    // Where along the row the next piece starts.
    let mut along = 0.0;
    for piece in &pieces {
        let chart = Chart::of(mesh, piece, &cuts);
        let mut at = match grid(mesh, &chart) {
            Some(at) => at,
            None => match uvs.as_ref() {
                Some(u) if usable_guess(mesh, &chart, u) => old_spots(mesh, &chart, u)
                    .into_iter()
                    .map(|v| v.unwrap_or(Vec2::ZERO))
                    .collect(),
                _ => {
                    let flat = projected(mesh, &chart);
                    conformal(mesh, &chart, &flat).unwrap_or(flat)
                }
            },
        };
        // The right way round.
        let triangles = chart.triangles(mesh);
        let signed: f32 = triangles.iter().map(|t| signed_area(&at, *t)).sum();
        if signed < 0.0 {
            for v in &mut at {
                v.x = -v.x;
            }
        }
        // Lined up with the square, its long side along the row.
        let angle = smallest_box_turn(&at);
        let (s, c) = angle.sin_cos();
        for v in &mut at {
            *v = Vec2::new(c * v.x - s * v.y, s * v.x + c * v.y);
        }
        let (min, max) = bounds(&at);
        if max.y - min.y > max.x - min.x {
            for v in &mut at {
                *v = v.perp();
            }
        }
        // Its height filling the row, from where the last piece ended.
        let (min, max) = bounds(&at);
        let tall = max.y - min.y;
        if tall <= 1e-12 || !tall.is_finite() {
            continue;
        }
        let k = height / tall;
        for v in &mut at {
            *v = Vec2::new(along + (v.x - min.x) * k, bottom + (v.y - min.y) * k);
        }
        along += (max.x - min.x) * k;
        laid.push((chart, at));
    }
    let changed = !laid.is_empty() && write(mesh, &laid);
    outcome(faces.len(), found, changed)
}

fn bounds(at: &[Vec2]) -> (Vec2, Vec2) {
    at.iter()
        .fold((Vec2::INFINITY, Vec2::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(*v), hi.max(*v))
        })
}

/// Whether `piece` (polygons of `mesh`) lies on the row from `bottom` to
/// `top`: its lowest and highest UVs on the row's edges, to within
/// [`ON_ROW`].
pub fn on_row(mesh: &Mesh, piece: &[FaceId], bottom: f32, top: f32) -> bool {
    match mesh::uvs::uv_bounds(mesh, piece, &[]) {
        Some((min, max)) => (min.y - bottom).abs() <= ON_ROW && (max.y - top).abs() <= ON_ROW,
        None => false,
    }
}

#[cfg(test)]
mod tests;
