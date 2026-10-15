//! Tests for the gizmo: its handles, grabbing them, and what dragging
//! them does.

use super::*;
use crate::navigation::{Navigator, ViewKind};
use glam::Vec3;

const SIZE: Vec2 = Vec2::new(1000.0, 600.0);
const WORLD: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];
const NONE_LOCKED: [bool; 3] = [false; 3];

/// Looking at the origin from the front, right and above.
fn camera() -> Camera {
    Camera::looking_at(Vec3::new(300.0, -500.0, 250.0), Vec3::ZERO)
}

/// Looking straight down at the origin (the top view): X right, Y up the
/// screen, Z towards the viewer.
fn top() -> Camera {
    *Navigator::for_view(ViewKind::Top).camera()
}

fn gizmo(tool: GizmoTool) -> Gizmo {
    Gizmo {
        origin: Vec3::ZERO,
        axes: WORLD,
        tool,
    }
}

fn screen(camera: &Camera, p: Vec3) -> Vec2 {
    camera.project(p, SIZE).unwrap()
}

fn handles(g: &Gizmo, camera: &Camera, locked: [bool; 3]) -> Vec<Handle> {
    g.shapes(camera, SIZE, locked)
        .into_iter()
        .map(|(h, _)| h)
        .collect()
}

#[test]
fn each_tool_has_its_handles() {
    let camera = camera();
    let moving = handles(&gizmo(GizmoTool::Move), &camera, NONE_LOCKED);
    for h in [
        Handle::Axis(0),
        Handle::Axis(1),
        Handle::Axis(2),
        Handle::Plane(0),
        Handle::Plane(1),
        Handle::Plane(2),
        Handle::Centre,
    ] {
        assert!(moving.contains(&h), "{h:?}");
    }
    assert!(!moving.contains(&Handle::ViewRing));

    let turning = handles(&gizmo(GizmoTool::Rotate), &camera, NONE_LOCKED);
    assert!(turning.contains(&Handle::Ring(2)) && turning.contains(&Handle::ViewRing));
    assert!(!turning.contains(&Handle::Centre) && !turning.contains(&Handle::Axis(0)));

    let scaling = handles(&gizmo(GizmoTool::Scale), &camera, NONE_LOCKED);
    assert!(scaling.contains(&Handle::ScaleAxis(1)) && scaling.contains(&Handle::ScaleAll));

    let all = handles(&gizmo(GizmoTool::All), &camera, NONE_LOCKED);
    for h in [
        Handle::Axis(0),
        Handle::Ring(0),
        Handle::ScaleAxis(0),
        Handle::Centre,
    ] {
        assert!(all.contains(&h), "{h:?}");
    }
}

#[test]
fn locked_axes_lose_their_handles() {
    let camera = camera();
    let locked_x = [true, false, false];
    let moving = handles(&gizmo(GizmoTool::All), &camera, locked_x);
    assert!(!moving.contains(&Handle::Axis(0)));
    assert!(!moving.contains(&Handle::Ring(0)));
    assert!(!moving.contains(&Handle::Plane(1)), "the XZ plane needs X");
    assert!(moving.contains(&Handle::Plane(0)), "the YZ plane doesn't");
    assert!(handles(&gizmo(GizmoTool::Move), &camera, [true; 3]).is_empty());
}

#[test]
fn an_arrow_pointing_at_the_viewer_is_left_out() {
    let top = top();
    let moving = handles(&gizmo(GizmoTool::Move), &top, NONE_LOCKED);
    assert!(
        !moving.contains(&Handle::Axis(2)),
        "Z points straight up at us"
    );
    assert!(moving.contains(&Handle::Axis(0)));
    assert!(
        moving.contains(&Handle::Plane(2)),
        "the ground plane faces us"
    );
    assert!(!moving.contains(&Handle::Plane(0)), "seen edge-on");
}

#[test]
fn the_handle_under_the_mouse_is_found() {
    let camera = camera();
    let g = gizmo(GizmoTool::Move);
    let length = g.length(&camera, SIZE);
    let near_tip = screen(&camera, Vec3::X * length * 0.9);
    assert_eq!(
        g.handle_at(&camera, SIZE, near_tip, NONE_LOCKED),
        Some(Handle::Axis(0))
    );
    assert_eq!(
        g.handle_at(&camera, SIZE, screen(&camera, Vec3::ZERO), NONE_LOCKED),
        Some(Handle::Centre)
    );
    let plane_middle = screen(&camera, (Vec3::X + Vec3::Y) * length * 0.32);
    assert_eq!(
        g.handle_at(&camera, SIZE, plane_middle, NONE_LOCKED),
        Some(Handle::Plane(2))
    );
    assert_eq!(
        g.handle_at(&camera, SIZE, Vec2::new(5.0, 5.0), NONE_LOCKED),
        None
    );
    // Locked: not grabbable.
    assert_ne!(
        g.handle_at(&camera, SIZE, near_tip, [true, false, false]),
        Some(Handle::Axis(0))
    );
}

