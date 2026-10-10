//! Moving the viewport camera with the mouse, as in Cinema 4D.
//!
//! | Action | Mouse |
//! |---|---|
//! | Orbit | Alt + left drag |
//! | Pan | Alt + middle drag |
//! | Zoom (dolly) | Alt + right drag, or the mouse wheel |
//!
//! The camera always looks at a **pivot**: the point it orbits around and
//! moves towards when zooming. Panning carries the pivot along. (Picking
//! the pivot from the point under the cursor is the next milestone.)
//!
//! Everything here is plain maths on plain input, so it is tested without
//! a window. Distances on screen are in any one unit (the interface uses
//! logical pixels); the view size passed in must use the same unit.

use crate::camera::{Camera, DEFAULT_TARGET, MAX_PITCH};
use glam::{Vec2, Vec3};

/// How far the view turns per unit of mouse movement while orbiting, in
/// radians (0.3° per pixel: a drag across a typical viewport turns the
/// view about a full half circle).
pub const ORBIT_SPEED: f32 = 0.3 * std::f32::consts::PI / 180.0;
/// How quickly dragging zooms: moving the mouse about 140 pixels halves
/// (or doubles) the distance to the pivot.
pub const DOLLY_DRAG_SPEED: f32 = 0.005;
/// How quickly the mouse wheel zooms, per unit of scrolling. One wheel
/// notch scrolls about 40 to 50 units, so each notch moves roughly 12%
/// of the way to the pivot.
pub const WHEEL_DOLLY_SPEED: f32 = 0.003;
/// Closest the camera may get to the pivot, in centimetres.
pub const MIN_DISTANCE: f32 = 2.0;
/// Farthest the camera may get from the pivot, in centimetres (1 km).
pub const MAX_DISTANCE: f32 = 100_000.0;

/// A mouse button, as far as navigation cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// The left button (or the main button).
    Primary,
    /// The middle button, usually pressing the wheel.
    Middle,
    /// The right button.
    Secondary,
}

/// What a navigation drag does to the camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragKind {
    /// Turn around the pivot.
    Orbit,
    /// Slide sideways and up or down, carrying the pivot along.
    Pan,
    /// Move towards or away from the pivot.
    Dolly,
}

impl DragKind {
    /// Which navigation a mouse drag starts, Cinema 4D style: with Alt
    /// held, left orbits, middle pans and right zooms. Without Alt the drag
    /// belongs to the tools, not the camera.
    pub fn for_drag(button: MouseButton, alt: bool) -> Option<Self> {
        if !alt {
            return None;
        }
        Some(match button {
            MouseButton::Primary => Self::Orbit,
            MouseButton::Middle => Self::Pan,
            MouseButton::Secondary => Self::Dolly,
        })
    }
}

/// The viewport camera together with the pivot it looks at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Navigator {
    camera: Camera,
    pivot: Vec3,
}

impl Default for Navigator {
    /// The opening view, looking at the middle of the default scene.
    fn default() -> Self {
        Self::new(Camera::default(), DEFAULT_TARGET)
    }
}

impl Navigator {
    /// A navigator that orbits `pivot`, with the camera turned to look at
    /// it from where it stands (keeping its field of view). If the camera
    /// stands on the pivot, the pivot is moved in front of it instead.
    pub fn new(camera: Camera, pivot: Vec3) -> Self {
        let distance = camera.position.distance(pivot);
        if !(distance.is_finite() && distance >= MIN_DISTANCE) {
            let pivot = camera.position + camera.forward() * MIN_DISTANCE;
            return Self { camera, pivot };
        }
        let mut aimed = Camera::looking_at(camera.position, pivot);
        aimed.fov_y = camera.fov_y;
        let mut navigator = Self {
            camera: aimed,
            pivot,
        };
        // Looking straight up or down is limited, so stand where the
        // limited view really looks at the pivot.
        navigator.place_camera(distance);
        navigator
    }

    /// The camera, for drawing.
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// The point the camera looks at, orbits and zooms towards.
    pub fn pivot(&self) -> Vec3 {
        self.pivot
    }

    /// How far the camera is from the pivot, in centimetres.
    pub fn distance(&self) -> f32 {
        self.camera.position.distance(self.pivot)
    }

    /// Goes back to the opening view.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Applies a navigation drag: `delta` is how far the mouse moved since
    /// the last update (right and down are positive), `view_size` the size
    /// of the viewport, both in the same unit.
    pub fn drag(&mut self, kind: DragKind, delta: Vec2, view_size: Vec2) {
        if !delta.is_finite() {
            return;
        }
        match kind {
            DragKind::Orbit => self.orbit(delta),
            DragKind::Pan => self.pan(delta, view_size.y),
            DragKind::Dolly => self.dolly((delta.x - delta.y) * DOLLY_DRAG_SPEED),
        }
    }

    /// Zooms for a turn of the mouse wheel: positive `scroll` (wheel turned
    /// away from you) moves closer.
    pub fn wheel(&mut self, scroll: f32) {
        self.dolly(scroll * WHEEL_DOLLY_SPEED);
    }

