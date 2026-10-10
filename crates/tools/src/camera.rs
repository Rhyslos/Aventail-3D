//! The viewport camera: where it is and where it looks.
//!
//! **Z is up** (decision 0012): X points right, Y points away from the
//! viewer in the front view, Z points up. Right-handed. Units are
//! centimetres.

use glam::{Mat4, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// The world's up direction.
pub const WORLD_UP: Vec3 = Vec3::Z;
/// Steepest the camera may look up or down: just short of straight, so the
/// view never flips over.
pub const MAX_PITCH: f32 = 89.0 * PI / 180.0;
/// Nearest distance the camera draws, in centimetres.
pub const NEAR_PLANE: f32 = 1.0;
/// Farthest distance the camera draws, in centimetres (5 km: far beyond
/// any game asset).
pub const FAR_PLANE: f32 = 500_000.0;
/// Default vertical field of view: a gentle 45°, like a 30 mm lens on a
/// 36 mm frame, so shapes are not distorted.
pub const DEFAULT_FOV_Y: f32 = 45.0 * PI / 180.0;

/// A straight line from `origin` in `direction` (unit length).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    /// Where the ray starts.
    pub origin: Vec3,
    /// Which way it points; unit length.
    pub direction: Vec3,
}

impl Ray {
    /// The point `distance` along the ray.
    pub fn at(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    /// Distance along the ray to the ground plane at height `z`, if the ray
    /// hits it in front of its origin.
    pub fn hit_ground(&self, z: f32) -> Option<f32> {
        if self.direction.z.abs() < 1e-6 {
            return None;
        }
        let distance = (z - self.origin.z) / self.direction.z;
        (distance > 0.0 && distance.is_finite()).then_some(distance)
    }
}

/// A perspective camera described by its position and two angles.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    /// Where the camera is.
    pub position: Vec3,
    /// Turn left or right, in radians, measured from +X towards +Y.
    pub yaw: f32,
    /// Look up or down, in radians; positive looks up.
    pub pitch: f32,
    /// Vertical field of view, in radians.
    pub fov_y: f32,
}

impl Default for Camera {
    /// The opening view: in front of the origin, a little to the right and
    /// above, looking at a 2 m object standing on the grid, as Cinema 4D
    /// opens a new scene.
    fn default() -> Self {
        Self::looking_at(Vec3::new(420.0, -760.0, 480.0), Vec3::new(0.0, 0.0, 60.0))
    }
}

impl Camera {
    /// A camera at `position` looking at `target`.
    pub fn looking_at(position: Vec3, target: Vec3) -> Self {
        let direction = (target - position).normalize_or_zero();
        let (yaw, pitch) = if direction == Vec3::ZERO {
            (0.0, 0.0)
        } else {
            (
                direction.y.atan2(direction.x),
                direction
                    .z
                    .clamp(-1.0, 1.0)
                    .asin()
                    .clamp(-MAX_PITCH, MAX_PITCH),
            )
        };
        Self {
            position,
            yaw,
            pitch,
            fov_y: DEFAULT_FOV_Y,
        }
    }

    /// The direction the camera looks.
    pub fn forward(&self) -> Vec3 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Vec3::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch)
    }

    /// The camera's right-hand direction; always level with the ground.
    pub fn right(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Vec3::new(sin_yaw, -cos_yaw, 0.0)
    }

    /// The camera's up direction (towards the top of the screen).
    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward())
    }

    /// World-to-camera transform (right-handed; the camera looks down its
    /// own -Z axis, with its own Y pointing up the screen).
    pub fn view_matrix(&self) -> Mat4 {
        let (f, r, u) = (self.forward(), self.right(), self.up());
        let eye = self.position;
        Mat4::from_cols(
            Vec4::new(r.x, u.x, -f.x, 0.0),
            Vec4::new(r.y, u.y, -f.y, 0.0),
            Vec4::new(r.z, u.z, -f.z, 0.0),
            Vec4::new(-r.dot(eye), -u.dot(eye), f.dot(eye), 1.0),
        )
    }

    /// Camera-to-screen transform for a viewport of the given aspect ratio
    /// (width / height). Depth runs from 0 (near) to 1 (far), as wgpu expects.
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        let aspect = if aspect.is_finite() && aspect > 1e-3 {
            aspect
        } else {
            1.0
        };
        let y_scale = 1.0 / (self.fov_y * 0.5).tan();
        let x_scale = y_scale / aspect;
        let depth = FAR_PLANE / (NEAR_PLANE - FAR_PLANE);
        Mat4::from_cols(
            Vec4::new(x_scale, 0.0, 0.0, 0.0),
            Vec4::new(0.0, y_scale, 0.0, 0.0),
            Vec4::new(0.0, 0.0, depth, -1.0),
            Vec4::new(0.0, 0.0, depth * NEAR_PLANE, 0.0),
        )
    }

    /// World-to-screen transform for a viewport of `size` (any unit).
    pub fn view_projection(&self, size: Vec2) -> Mat4 {
        self.projection_matrix(aspect_of(size)) * self.view_matrix()
    }

    /// The ray from the camera through a point in the viewport, given in the
    /// same units as `size`, measured from the top-left corner.
    pub fn ray_through(&self, point: Vec2, size: Vec2) -> Ray {
        let size = size.max(Vec2::ONE);
        let ndc_x = 2.0 * point.x / size.x - 1.0;
        let ndc_y = 1.0 - 2.0 * point.y / size.y;
        let half_height = (self.fov_y * 0.5).tan();
        let half_width = half_height * aspect_of(size);
        let direction = (self.forward()
            + self.right() * (ndc_x * half_width)
            + self.up() * (ndc_y * half_height))
            .normalize_or(self.forward());
        Ray {
            origin: self.position,
            direction,
        }
    }

    /// Where a world point appears in a viewport of `size`, measured from the
    /// top-left corner. `None` when the point is behind the camera.
    pub fn project(&self, point: Vec3, size: Vec2) -> Option<Vec2> {
        let clip = self.view_projection(size) * point.extend(1.0);
        if clip.w <= 1e-4 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(Vec2::new(
            (ndc.x + 1.0) * 0.5 * size.x,
            (1.0 - ndc.y) * 0.5 * size.y,
        ))
    }
}

