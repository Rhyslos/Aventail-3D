//! Moving the viewport camera: orbit, pan and zoom (dolly).
//!
//! Which buttons and keys start each movement is decided in
//! [`crate::controls`]; this module does the movements themselves.
//!
//! Every movement is around a **pivot**: the point the camera orbits
//! around and zooms towards. It starts as the point the camera looks at,
//! and the viewport moves it to whatever is under the mouse when an orbit
//! or a zoom starts ([`Navigator::set_pivot`], [`Navigator::dolly_towards`]),
//! so you turn around and move towards what you point at. Panning carries
//! the pivot along.
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

/// The viewport camera together with the pivot it turns around.
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

    /// The point the camera orbits around and zooms towards.
    pub fn pivot(&self) -> Vec3 {
        self.pivot
    }

    /// How far the camera is from the pivot, in centimetres.
    pub fn distance(&self) -> f32 {
        self.camera.position.distance(self.pivot)
    }

    /// Makes `point` the pivot, without moving the camera: the next orbit
    /// turns around it. Refused (returning `false`) for a point behind the
    /// camera, closer than [`MIN_DISTANCE`], farther than [`MAX_DISTANCE`],
    /// or not a real number.
    pub fn set_pivot(&mut self, point: Vec3) -> bool {
        if !self.usable_target(point) {
            return false;
        }
        self.pivot = point;
        true
    }

    /// Whether the camera could orbit around or zoom towards `point`.
    fn usable_target(&self, point: Vec3) -> bool {
        let to = point - self.camera.position;
        point.is_finite()
            && to.dot(self.camera.forward()) >= MIN_DISTANCE
            && to.length() <= MAX_DISTANCE
    }

    /// How far in front of the camera the pivot is, along the view
    /// direction (its depth); at least [`MIN_DISTANCE`].
    fn pivot_depth(&self) -> f32 {
        let depth = (self.pivot - self.camera.position).dot(self.camera.forward());
        if depth.is_finite() {
            depth.max(MIN_DISTANCE)
        } else {
            MIN_DISTANCE
        }
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
    /// away from you) moves closer to the pivot.
    pub fn wheel(&mut self, scroll: f32) {
        self.dolly(scroll * WHEEL_DOLLY_SPEED);
    }

    /// Zooms for a turn of the mouse wheel, towards `target` (the point
    /// under the mouse), which becomes the pivot. See [`Self::dolly_towards`].
    pub fn wheel_towards(&mut self, scroll: f32, target: Vec3) {
        self.dolly_towards(target, scroll * WHEEL_DOLLY_SPEED);
    }

    /// Turns the camera around the pivot, wherever it is on screen: the
    /// pivot stays exactly where it is in the view while everything turns
    /// around it. The scene follows the mouse: dragging right turns its
    /// front to the right, dragging down tips its top towards you.
    pub fn orbit(&mut self, delta: Vec2) {
        if !delta.is_finite() {
            return;
        }
        // Where the pivot sits as seen from the camera (right, up and
        // forward of it) stays the same; only the view direction turns.
        // Rebuilding the position from that each time, rather than turning
        // it bit by bit, keeps rounding errors from adding up.
        let to_pivot = self.pivot - self.camera.position;
        let seen = Vec3::new(
            to_pivot.dot(self.camera.right()),
            to_pivot.dot(self.camera.up()),
            to_pivot.dot(self.camera.forward()),
        );
        // Turn left or right, then tip up or down, stopping just short of
        // looking straight up or down.
        self.camera.yaw = wrap_angle(self.camera.yaw - delta.x * ORBIT_SPEED);
        self.camera.pitch =
            (self.camera.pitch - delta.y * ORBIT_SPEED).clamp(-MAX_PITCH, MAX_PITCH);
        let (right, up, forward) = (self.camera.right(), self.camera.up(), self.camera.forward());
        self.camera.position = self.pivot - (right * seen.x + up * seen.y + forward * seen.z);
    }

    /// Slides the camera and the pivot together so that points at the
    /// pivot's depth stay exactly under the mouse. `view_height` is the
    /// viewport's height in the same unit as `delta`.
    pub fn pan(&mut self, delta: Vec2, view_height: f32) {
        if !(delta.is_finite() && view_height.is_finite() && view_height > 0.0) {
            return;
        }
        // Height of the visible world at the pivot's depth, per unit.
        let per_unit = 2.0 * self.pivot_depth() * (self.camera.fov_y * 0.5).tan() / view_height;
        let shift = (-self.camera.right() * delta.x + self.camera.up() * delta.y) * per_unit;
        self.camera.position += shift;
        self.pivot += shift;
    }

    /// Moves towards (positive `amount`) or away from the pivot. The
    /// distance changes by a factor of e^amount, so each step feels the
    /// same whether near or far. The camera keeps its direction.
    pub fn dolly(&mut self, amount: f32) {
        self.move_towards(self.pivot, amount);
    }

    /// Moves towards (positive `amount`) or away from `target`, which
    /// becomes the pivot. The target stays at the same place on screen, so
    /// the view zooms in on whatever is under the mouse. A target behind
    /// the camera, too close or too far is ignored, and the view zooms
    /// towards the pivot instead.
    pub fn dolly_towards(&mut self, target: Vec3, amount: f32) {
        if self.usable_target(target) {
            self.pivot = target;
        }
        self.dolly(amount);
    }

    /// Moves the camera straight towards (or away from) `target`, keeping
    /// its direction: the distance changes by a factor of e^-amount, within
    /// [`MIN_DISTANCE`] and [`MAX_DISTANCE`].
    fn move_towards(&mut self, target: Vec3, amount: f32) {
        let to = target - self.camera.position;
        let distance = to.length();
        if !(amount.is_finite() && distance.is_finite() && distance > 1e-6) {
            return;
        }
        let wanted = (distance * (-amount).exp()).clamp(MIN_DISTANCE, MAX_DISTANCE);
        self.camera.position = target - to / distance * wanted;
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

    /// A point on the cube's front-right corner area, off the middle of
    /// the opening view.
    const OFF_CENTRE: Vec3 = Vec3::new(100.0, -60.0, 150.0);

    #[test]
    fn orbiting_around_a_point_keeps_it_still_on_screen() {
        let mut navigator = Navigator::default();
        assert!(navigator.set_pivot(OFF_CENTRE));
        let start = on_screen(&navigator, OFF_CENTRE);
        let distance = navigator.distance();
        for delta in [
            Vec2::new(80.0, 0.0),
            Vec2::new(-30.0, 45.0),
            Vec2::new(10.0, -90.0),
        ] {
            navigator.orbit(delta);
            let now = on_screen(&navigator, OFF_CENTRE);
            assert!((now - start).length() < 0.05, "{now} vs {start}");
            assert!((navigator.distance() - distance).abs() < 0.01);
            assert!(navigator.camera().up().z > 0.0);
        }
    }

    #[test]
    fn orbiting_around_a_point_still_never_flips() {
        let mut navigator = Navigator::default();
        assert!(navigator.set_pivot(OFF_CENTRE));
        let start = on_screen(&navigator, OFF_CENTRE);
        for _ in 0..40 {
            navigator.orbit(Vec2::new(23.0, 300.0));
        }
        assert!(navigator.camera().pitch >= -MAX_PITCH);
        let now = on_screen(&navigator, OFF_CENTRE);
        assert!((now - start).length() < 0.05, "{now} vs {start}");
    }

    #[test]
    fn setting_the_pivot_does_not_move_the_camera() {
        let mut navigator = Navigator::default();
        let camera = *navigator.camera();
        assert!(navigator.set_pivot(OFF_CENTRE));
        assert_eq!(*navigator.camera(), camera);
        assert_eq!(navigator.pivot(), OFF_CENTRE);
    }

    #[test]
    fn unusable_pivots_are_refused() {
        let mut navigator = Navigator::default();
        let behind = navigator.camera().position - navigator.camera().forward() * 100.0;
        let too_close = navigator.camera().position + navigator.camera().forward() * 0.5;
        let too_far = navigator.camera().position + navigator.camera().forward() * 1e7;
        for point in [behind, too_close, too_far, Vec3::NAN, Vec3::INFINITY] {
            assert!(!navigator.set_pivot(point), "{point}");
        }
        assert_eq!(navigator.pivot(), DEFAULT_TARGET);
    }

    #[test]
    fn zooming_towards_a_point_keeps_it_under_the_mouse() {
        let mut navigator = Navigator::default();
        let start = on_screen(&navigator, OFF_CENTRE);
        let before = navigator.camera().position.distance(OFF_CENTRE);
        let (yaw, pitch) = (navigator.camera().yaw, navigator.camera().pitch);
        navigator.dolly_towards(OFF_CENTRE, 0.4);
        let now = on_screen(&navigator, OFF_CENTRE);
        assert!((now - start).length() < 0.05, "{now} vs {start}");
        assert!(navigator.distance() < before);
        assert_eq!(navigator.pivot(), OFF_CENTRE, "the point becomes the pivot");
        assert_eq!(
            (navigator.camera().yaw, navigator.camera().pitch),
            (yaw, pitch),
            "zooming never turns the view"
        );
    }

    #[test]
    fn zooming_towards_a_point_behind_falls_back_to_the_pivot() {
        let mut navigator = Navigator::default();
        let behind = navigator.camera().position - navigator.camera().forward() * 300.0;
        let before = navigator.distance();
        navigator.dolly_towards(behind, 0.4);
        assert_eq!(navigator.pivot(), DEFAULT_TARGET);
        assert!(navigator.distance() < before);
        assert!(centred(&navigator));
    }

    #[test]
    fn the_wheel_zooms_towards_a_point() {
        let mut navigator = Navigator::default();
        let before = navigator.camera().position.distance(OFF_CENTRE);
        navigator.wheel_towards(50.0, OFF_CENTRE);
        assert!(navigator.distance() < before);
        navigator.wheel_towards(-1.0e6, OFF_CENTRE);
        assert!(
            (navigator.distance() - MAX_DISTANCE).abs() < 1.0,
            "zooming out stops too"
        );
    }

    #[test]
    fn panning_with_an_off_centre_pivot_keeps_it_under_the_mouse() {
        let mut navigator = Navigator::default();
        assert!(navigator.set_pivot(OFF_CENTRE));
        let start = on_screen(&navigator, OFF_CENTRE);
        let delta = Vec2::new(-60.0, 25.0);
        navigator.pan(delta, SIZE.y);
        // The pivot moves along; a fixed point at its old place in the
        // world appears to follow the mouse.
        let moved = navigator.pivot();
        assert!((on_screen(&navigator, moved) - start).length() < 0.05);
        let world_point_now = on_screen(&navigator, OFF_CENTRE);
        assert!((world_point_now - (start + delta)).length() < 0.05);
    }
}