    /// Turns the camera around the pivot. The scene follows the mouse:
    /// dragging right turns its front to the right, dragging down tips its
    /// top towards you.
    pub fn orbit(&mut self, delta: Vec2) {
        if !delta.is_finite() {
            return;
        }
        let distance = self.distance();
        self.camera.yaw = wrap_angle(self.camera.yaw - delta.x * ORBIT_SPEED);
        self.camera.pitch =
            (self.camera.pitch - delta.y * ORBIT_SPEED).clamp(-MAX_PITCH, MAX_PITCH);
        self.place_camera(distance);
    }

    /// Slides the camera and the pivot together so that points at the
    /// pivot's depth stay exactly under the mouse. `view_height` is the
    /// viewport's height in the same unit as `delta`.
    pub fn pan(&mut self, delta: Vec2, view_height: f32) {
        if !(delta.is_finite() && view_height.is_finite() && view_height > 0.0) {
            return;
        }
        // Height of the visible world at the pivot's depth, per unit.
        let per_unit = 2.0 * self.distance() * (self.camera.fov_y * 0.5).tan() / view_height;
        let shift = (-self.camera.right() * delta.x + self.camera.up() * delta.y) * per_unit;
        self.camera.position += shift;
        self.pivot += shift;
    }

    /// Moves towards (positive `amount`) or away from the pivot. The
    /// distance changes by a factor of e^amount, so each step feels the
    /// same whether near or far.
    pub fn dolly(&mut self, amount: f32) {
        if !amount.is_finite() {
            return;
        }
        let distance = (self.distance() * (-amount).exp()).clamp(MIN_DISTANCE, MAX_DISTANCE);
        self.place_camera(distance);
    }

    /// Puts the camera `distance` from the pivot, looking at it.
    fn place_camera(&mut self, distance: f32) {
        self.camera.position = self.pivot - self.camera.forward() * distance;
    }
}

