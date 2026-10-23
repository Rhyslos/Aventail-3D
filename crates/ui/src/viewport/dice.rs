//! The view dice in each view's bottom-left corner ([decision 0084](../../../../docs/decisions/0084-view-dice.md)),
//! in place of the X, Y and Z arrows: a small cube turned as the scene
//! is, its faces tinted in the axis colours. Click a face, an edge or a
//! corner to look from there (26 views, like 3ds Max's view cube); Ctrl +
//! click looks from the opposite side.
//!
//! Each visible face is cut into a 3 × 3 grid: the middle is the face,
//! the strips along its sides are the edges, the squares in its corners
//! are the corners. The dice is drawn flat (no perspective) whatever the
//! view, so its parts keep their shape.

use crate::text::tr;
use crate::theme::{self, tint};
use egui::{Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Shape, Stroke, Ui, vec2};
use glam::{IVec3, Vec3};

/// Where the dice's middle sits, from a view's bottom-left corner.
pub(super) const DICE_AT: egui::Vec2 = vec2(46.0, -48.0);

/// Half the dice's size, in points: a corner is up to √3 times this from
/// the middle.
const HALF: f32 = 17.0;

/// Where a face's middle part ends and its edge strips begin, as a share
/// of half its width: the face gets the biggest target, as on 3ds Max's
/// view cube.
const INNER: f32 = 0.55;

/// The accessible name of the dice.
pub(crate) const DICE_LABEL: &str = crate::text!("View dice");

/// One part of the dice as seen: which side it stands for, and its
/// outline on screen.
#[derive(Debug, Clone)]
struct Part {
    side: IVec3,
    outline: [Pos2; 4],
}

/// The visible faces of a dice around `centre` for `camera`, back to
/// front, each as its outward direction and its four corners on screen.
fn faces(camera: &tools::Camera, centre: Pos2) -> Vec<(IVec3, [Pos2; 4], f32)> {
    let (right, up, forward) = (camera.right(), camera.up(), camera.forward());
    let screen = |p: Vec3| centre + vec2(p.dot(right), -p.dot(up)) * HALF;
    let mut out = Vec::new();
    for side in tools::dice::sides().into_iter().take(6) {
        let n = side.as_vec3();
        let facing = -n.dot(forward);
        if facing <= 0.05 {
            continue;
        }
        let (u, v) = across(side);
        let corners = [n - u - v, n + u - v, n + u + v, n - u + v].map(screen);
        out.push((side, corners, facing));
    }
    out.sort_by(|a, b| a.2.total_cmp(&b.2));
    out
}

/// Two directions across a face, as unit vectors along the other two
/// axes.
fn across(face: IVec3) -> (Vec3, Vec3) {
    if face.x != 0 {
        (Vec3::Y, Vec3::Z)
    } else if face.y != 0 {
        (Vec3::X, Vec3::Z)
    } else {
        (Vec3::X, Vec3::Y)
    }
}

/// Every part of the visible faces: on each face its middle, four edge
/// strips and four corners, with the side each stands for. Front faces
/// last, so the first part (from the end) containing a point is the one
/// seen there.
fn parts(camera: &tools::Camera, centre: Pos2) -> Vec<Part> {
    let (right, up) = (camera.right(), camera.up());
    let screen = |p: Vec3| centre + vec2(p.dot(right), -p.dot(up)) * HALF;
    let bands = [(-1.0, -INNER, -1), (-INNER, INNER, 0), (INNER, 1.0, 1)];
    let mut out = Vec::new();
    for (face, _, _) in faces(camera, centre) {
        let n = face.as_vec3();
        let (u, v) = across(face);
        for (u0, u1, i) in bands {
            for (v0, v1, j) in bands {
                let point = |a: f32, b: f32| screen(n + u * a + v * b);
                let side = face + (u * i as f32 + v * j as f32).as_ivec3();
                out.push(Part {
                    side,
                    outline: [point(u0, v0), point(u1, v0), point(u1, v1), point(u0, v1)],
                });
            }
        }
    }
    out.extend(arrows(camera, centre));
    out
}

