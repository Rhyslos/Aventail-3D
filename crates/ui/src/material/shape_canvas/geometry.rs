//! The shape canvas's geometry: where the gizmo's parts are, the
//! selected pieces' middle, the change a drag makes, a point or handle
//! dragged, and the click rule.

use super::ShapeView;
use super::pointer::{ARROW, Gizmo, How, Part};
use egui::{Modifiers, Pos2, Rect};
use glam::Vec2;
use shapes::{Change, Drawing, PieceId};

/// The middle of pieces `ids` (their points' rectangle's middle).
pub(crate) fn middle_of(drawing: &Drawing, ids: impl IntoIterator<Item = PieceId>) -> Option<Vec2> {
    let ids: Vec<PieceId> = ids.into_iter().collect();
    drawing.middle_of(&ids)
}

/// The gizmo's parts on screen round `m`: (part, where its handle is).
pub(crate) fn gizmo_parts(gizmo: Gizmo, m: Pos2) -> Vec<(Part, Pos2)> {
    let x = m + egui::vec2(ARROW, 0.0);
    let y = m + egui::vec2(0.0, -ARROW);
    let sx = m + egui::vec2(ARROW + 18.0, 0.0);
    let sy = m + egui::vec2(0.0, -ARROW - 18.0);
    match gizmo {
        Gizmo::Move => vec![(Part::MoveX, x), (Part::MoveY, y)],
        Gizmo::Scale => vec![(Part::ScaleX, x), (Part::ScaleY, y)],
        Gizmo::Rotate => vec![(Part::Ring, m)],
        Gizmo::All => vec![
            (Part::MoveX, x),
            (Part::MoveY, y),
            (Part::ScaleX, sx),
            (Part::ScaleY, sy),
            (Part::Ring, m),
        ],
    }
}

/// The app's rule for a click (decision 0133): Ctrl adds, Shift takes
/// out, otherwise only these (a piece clicked, or its whole group).
pub(super) fn choose(
    selected: &mut std::collections::BTreeSet<PieceId>,
    ids: &[PieceId],
    modifiers: Modifiers,
) {
    if modifiers.shift {
        for id in ids {
            selected.remove(id);
        }
    } else if modifiers.command {
        selected.extend(ids.iter().copied());
    } else {
        *selected = ids.iter().copied().collect();
    }
}

/// A point dragged near the mirror line snaps onto it.
pub(super) fn self_snap(drawing: &Drawing, area: Rect, view: &ShapeView, p: Vec2) -> Vec2 {
    let mirror = drawing.mirror();
    if mirror.on && (p.x - mirror.x).abs() * view.scale(area) < 8.0 {
        Vec2::new(mirror.x, p.y)
    } else {
        p
    }
}

/// The change a drag from `start` to `now` makes, round `middle`.
pub(crate) fn change_for(how: How, middle: Vec2, start: Vec2, now: Vec2) -> Change {
    match how {
        How::Move(axis) => {
            let mut by = now - start;
            match axis {
                Some(0) => by.y = 0.0,
                Some(_) => by.x = 0.0,
                None => {}
            }
            Change::moved(by)
        }
        How::Rotate => {
            let (a, b) = (start - middle, now - middle);
            Change::turned(middle, b.y.atan2(b.x) - a.y.atan2(a.x))
        }
        How::Scale(axis) => {
            let ratio = |a: f32, b: f32| {
                if a.abs() < 1e-6 {
                    1.0
                } else {
                    (b / a).abs().max(0.01)
                }
            };
            let (a, b) = (start - middle, now - middle);
            let scale = match axis {
                Some(0) => Vec2::new(ratio(a.x, b.x), 1.0),
                Some(_) => Vec2::new(1.0, ratio(a.y, b.y)),
                None => Vec2::splat(ratio(a.length(), b.length())),
            };
            Change::scaled(middle, scale)
        }
    }
}