/// An angle brought into -π to π, so it never grows without end.
fn wrap_angle(angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    (angle + PI).rem_euclid(TAU) - PI
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Vec2 = Vec2::new(1000.0, 600.0);

    fn on_screen(navigator: &Navigator, point: Vec3) -> Vec2 {
        navigator.camera().project(point, SIZE).unwrap()
    }

    fn centred(navigator: &Navigator) -> bool {
        (on_screen(navigator, navigator.pivot()) - SIZE / 2.0).length() < 0.05
    }

    #[test]
    fn alt_and_a_button_pick_the_navigation() {
        use DragKind::*;
        use MouseButton::*;
        assert_eq!(DragKind::for_drag(Primary, true), Some(Orbit));
        assert_eq!(DragKind::for_drag(Middle, true), Some(Pan));
        assert_eq!(DragKind::for_drag(Secondary, true), Some(Dolly));
        for button in [Primary, Middle, Secondary] {
            assert_eq!(DragKind::for_drag(button, false), None, "{button:?}");
        }
    }

    #[test]
    fn the_opening_view_looks_at_the_pivot() {
        let navigator = Navigator::default();
        assert_eq!(navigator.pivot(), DEFAULT_TARGET);
        assert!(centred(&navigator));
        assert!((navigator.camera().position - Camera::default().position).length() < 0.01);
    }

    #[test]
    fn orbiting_keeps_the_distance_and_the_pivot_centred() {
        let mut navigator = Navigator::default();
        let distance = navigator.distance();
        for delta in [
            Vec2::new(120.0, 0.0),
            Vec2::new(-40.0, 75.0),
            Vec2::new(3.0, -200.0),
        ] {
            navigator.orbit(delta);
            assert!((navigator.distance() - distance).abs() < 0.01);
            assert!(centred(&navigator));
        }
    }

    #[test]
    fn orbiting_right_turns_the_front_of_the_scene_to_the_right() {
        let mut navigator = Navigator::default();
        // A point between the camera and the pivot: the side facing you.
        let front = navigator.pivot() + (navigator.camera().position - navigator.pivot()) * 0.5;
        let before = on_screen(&navigator, front);
        navigator.orbit(Vec2::new(30.0, 0.0));
        assert!(on_screen(&navigator, front).x > before.x + 1.0);
    }

    #[test]
    fn orbiting_down_shows_more_of_the_top() {
        let mut navigator = Navigator::default();
        let height = navigator.camera().position.z;
        navigator.orbit(Vec2::new(0.0, 40.0));
        assert!(navigator.camera().position.z > height);
        assert!(navigator.camera().pitch < Camera::default().pitch);
    }

    #[test]
    fn orbiting_never_flips_over_the_top() {
        let mut navigator = Navigator::default();
        for _ in 0..50 {
            navigator.orbit(Vec2::new(17.0, 400.0));
            assert!(navigator.camera().up().z > 0.0, "screen-up still points up");
            assert!(navigator.camera().pitch >= -MAX_PITCH);
            assert!(centred(&navigator));
        }
        for _ in 0..50 {
            navigator.orbit(Vec2::new(0.0, -400.0));
            assert!(navigator.camera().pitch <= MAX_PITCH);
            assert!(navigator.camera().position.is_finite());
        }
    }

    #[test]
    fn many_turns_keep_the_angle_small() {
        let mut navigator = Navigator::default();
        for _ in 0..10_000 {
            navigator.orbit(Vec2::new(500.0, 0.0));
        }
        assert!(navigator.camera().yaw.abs() <= std::f32::consts::PI + 1e-4);
        assert!(centred(&navigator));
    }

    #[test]
    fn panning_keeps_the_point_under_the_mouse() {
        let mut navigator = Navigator::default();
        let grabbed = navigator.pivot();
        let start = on_screen(&navigator, grabbed);
        let delta = Vec2::new(80.0, -35.0);
        navigator.pan(delta, SIZE.y);
        let now = on_screen(&navigator, grabbed);
        assert!(
            (now - (start + delta)).length() < 0.05,
            "{now} vs {}",
            start + delta
        );
        assert!(centred(&navigator), "the pivot travels with the view");
    }

    #[test]
    fn panning_does_not_turn_or_zoom() {
        let mut navigator = Navigator::default();
        let (yaw, pitch, distance) = (
            navigator.camera().yaw,
            navigator.camera().pitch,
            navigator.distance(),
        );
        navigator.pan(Vec2::new(-300.0, 140.0), SIZE.y);
        assert_eq!(navigator.camera().yaw, yaw);
        assert_eq!(navigator.camera().pitch, pitch);
        assert!((navigator.distance() - distance).abs() < 0.01);
    }

    #[test]
    fn zooming_moves_towards_the_pivot_and_back() {
        let mut navigator = Navigator::default();
        let distance = navigator.distance();
        navigator.dolly(0.5);
        assert!(navigator.distance() < distance);
        assert!(centred(&navigator));
        navigator.dolly(-0.5);
        assert!((navigator.distance() - distance).abs() < 0.05);
    }

    #[test]
    fn zooming_stops_at_the_limits() {
        let mut navigator = Navigator::default();
        navigator.dolly(1000.0);
        assert!((navigator.distance() - MIN_DISTANCE).abs() < 1e-3);
        navigator.dolly(-1000.0);
        assert!((navigator.distance() - MAX_DISTANCE).abs() < 1.0);
        assert!(centred(&navigator));
    }

    #[test]
    fn the_wheel_and_right_drag_zoom_the_expected_way() {
        let mut navigator = Navigator::default();
        let distance = navigator.distance();
        navigator.wheel(50.0);
        assert!(
            navigator.distance() < distance,
            "wheel away from you: closer"
        );
        let mut navigator = Navigator::default();
        navigator.drag(DragKind::Dolly, Vec2::new(40.0, 0.0), SIZE);
        assert!(navigator.distance() < distance, "drag right: closer");
        let mut navigator = Navigator::default();
        navigator.drag(DragKind::Dolly, Vec2::new(0.0, 40.0), SIZE);
        assert!(navigator.distance() > distance, "drag down: farther");
    }

    #[test]
    fn drags_go_to_the_right_movement() {
        let mut orbited = Navigator::default();
        orbited.drag(DragKind::Orbit, Vec2::new(10.0, 0.0), SIZE);
        assert_ne!(orbited.camera().yaw, Navigator::default().camera().yaw);
        let mut panned = Navigator::default();
        panned.drag(DragKind::Pan, Vec2::new(10.0, 0.0), SIZE);
        assert_ne!(panned.pivot(), DEFAULT_TARGET);
    }

    #[test]
    fn broken_input_changes_nothing() {
        let start = Navigator::default();
        let mut navigator = start;
        for bad in [f32::NAN, f32::INFINITY] {
            navigator.orbit(Vec2::new(bad, 1.0));
            navigator.pan(Vec2::new(1.0, bad), SIZE.y);
            navigator.dolly(bad);
            navigator.wheel(bad);
            navigator.drag(DragKind::Pan, Vec2::splat(bad), SIZE);
        }
        navigator.pan(Vec2::new(5.0, 5.0), 0.0);
        navigator.pan(Vec2::new(5.0, 5.0), -10.0);
        assert_eq!(navigator, start);
    }

    #[test]
    fn a_camera_standing_on_its_pivot_gets_a_pivot_in_front() {
        let camera = Camera::default();
        let navigator = Navigator::new(camera, camera.position);
        assert!((navigator.distance() - MIN_DISTANCE).abs() < 1e-3);
        assert!(navigator.camera().position.is_finite());
    }

    #[test]
    fn a_pivot_straight_below_is_still_looked_at() {
        let camera = Camera::looking_at(Vec3::new(0.0, 0.0, 500.0), Vec3::ZERO);
        let navigator = Navigator::new(camera, Vec3::ZERO);
        assert!(centred(&navigator));
        assert!((navigator.distance() - 500.0).abs() < 0.01);
    }

    #[test]
    fn reset_returns_to_the_opening_view() {
        let mut navigator = Navigator::default();
        navigator.orbit(Vec2::new(300.0, 120.0));
        navigator.pan(Vec2::new(50.0, 50.0), SIZE.y);
        navigator.dolly(1.0);
        navigator.reset();
        assert_eq!(navigator, Navigator::default());
    }
}