/// Width divided by height, with a safe fallback for empty sizes.
fn aspect_of(size: Vec2) -> f32 {
    if size.y > 0.0 && size.x > 0.0 {
        size.x / size.y
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Vec2 = Vec2::new(1000.0, 600.0);

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn z_is_up_and_the_view_stays_level() {
        let camera = Camera::default();
        assert!(camera.up().z > 0.5, "screen-up points up the world");
        assert_eq!(camera.right().z, 0.0, "no tilt to the side");
    }

    #[test]
    fn x_points_right_and_y_away_in_the_front_view() {
        // Standing at -Y, looking towards +Y.
        let camera = Camera::looking_at(Vec3::new(0.0, -500.0, 0.0), Vec3::ZERO);
        assert!(close(camera.right(), Vec3::X));
        assert!(close(camera.forward(), Vec3::Y));
        assert!(close(camera.up(), Vec3::Z));
    }

    #[test]
    fn the_target_appears_in_the_middle() {
        let target = Vec3::new(10.0, 20.0, 30.0);
        let camera = Camera::looking_at(Vec3::new(300.0, -400.0, 250.0), target);
        let at = camera.project(target, SIZE).unwrap();
        assert!((at - SIZE / 2.0).length() < 0.01);
    }

    #[test]
    fn the_default_view_shows_x_to_the_right_and_z_up() {
        let camera = Camera::default();
        let origin = camera.project(Vec3::ZERO, SIZE).unwrap();
        let along_x = camera.project(Vec3::X * 100.0, SIZE).unwrap();
        let along_z = camera.project(Vec3::Z * 100.0, SIZE).unwrap();
        assert!(along_x.x > origin.x, "X goes right on screen");
        assert!(along_z.y < origin.y, "Z goes up on screen");
    }

    #[test]
    fn the_default_view_shows_a_two_metre_cube_whole() {
        let camera = Camera::default();
        for x in [-100.0, 100.0] {
            for y in [-100.0, 100.0] {
                for z in [0.0, 200.0] {
                    let p = camera.project(Vec3::new(x, y, z), SIZE).unwrap();
                    assert!(p.x > 0.0 && p.x < SIZE.x && p.y > 0.0 && p.y < SIZE.y);
                }
            }
        }
    }

    #[test]
    fn points_behind_the_camera_are_not_projected() {
        let camera = Camera::default();
        let behind = camera.position - camera.forward() * 10.0;
        assert!(camera.project(behind, SIZE).is_none());
    }

    #[test]
    fn a_ray_through_a_projected_point_passes_through_it() {
        let camera = Camera::default();
        let point = Vec3::new(50.0, 80.0, 0.0);
        let at = camera.project(point, SIZE).unwrap();
        let ray = camera.ray_through(at, SIZE);
        let distance = ray.hit_ground(0.0).unwrap();
        assert!((ray.at(distance) - point).length() < 0.5);
    }

    #[test]
    fn the_centre_ray_points_forward() {
        let camera = Camera::default();
        let ray = camera.ray_through(SIZE / 2.0, SIZE);
        assert!(close(ray.direction, camera.forward()));
    }

    #[test]
    fn rays_parallel_to_the_ground_never_hit_it() {
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, 100.0),
            direction: Vec3::X,
        };
        assert_eq!(ray.hit_ground(0.0), None);
        let up = Ray {
            origin: Vec3::new(0.0, 0.0, 100.0),
            direction: Vec3::Z,
        };
        assert_eq!(up.hit_ground(0.0), None, "the ground is behind");
    }

    #[test]
    fn depth_runs_from_zero_at_near_to_one_at_far() {
        let camera = Camera::looking_at(Vec3::ZERO, Vec3::Y);
        let vp = camera.view_projection(SIZE);
        let depth = |d: f32| {
            let clip = vp * Vec3::new(0.0, d, 0.0).extend(1.0);
            clip.z / clip.w
        };
        assert!(depth(NEAR_PLANE).abs() < 1e-4);
        assert!((depth(FAR_PLANE) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn looking_straight_down_does_not_flip() {
        let camera = Camera::looking_at(Vec3::new(0.0, 0.0, 500.0), Vec3::ZERO);
        assert!(camera.pitch >= -MAX_PITCH);
        assert!(camera.forward().z < -0.99);
        assert!(camera.up().is_finite());
    }

    #[test]
    fn bad_sizes_do_not_break_the_maths() {
        let camera = Camera::default();
        for size in [Vec2::ZERO, Vec2::new(0.0, 100.0), Vec2::new(f32::NAN, 1.0)] {
            assert!(camera.view_projection(size).is_finite() || size.x.is_nan());
        }
        assert!(camera.view_projection(Vec2::ZERO).is_finite());
    }

    #[test]
    fn the_camera_survives_saving() {
        let camera = Camera::default();
        let text = ron::to_string(&camera).unwrap();
        assert_eq!(ron::from_str::<Camera>(&text).unwrap(), camera);
    }
}