/// Looking straight at a face, the four faces around it can't be seen:
/// a small arrow beside each side of the dice turns the view to the face
/// beyond it (as 3ds Max's view cube does).
fn arrows(camera: &tools::Camera, centre: Pos2) -> Vec<Part> {
    let (right, up, forward) = (camera.right(), camera.up(), camera.forward());
    let Some(face) = tools::dice::sides()
        .into_iter()
        .take(6)
        .find(|f| -f.as_vec3().dot(forward) > ARROWS_WHEN)
    else {
        return Vec::new();
    };
    let (u, v) = across(face);
    [u, -u, v, -v]
        .into_iter()
        .map(|way| {
            let dir = vec2(way.dot(right), -way.dot(up)).normalized();
            let side_on = vec2(-dir.y, dir.x);
            let tip = centre + dir * HALF * 1.75;
            let base = centre + dir * HALF * 1.25;
            Part {
                side: way.as_ivec3(),
                outline: [tip, base + side_on * 5.0, base, base - side_on * 5.0],
            }
        })
        .collect()
}

/// How squarely a face must be seen for the arrows to the faces around it
/// to show.
const ARROWS_WHEN: f32 = 0.995;

/// Whether `p` is inside the four-cornered `outline` (convex, either
/// winding).
fn inside(outline: &[Pos2; 4], p: Pos2) -> bool {
    let mut sign = 0.0_f32;
    for k in 0..4 {
        let (a, b) = (outline[k], outline[(k + 1) % 4]);
        let cross = (b - a).x * (p - a).y - (b - a).y * (p - a).x;
        if cross.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    true
}

/// The side of the dice under `p`, if any.
fn side_at(camera: &tools::Camera, centre: Pos2, p: Pos2) -> Option<IVec3> {
    parts(camera, centre)
        .into_iter()
        .rev()
        .find(|part| inside(&part.outline, p))
        .map(|part| part.side)
}

/// Where on screen to click to look from `side`, if it can be seen: the
/// middle of its part on the frontmost face showing it. For tests.
#[cfg(test)]
pub(crate) fn point_for(camera: &tools::Camera, view: Rect, side: IVec3) -> Option<Pos2> {
    let centre = view.left_bottom() + DICE_AT;
    parts(camera, centre)
        .into_iter()
        .rev()
        .find(|part| part.side == side)
        .map(|part| {
            let sum = part
                .outline
                .iter()
                .fold(vec2(0.0, 0.0), |s, p| s + p.to_vec2());
            (sum / 4.0).to_pos2()
        })
}

/// A face's tint: its axis colour, quietly.
fn tint_of(face: IVec3) -> Color32 {
    let axis = if face.x != 0 {
        tint::AXIS_X
    } else if face.y != 0 {
        tint::AXIS_Y
    } else {
        tint::AXIS_Z
    };
    let base = Color32::from_rgb(0x34, 0x34, 0x34);
    lerp(base, axis, 0.22)
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

/// Draws the dice in the bottom-left corner of the view at `rect` and
/// handles clicks on it: returns the side to look from (the opposite one
/// with Ctrl held), if one was clicked.
pub(super) fn show(ui: &Ui, rect: Rect, slot: usize, camera: &tools::Camera) -> Option<IVec3> {
    let centre = rect.left_bottom() + DICE_AT;
    let reach = HALF * 1.8;
    let area = Rect::from_center_size(centre, vec2(reach, reach) * 2.0);
    let response = ui.interact(area, ui.id().with(("view_dice", slot)), Sense::click());
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tr(DICE_LABEL)));
    let hovered = response
        .hover_pos()
        .and_then(|p| side_at(camera, centre, p));
    let painter = ui.painter_at(rect);
    // The faces, back to front, each with its middle and edges marked.
    for (face, corners, facing) in faces(camera, centre) {
        painter.add(Shape::convex_polygon(
            corners.to_vec(),
            tint_of(face),
            Stroke::new(1.0, Color32::from_rgb(0x1a, 0x1a, 0x1a)),
        ));
        if let Some(label) = tools::dice::face_label(face)
            && facing > 0.45
        {
            let at = corners.iter().fold(vec2(0.0, 0.0), |s, p| s + p.to_vec2()) / 4.0;
            painter.text(
                at.to_pos2(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(8.5),
                theme::TEXT_STRONG.gamma_multiply(facing.clamp(0.5, 1.0)),
            );
        }
    }
    // Looking straight at a face: arrows to the faces around it.
    for arrow in arrows(camera, centre) {
        let lit = hovered == Some(arrow.side);
        let [tip, left, _, right] = arrow.outline;
        painter.add(Shape::convex_polygon(
            vec![tip, left, right],
            if lit {
                theme::TEXT_STRONG
            } else {
                theme::TEXT_WEAK.gamma_multiply(0.8)
            },
            Stroke::NONE,
        ));
    }
    // The part under the mouse lights up, on every face it shows on.
    if let Some(side) = hovered {
        let accent = theme::accent(ui.ctx()).colors();
        let on_faces = parts(camera, centre).len() - arrows(camera, centre).len();
        for part in parts(camera, centre)
            .into_iter()
            .take(on_faces)
            .filter(|p| p.side == side)
        {
            painter.add(Shape::convex_polygon(
                part.outline.to_vec(),
                accent.fill.gamma_multiply(0.85),
                Stroke::new(1.0, accent.light),
            ));
        }
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let opposite = ui.input(|i| i.modifiers.command);
    let response = match hovered {
        Some(side) => response.on_hover_text(crate::trf!(
            "Look from the {} (Ctrl + click: from the {})",
            tools::dice::name(side),
            tools::dice::name(-side)
        )),
        None => response,
    };
    if response.clicked() {
        let side = response
            .interact_pointer_pos()
            .and_then(|p| side_at(camera, centre, p))?;
        return Some(if opposite { -side } else { side });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera_from(side: IVec3) -> tools::Camera {
        *tools::Navigator::default().looking_from(side).camera()
    }

    #[test]
    fn looking_from_a_corner_shows_three_faces_and_all_their_parts() {
        let camera = camera_from(IVec3::new(1, -1, 1));
        let centre = Pos2::new(100.0, 100.0);
        let seen: Vec<IVec3> = faces(&camera, centre).iter().map(|f| f.0).collect();
        assert_eq!(seen.len(), 3);
        for face in [IVec3::X, IVec3::NEG_Y, IVec3::Z] {
            assert!(seen.contains(&face), "{face}");
        }
        // The middle of the dice is the corner looked from.
        assert_eq!(side_at(&camera, centre, centre), Some(IVec3::new(1, -1, 1)));
        // Nine parts on each of the three faces, and no arrows.
        let parts = parts(&camera, centre);
        assert_eq!(parts.len(), 27);
    }

    #[test]
    fn every_part_seen_can_be_clicked_and_is_the_side_it_shows() {
        for from in tools::dice::sides() {
            let camera = camera_from(from);
            let view = Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0));
            let centre = view.left_bottom() + DICE_AT;
            for part in parts(&camera, centre) {
                if let Some(at) = point_for(&camera, view, part.side) {
                    assert_eq!(side_at(&camera, centre, at), Some(part.side));
                }
            }
            // The side looked from is always there to click, in the middle.
            assert_eq!(side_at(&camera, centre, centre), Some(from), "{from}");
        }
    }

    #[test]
    fn straight_on_a_face_shows_only_it() {
        let camera = camera_from(IVec3::NEG_Y);
        let centre = Pos2::new(50.0, 50.0);
        let seen: Vec<IVec3> = faces(&camera, centre).iter().map(|f| f.0).collect();
        assert_eq!(seen, vec![IVec3::NEG_Y]);
        // Its edges and corners are still there, around it.
        let right_edge = centre + vec2(HALF * 0.8, 0.0);
        assert_eq!(
            side_at(&camera, centre, right_edge),
            Some(IVec3::new(1, -1, 0))
        );
        let top_right = centre + vec2(HALF * 0.8, -HALF * 0.8);
        assert_eq!(
            side_at(&camera, centre, top_right),
            Some(IVec3::new(1, -1, 1))
        );
        // Beside the dice, the arrow to the face round to the right.
        assert_eq!(
            side_at(&camera, centre, centre + vec2(HALF * 1.6, 0.0)),
            Some(IVec3::X)
        );
        // Further out: nothing.
        assert_eq!(
            side_at(&camera, centre, centre + vec2(HALF * 2.5, 0.0)),
            None
        );
        assert_eq!(
            side_at(&camera, centre, centre + vec2(HALF * 1.6, HALF * 1.6)),
            None
        );
    }

    #[test]
    fn points_inside_a_four_cornered_outline() {
        let square = [
            Pos2::new(0.0, 0.0),
            Pos2::new(1.0, 0.0),
            Pos2::new(1.0, 1.0),
            Pos2::new(0.0, 1.0),
        ];
        assert!(inside(&square, Pos2::new(0.5, 0.5)));
        assert!(!inside(&square, Pos2::new(1.5, 0.5)));
        let reversed = [square[3], square[2], square[1], square[0]];
        assert!(inside(&reversed, Pos2::new(0.2, 0.9)));
    }
}
