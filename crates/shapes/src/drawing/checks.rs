//! The checks every piece passes before it is kept.

use super::{MAX_NAME, MIN_SPREAD, Piece, PieceKind, Problem};
use crate::path::Path;

/// Why `piece` can't be kept, if it can't. Its shape must be real
/// numbers and where it is placed within reach of the texture.
pub(super) fn check_piece(piece: &Piece) -> Result<(), Problem> {
    if piece.name.trim().is_empty() || piece.name.len() > MAX_NAME {
        return Err(Problem::BadName);
    }
    if piece.path.knots().len() > crate::path::MAX_KNOTS {
        return Err(Problem::TooMany);
    }
    if !piece.path.is_finite() {
        return Err(Problem::OutOfPlace);
    }
    if !piece.place.is_usable() {
        return Err(Problem::Squashed);
    }
    let placed = piece.placed();
    if placed.problem().is_some() {
        return Err(Problem::OutOfPlace);
    }
    match piece.kind {
        PieceKind::Line if placed.knots().len() < 2 => Err(Problem::ShortLine),
        PieceKind::Line => Ok(()),
        _ if !placed.is_closed() => Err(Problem::OpenPath),
        _ if !has_room(&placed) => Err(Problem::NoInside),
        _ => Ok(()),
    }
}

/// Whether a closed path has room inside: at least two points, not all
/// (handles included) on one line. Sides crossing over don't matter: a
/// figure of eight has room in both loops though its signed area is
/// nothing.
fn has_room(path: &Path) -> bool {
    let points: Vec<glam::Vec2> = path
        .knots()
        .iter()
        .flat_map(|k| [k.at, k.at + k.inn, k.at + k.out])
        .collect();
    let Some(first) = points.first().copied() else {
        return false;
    };
    let Some(far) = points
        .iter()
        .copied()
        .max_by(|a, b| a.distance(first).total_cmp(&b.distance(first)))
    else {
        return false;
    };
    let along = far - first;
    if path.knots().len() < 2 || along.length() < MIN_SPREAD {
        return false;
    }
    let across = along.normalize().perp();
    points
        .iter()
        .any(|p| (*p - first).dot(across).abs() >= MIN_SPREAD)
}