/// `drawing` with point `index` of piece `id` (or one of its handles)
/// dragged to `p`. A smooth point's handles stay in line: moving one
/// turns the other to match, keeping its length.
pub(crate) fn knot_moved(
    drawing: &Drawing,
    id: PieceId,
    index: usize,
    handle: Option<bool>,
    p: Vec2,
) -> Result<Drawing, shapes::Problem> {
    let piece = drawing.piece(id).ok_or(shapes::Problem::Gone)?;
    let placed = piece.placed();
    let Some(mut k) = placed.knots().get(index).copied() else {
        return Err(shapes::Problem::Gone);
    };
    let in_line = |a: Vec2, b: Vec2| {
        a != Vec2::ZERO && b != Vec2::ZERO && a.normalize().dot(b.normalize()) < -0.999
    };
    match handle {
        None => k.at = p,
        Some(out) => {
            let smooth = in_line(k.inn, k.out);
            let pulled = p - k.at;
            let (this, other) = if out {
                (&mut k.out, &mut k.inn)
            } else {
                (&mut k.inn, &mut k.out)
            };
            *this = pulled;
            if smooth {
                *other = -pulled.normalize_or_zero() * other.length();
            }
        }
    }
    drawing.with_path(id, placed.with_knot(index, k))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapes::{Knot, PieceKind};

    #[test]
    fn drags_follow_the_pull() {
        let m = Vec2::splat(0.5);
        let moved = change_for(How::Move(Some(0)), m, Vec2::ZERO, Vec2::new(0.2, 0.3));
        assert_eq!(moved.point(Vec2::ZERO), Vec2::new(0.2, 0.0));
        let turned = change_for(How::Rotate, m, Vec2::new(0.7, 0.5), Vec2::new(0.5, 0.7));
        assert!(
            turned
                .point(Vec2::new(0.7, 0.5))
                .distance(Vec2::new(0.5, 0.7))
                < 1e-5
        );
        let sized = change_for(
            How::Scale(None),
            m,
            Vec2::new(0.6, 0.5),
            Vec2::new(0.7, 0.5),
        );
        assert!(
            sized
                .point(Vec2::new(0.6, 0.5))
                .distance(Vec2::new(0.7, 0.5))
                < 1e-5
        );
        let across = change_for(
            How::Scale(Some(0)),
            m,
            Vec2::new(0.6, 0.5),
            Vec2::new(0.4, 0.9),
        );
        assert!(
            across
                .point(Vec2::new(0.6, 0.6))
                .distance(Vec2::new(0.6, 0.6))
                < 1e-5
        );
        // Through the middle never flips or vanishes.
        let tiny = change_for(How::Scale(None), m, Vec2::new(0.6, 0.5), m);
        assert!(!tiny.flips() && tiny.matrix.determinant() > 0.0);
    }

    #[test]
    fn a_smooth_points_handles_stay_in_line() {
        let path = shapes::Path::new(
            vec![
                Knot::smooth(Vec2::new(0.3, 0.5), Vec2::new(0.1, 0.0)),
                Knot::corner(Vec2::new(0.7, 0.3)),
                Knot::corner(Vec2::new(0.7, 0.7)),
            ],
            true,
        );
        let d = Drawing::new()
            .with_piece("P", path, PieceKind::Add)
            .unwrap();
        let id = d.pieces()[0].id;
        let moved = knot_moved(&d, id, 0, Some(true), Vec2::new(0.3, 0.7)).unwrap();
        let k = moved.piece(id).unwrap().path.knots()[0];
        assert!(k.out.distance(Vec2::new(0.0, 0.2)) < 1e-6);
        assert!(k.inn.distance(Vec2::new(0.0, -0.1)) < 1e-6);
        let point = knot_moved(&d, id, 1, None, Vec2::new(0.8, 0.2)).unwrap();
        assert_eq!(
            point.piece(id).unwrap().path.knots()[1].at,
            Vec2::new(0.8, 0.2)
        );
        assert!(knot_moved(&d, id, 9, None, Vec2::ZERO).is_err());
        assert!(knot_moved(&d, PieceId(9), 0, None, Vec2::ZERO).is_err());
    }
}