#[test]
fn an_arrow_moves_along_its_axis_only() {
    let camera = camera();
    let g = gizmo(GizmoTool::Move);
    let start = screen(&camera, Vec3::X * 50.0);
    let mut drag = Drag::new(g, Handle::Axis(0), &camera, SIZE, start);
    // Move the mouse to where X = 80 appears, plus some sideways wobble.
    let to = screen(&camera, Vec3::X * 80.0) + Vec2::new(0.0, 15.0);
    let Change::Translate(moved) = drag.update(to, false, NONE_LOCKED) else {
        panic!("a move")
    };
    assert!(moved.y.abs() < 1e-3 && moved.z.abs() < 1e-3, "{moved}");
    assert!(
        (moved.x - 30.0).abs() < 5.0,
        "about 30, the wobble aside: {moved}"
    );
}

#[test]
fn a_plane_square_moves_in_its_plane() {
    let top = top();
    let g = gizmo(GizmoTool::Move);
    let start = screen(&top, Vec3::new(20.0, 20.0, 0.0));
    let mut drag = Drag::new(g, Handle::Plane(2), &top, SIZE, start);
    let to = screen(&top, Vec3::new(60.0, -10.0, 0.0));
    let Change::Translate(moved) = drag.update(to, false, NONE_LOCKED) else {
        panic!("a move")
    };
    assert!(
        moved.abs_diff_eq(Vec3::new(40.0, -30.0, 0.0), 0.5),
        "{moved}"
    );
}

#[test]
fn fine_steps_go_ten_times_slower_and_locks_hold() {
    let top = top();
    let g = gizmo(GizmoTool::Move);
    let start = screen(&top, Vec3::ZERO);
    let mut drag = Drag::new(g, Handle::Centre, &top, SIZE, start);
    let to = screen(&top, Vec3::new(100.0, 50.0, 0.0));
    let Change::Translate(fine) = drag.update(to, true, NONE_LOCKED) else {
        panic!()
    };
    assert!(fine.abs_diff_eq(Vec3::new(10.0, 5.0, 0.0), 0.2), "{fine}");
    // Locking X keeps only the Y part of the move.
    let Change::Translate(locked) = drag.change([true, false, false]) else {
        panic!()
    };
    assert!(
        locked.abs_diff_eq(Vec3::new(0.0, 5.0, 0.0), 0.2),
        "{locked}"
    );
}

#[test]
fn a_ring_turns_the_way_the_mouse_goes_round() {
    let top = top();
    let g = gizmo(GizmoTool::Rotate);
    let length = g.length(&top, SIZE);
    // From the +X side of the Z ring to the +Y side: a quarter turn
    // anticlockwise seen from above, which is +90° around Z.
    let at = |p: Vec3| screen(&top, p * length * RING);
    let mut drag = Drag::new(g, Handle::Ring(2), &top, SIZE, at(Vec3::X));
    drag.update(at(Vec3::new(1.0, 1.0, 0.0).normalize()), false, NONE_LOCKED);
    let Change::Rotate { axis, angle } = drag.update(at(Vec3::Y), false, NONE_LOCKED) else {
        panic!("a turn")
    };
    assert_eq!(axis, Vec3::Z);
    assert!(
        (angle - std::f32::consts::FRAC_PI_2).abs() < 0.02,
        "{angle}"
    );
    // The view ring turns around the line of sight, the same way.
    let mut drag = Drag::new(g, Handle::ViewRing, &top, SIZE, at(Vec3::X));
    let Change::Rotate { axis, angle } = drag.update(at(Vec3::Y), false, NONE_LOCKED) else {
        panic!()
    };
    assert!(axis.abs_diff_eq(Vec3::Z, 1e-3));
    assert!(angle > 1.0);
}

#[test]
fn scale_handles_scale_along_their_axis_or_all() {
    let top = top();
    let g = gizmo(GizmoTool::Scale);
    let length = g.length(&top, SIZE);
    let start = screen(&top, Vec3::X * length * SCALE_END);
    let mut drag = Drag::new(g, Handle::ScaleAxis(0), &top, SIZE, start);
    let to = screen(&top, Vec3::X * length * (SCALE_END + 1.0));
    let Change::Scale(f) = drag.update(to, false, NONE_LOCKED) else {
        panic!("a scale")
    };
    assert!((f.x - 2.0).abs() < 0.02 && f.y == 1.0 && f.z == 1.0, "{f}");

    let centre = screen(&top, Vec3::ZERO);
    let mut drag = Drag::new(g, Handle::ScaleAll, &top, SIZE, centre);
    let Change::Scale(f) = drag.update(centre + Vec2::new(GIZMO_SIZE, 0.0), false, NONE_LOCKED)
    else {
        panic!()
    };
    assert!(f.abs_diff_eq(Vec3::splat(2.0), 1e-3), "{f}");
    let Change::Scale(f) = drag.change([false, false, true]) else {
        panic!()
    };
    assert_eq!(f.z, 1.0, "a locked axis keeps its size");
}

