//! The live mirrors' planes in each view ([decision 0088](../../../../docs/decisions/0088-live-mirror.md)):
//! a faint, see-through red sheet where the mirror stands, and a clearer
//! red line where it meets the ground. Seen edge-on (a mirror across X in
//! the Front or Top view), the sheet is just a line.

use crate::drawing::MirrorPlane;
use egui::{Color32, Pos2, Rect, Shape, Stroke, Ui};
use glam::Vec2;
use glam::Vec3;
use tools::Camera;

/// The sheet's colour: the X axis's red, very faint.
const SHEET: Color32 = Color32::from_rgba_premultiplied(16, 4, 4, 16);
/// Its edge and the line on the ground: the same red, clearer.
const LINE: Color32 = Color32::from_rgba_premultiplied(150, 45, 40, 170);

/// How many pieces the sheet is drawn in each way, so a part behind the
/// camera leaves out only the pieces there.
const PIECES: usize = 8;

/// Draws `planes` over the view in `rect`, seen by `camera`.
pub(super) fn paint(ui: &Ui, rect: Rect, camera: &Camera, planes: &[MirrorPlane]) {
    if planes.is_empty() {
        return;
    }
    let painter = ui.painter_at(rect);
    let size = Vec2::new(rect.width(), rect.height());
    let project = |p: Vec3| {
        camera
            .project(p, size)
            .filter(|at| at.is_finite())
            .map(|at| rect.min + egui::vec2(at.x, at.y))
    };
    for plane in planes {
        let [a, b, _, d] = plane.corners();
        let (along_u, along_v) = (b - a, d - a);
        let point = |i: usize, j: usize| {
            a + along_u * (i as f32 / PIECES as f32) + along_v * (j as f32 / PIECES as f32)
        };
        // The sheet, piece by piece.
        for i in 0..PIECES {
            for j in 0..PIECES {
                let corners = [
                    point(i, j),
                    point(i + 1, j),
                    point(i + 1, j + 1),
                    point(i, j + 1),
                ];
                let projected: Option<Vec<Pos2>> = corners.iter().map(|c| project(*c)).collect();
                if let Some(points) = projected {
                    painter.add(Shape::convex_polygon(points, SHEET, Stroke::NONE));
                }
            }
        }
        // Its edge.
        let edge: Vec<Option<Pos2>> = plane.corners().iter().map(|c| project(*c)).collect();
        for k in 0..4 {
            if let (Some(p), Some(q)) = (edge[k], edge[(k + 1) % 4]) {
                painter.line_segment([p, q], Stroke::new(1.0, LINE.gamma_multiply(0.5)));
            }
        }
        // Where it meets the ground: a mirror across X or Y stands
        // upright, so it crosses the ground in a line.
        if plane.axis < 2 {
            let other = 1 - plane.axis;
            let [p, q] = ground_line(plane, other);
            segment(&painter, &project, p, q);
        }
    }
}

/// The two ends of the line where `plane` (upright) meets the ground,
/// spanning the sheet along the `other` axis.
fn ground_line(plane: &MirrorPlane, other: usize) -> [Vec3; 2] {
    let corners = plane.corners();
    let (low, high) = corners.iter().fold((f32::MAX, f32::MIN), |(lo, hi), c| {
        (lo.min(c[other]), hi.max(c[other]))
    });
    let end = |value: f32| {
        let mut p = Vec3::ZERO;
        p[plane.axis] = plane.at;
        p[other] = value;
        p
    };
    [end(low), end(high)]
}

/// Draws the line from `p` to `q`, in pieces so that a part behind the
/// camera leaves out only that part.
fn segment(painter: &egui::Painter, project: &impl Fn(Vec3) -> Option<Pos2>, p: Vec3, q: Vec3) {
    let steps = PIECES * 2;
    for k in 0..steps {
        let a = p.lerp(q, k as f32 / steps as f32);
        let b = p.lerp(q, (k + 1) as f32 / steps as f32);
        if let (Some(a), Some(b)) = (project(a), project(b)) {
            painter.line_segment([a, b], Stroke::new(2.0, LINE));
        }
    }
}

/// The screen points of `plane`'s ground line in a view of `size`, for
/// tests.
#[cfg(test)]
pub(super) fn ground_on_screen(
    camera: &Camera,
    size: Vec2,
    plane: &MirrorPlane,
) -> Option<[Pos2; 2]> {
    let [p, q] = ground_line(plane, 1 - plane.axis);
    let a = camera.project(p, size)?;
    let b = camera.project(q, size)?;
    Some([egui::pos2(a.x, a.y), egui::pos2(b.x, b.y)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::Bounds;

    fn plane() -> MirrorPlane {
        MirrorPlane {
            axis: 0,
            at: 0.0,
            around: Bounds {
                min: Vec3::new(-150.0, -50.0, 0.0),
                max: Vec3::new(150.0, 50.0, 100.0),
            },
        }
    }

    #[test]
    fn the_sheet_stands_on_the_mirror_around_its_objects() {
        let corners = plane().corners();
        assert!(corners.iter().all(|c| c.x == 0.0));
        // At least 2 m across, around the objects' middle.
        let ys: Vec<f32> = corners.iter().map(|c| c.y).collect();
        assert!(ys.iter().cloned().fold(f32::MIN, f32::max) >= 100.0);
        let zs: Vec<f32> = corners.iter().map(|c| c.z).collect();
        assert!(zs.iter().cloned().fold(f32::MAX, f32::min) < 0.0);
        let [p, q] = ground_line(&plane(), 1);
        assert_eq!((p.x, p.z, q.x, q.z), (0.0, 0.0, 0.0, 0.0));
        assert!(p.y < q.y);
    }

    #[test]
    fn the_standard_view_sees_the_ground_line() {
        let camera = Camera::default();
        let size = Vec2::new(800.0, 600.0);
        let [a, b] = ground_on_screen(&camera, size, &plane()).expect("in view");
        assert!(a.x.is_finite() && b.y.is_finite() && a != b);
    }
}
