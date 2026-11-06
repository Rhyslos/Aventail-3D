//! Making pieces what they were told to be again ([decision 0248](../../../docs/decisions/0248-uv-intents.md)).
//!
//! After an edit changes a shape (and mending lays broken UVs out again),
//! [`reapply`] looks at each piece holding a changed polygon and, if the
//! piece has an intent (`mesh::uv_intents`), makes it so again: a strip is
//! straightened, a density is set, a trim row's piece is fitted back into
//! the row, an upright piece is turned so the world's up runs up the
//! texture. Pieces without an intent, locked pieces, and pieces the edit
//! didn't touch are left as they are.

use crate::pack::islands_of;
use glam::{Affine2, Vec2};
use mesh::uv_intents::{UvIntent, piece_intent};
use mesh::uvs::Uvs;
use mesh::{FaceId, Mesh};
use std::collections::BTreeSet;

/// Re-applies the intents of the pieces `mesh`'s edit changed (`before`
/// is the shape before it). Returns how many pieces were made so again.
pub fn reapply(before: &Mesh, mesh: &mut Mesh) -> usize {
    if !mesh::uv_intents::has_intents(mesh) || !mesh::uvs::has_uvs(mesh) {
        return 0;
    }
    let changed: BTreeSet<FaceId> = mesh
        .faces()
        .filter(|(f, list)| before.face(*f) != Some(*list) || uvs_differ(before, mesh, *f))
        .map(|(f, _)| f)
        .collect();
    if changed.is_empty() {
        return 0;
    }
    let pieces = islands_of(mesh, &changed.iter().copied().collect::<Vec<_>>());
    let mut done = 0;
    for piece in pieces {
        if mesh::uvs::any_locked(mesh, &piece) {
            continue;
        }
        let intent = piece_intent(mesh, &piece);
        if apply(mesh, &piece, intent) {
            done += 1;
        }
    }
    done
}

/// Whether polygon `f`'s UVs differ between `before` and `mesh`.
fn uvs_differ(before: &Mesh, mesh: &Mesh, f: FaceId) -> bool {
    let (Some(a), Some(b)) = (Uvs::of(before), Uvs::of(mesh)) else {
        return false;
    };
    let n = mesh.face(f).map_or(0, <[_]>::len);
    (0..n).any(|k| a.corner(before, f, k) != b.corner(mesh, f, k))
}

/// Makes the piece `faces` what `intent` says, now. Whether it changed.
pub fn apply(mesh: &mut Mesh, faces: &[FaceId], intent: UvIntent) -> bool {
    match intent {
        UvIntent::None => false,
        UvIntent::Strip => crate::unwrap::straighten(mesh, faces).matched > 0,
        UvIntent::Density(d) => crate::density::set_density(mesh, faces, d).matched > 0,
        UvIntent::TrimRow { bottom, top } => fit_rows(mesh, faces, bottom, top),
        UvIntent::Upright => upright(mesh, faces),
    }
}

/// The box round the piece's UVs.
fn bounds(mesh: &Mesh, faces: &[FaceId]) -> Option<(Vec2, Vec2)> {
    let uvs = Uvs::of(mesh)?;
    let (mut min, mut max) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    for f in faces {
        let n = mesh.face(*f).map_or(0, <[_]>::len);
        for k in 0..n {
            if let Some(p) = uvs.corner(mesh, *f, k) {
                min = min.min(p);
                max = max.max(p);
            }
        }
    }
    (min.x <= max.x && min.y <= max.y).then_some((min, max))
}

/// Scales the piece evenly so its height fills the row from `bottom` to
/// `top`, keeping its left side where it is.
fn fit_rows(mesh: &mut Mesh, faces: &[FaceId], bottom: f32, top: f32) -> bool {
    let Some((min, max)) = bounds(mesh, faces) else {
        return false;
    };
    let height = max.y - min.y;
    if height <= 1e-9 {
        return false;
    }
    let scale = (top - bottom) / height;
    if (scale - 1.0).abs() < 1e-5 && (min.y - bottom).abs() < 1e-5 {
        return false;
    }
    // p' = (p - min) * scale + (min.x, bottom)
    let transform = Affine2::from_translation(Vec2::new(min.x, bottom))
        * Affine2::from_scale(Vec2::splat(scale))
        * Affine2::from_translation(-min);
    mesh::uvs::transform_uvs(mesh, faces, &[], transform).matched > 0
}

/// Which way the world's up (+Z) runs over the piece's UVs: the piece's
/// gradient of height, weighted by area. `None` for a flat-lying piece.
fn up_direction(mesh: &Mesh, faces: &[FaceId]) -> Option<Vec2> {
    let uvs = Uvs::of(mesh)?;
    let mut total = Vec2::ZERO;
    for f in faces {
        let corners = mesh.face(*f)?;
        let n = corners.len();
        for k in 1..n.saturating_sub(1) {
            let (Some(p0), Some(p1), Some(p2)) = (
                mesh.position(corners[0]),
                mesh.position(corners[k]),
                mesh.position(corners[k + 1]),
            ) else {
                continue;
            };
            let (Some(t0), Some(t1), Some(t2)) = (
                uvs.corner(mesh, *f, 0),
                uvs.corner(mesh, *f, k),
                uvs.corner(mesh, *f, k + 1),
            ) else {
                continue;
            };
            // Height's change across the triangle in UV: solve the 2 × 2.
            let (e1, e2) = (t1 - t0, t2 - t0);
            let det = e1.perp_dot(e2);
            if det.abs() < 1e-12 {
                continue;
            }
            let (h1, h2) = (p1.z - p0.z, p2.z - p0.z);
            let grad = Vec2::new(h1 * e2.y - h2 * e1.y, h2 * e1.x - h1 * e2.x) / det;
            total += grad * det.abs();
        }
    }
    (total.length() > 1e-9).then(|| total.normalize())
}

/// Whether the piece `faces` has an up to keep: false for a piece lying
/// flat (a floor, a ceiling), whose height doesn't change across it, and
/// for one without UVs. [`UvIntent::Upright`] does nothing on such a piece.
pub fn has_up(mesh: &Mesh, faces: &[FaceId]) -> bool {
    up_direction(mesh, faces).is_some()
}

/// Turns the piece about its box's middle so the world's up runs up the
/// texture (+V).
fn upright(mesh: &mut Mesh, faces: &[FaceId]) -> bool {
    let (Some(up), Some((min, max))) = (up_direction(mesh, faces), bounds(mesh, faces)) else {
        return false;
    };
    let turn = Vec2::Y.to_angle() - up.to_angle();
    if turn.abs() < 1e-4 {
        return false;
    }
    let middle = (min + max) / 2.0;
    let transform = Affine2::from_translation(middle)
        * Affine2::from_angle(turn)
        * Affine2::from_translation(-middle);
    mesh::uvs::transform_uvs(mesh, faces, &[], transform).matched > 0
}

#[cfg(test)]
mod tests;