#[test]
fn snapping_and_constraining_helpers() {
    assert_eq!(snap(26.0, 10.0), 30.0);
    assert_eq!(snap(-24.0, 10.0), -20.0);
    assert_eq!(snap(26.0, 0.0), 26.0, "no step: unchanged");
    assert_eq!(snap(26.0, f32::NAN), 26.0);
    let target = Vec3::new(5.0, 7.0, 9.0);
    assert_eq!(
        nearest_on_axis(Vec3::ZERO, Vec3::X, target),
        Vec3::new(5.0, 0.0, 0.0)
    );
    assert_eq!(
        into_plane(Vec3::ZERO, Vec3::Z, target),
        Vec3::new(5.0, 7.0, 0.0)
    );
}

#[test]
fn the_gizmo_keeps_its_size_on_screen() {
    let near = camera();
    let far = Camera {
        position: near.position * 3.0,
        ..near
    };
    let g = gizmo(GizmoTool::Move);
    assert!(g.length(&far, SIZE) > 2.5 * g.length(&near, SIZE));
    let top = top();
    let shown = top.ortho_height.unwrap();
    assert!((top.world_per_pixel(Vec3::ZERO, SIZE) - shown / SIZE.y).abs() < 1e-4);
    // Behind the camera: nothing to draw or grab.
    let behind = Gizmo {
        origin: near.position + near.forward() * -100.0,
        ..g
    };
    assert!(behind.shapes(&near, SIZE, NONE_LOCKED).is_empty());
}

#[test]
fn shapes_measure_distance_from_the_mouse() {
    let line = Shape::Line {
        a: Vec2::ZERO,
        b: Vec2::new(10.0, 0.0),
        arrow: true,
    };
    assert_eq!(line.distance(Vec2::new(5.0, 3.0)), 3.0);
    let quad = Shape::Quad(square(Vec2::ZERO, 5.0));
    assert_eq!(quad.distance(Vec2::ZERO), 0.0);
    assert_eq!(quad.distance(Vec2::new(8.0, 0.0)), 3.0);
    let dot = Shape::Dot {
        centre: Vec2::ZERO,
        radius: 4.0,
    };
    assert_eq!(dot.distance(Vec2::new(6.0, 0.0)), 2.0);
    assert_eq!(Handle::Ring(1).axis(), Some(1));
    assert_eq!(Handle::Centre.axis(), None);
}

#[test]
fn a_scale_square_scales_its_two_axes_equally() {
    let top = top();
    let g = gizmo(GizmoTool::Scale);
    let scaling = handles(&g, &top, NONE_LOCKED);
    assert!(
        scaling.contains(&Handle::ScalePlane(2)),
        "the ground square"
    );
    assert!(!scaling.contains(&Handle::Plane(2)), "no move squares");
    let length = g.length(&top, SIZE);
    let start = Vec3::new(0.3, 0.3, 0.0) * length;
    let mut drag = Drag::new(g, Handle::ScalePlane(2), &top, SIZE, screen(&top, start));
    // Mostly along X: both X and Y grow, equally; Z not at all.
    let to = start + Vec3::new(1.0, 0.2, 0.0) * length;
    let Change::Scale(f) = drag.update(screen(&top, to), false, NONE_LOCKED) else {
        panic!("a scale")
    };
    assert!((f.x - 1.6).abs() < 0.02, "{f}");
    assert!((f.x - f.y).abs() < 1e-4, "{f}");
    assert_eq!(f.z, 1.0);
    // Diagonally: the same.
    let to = start + Vec3::new(0.5, 0.5, 0.0) * length;
    let Change::Scale(f) = drag.update(screen(&top, to), false, NONE_LOCKED) else {
        panic!("a scale")
    };
    assert!((f.x - f.y).abs() < 1e-4 && (f.x - 1.5).abs() < 0.02, "{f}");
    assert_eq!(f.z, 1.0);
    // Back towards the centre: both shrink.
    let to = start - Vec3::new(0.2, 0.0, 0.0) * length;
    let Change::Scale(f) = drag.update(screen(&top, to), false, NONE_LOCKED) else {
        panic!("a scale")
    };
    assert!(f.x < 1.0 && (f.x - f.y).abs() < 1e-4, "{f}");
    // A locked axis keeps its size.
    let Change::Scale(f) = drag.change([true, false, false]) else {
        panic!()
    };
    assert_eq!(f.x, 1.0);
    // With all three gizmos (T), the squares move instead.
    let all = handles(&gizmo(GizmoTool::All), &top, NONE_LOCKED);
    assert!(all.contains(&Handle::Plane(2)) && !all.contains(&Handle::ScalePlane(2)));
    assert_eq!(Handle::ScalePlane(1).axis(), Some(1));
}
