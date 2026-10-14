//! What the renderer needs to know each frame, and how it is laid out for
//! the GPU.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// Half the width of the drawn grid, in centimetres (100 m each way: far
/// more room than any game asset needs).
pub const GRID_HALF_EXTENT: f32 = 10_000.0;
/// Spacing of the three grid levels, in centimetres: 10 cm, 1 m and 10 m.
/// The finest level can also be drawn at 50 cm instead (see
/// [`FrameParams::fine_spacing`]).
pub const GRID_SPACINGS: [f32; 3] = [10.0, 100.0, 1000.0];
/// How strongly 50 cm lines are drawn: a little stronger than 10 cm ones,
/// as there are fewer of them.
pub const HALF_METRE_STRENGTH: f32 = 0.05;
/// How strongly each grid level is drawn (opacity of a white line, 0 to 1),
/// in the same order as [`GRID_SPACINGS`]. Kept low so the grid stays a
/// quiet guide rather than a pattern.
pub const GRID_STRENGTHS: [f32; 3] = [0.035, 0.075, 0.12];

/// Smallest distance around the focus point that the grid reaches, in
/// centimetres (18 m: even zoomed right in, the ground stretches well
/// beyond the object, so the view never feels closed in).
pub const GRID_MIN_REACH: f32 = 1800.0;
/// How far the grid reaches compared with the camera's distance from the
/// focus point: zooming out shows more of it.
pub const GRID_REACH_PER_DISTANCE: f32 = 2.0;

/// Everything that changes from frame to frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameParams {
    /// World-to-screen transform (depth 0 at near, 1 at far).
    pub view_projection: Mat4,
    /// Where the camera is.
    pub camera_position: Vec3,
    /// The point the camera looks at; the grid is drawn around it.
    pub focus: Vec3,
    /// The sky straight up, sRGB 0 to 1 (as colour pickers and the
    /// interface theme give them).
    pub sky_top: [f32; 3],
    /// The sky at the horizon, sRGB 0 to 1.
    pub sky_horizon: [f32; 3],
    /// The ground just below the horizon, sRGB 0 to 1.
    pub ground: [f32; 3],
    /// The spacing of the grid's finest lines in centimetres (10 cm, or
    /// 50 cm to cut each metre square into four), or `None` for only the
    /// 1 m and 10 m lines.
    pub fine_spacing: Option<f32>,
    /// For a flat (orthographic) view, the height it shows in centimetres;
    /// `None` for perspective. Flat views get a plain backdrop, axes drawn
    /// in full, and a grid sized to the view.
    pub flat_height: Option<f32>,
    /// The two directions the grid's plane spans: X and Y for the ground
    /// ([`GROUND_PLANE`]), or an upright plane facing a flat side view.
    pub grid_plane: [Vec3; 2],
}

/// The ground's grid plane: X and Y.
pub const GROUND_PLANE: [Vec3; 2] = [Vec3::X, Vec3::Y];

/// Converts one sRGB channel (0 to 1) to linear light, as the shaders work.
pub fn srgb_to_linear(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The finest grid level's spacing and the strength of each level for the
/// shader (the fourth strength is unused). Without fine lines the finest
/// level is hidden; a spacing that isn't a usable number counts as none.
fn grid_levels(fine_spacing: Option<f32>) -> (f32, [f32; 4]) {
    let [minor, major, super_level] = GRID_STRENGTHS;
    match fine_spacing.filter(|s| s.is_finite() && *s > 0.0 && *s < GRID_SPACINGS[1]) {
        Some(spacing) => {
            let strength = if spacing >= 50.0 {
                HALF_METRE_STRENGTH
            } else {
                minor
            };
            (spacing, [strength, major, super_level, 0.0])
        }
        None => (GRID_SPACINGS[0], [0.0, major, super_level, 0.0]),
    }
}

fn linear(rgb: [f32; 3]) -> [f32; 4] {
    [
        srgb_to_linear(rgb[0]),
        srgb_to_linear(rgb[1]),
        srgb_to_linear(rgb[2]),
        1.0,
    ]
}

/// The frame data exactly as the shader's `Frame` struct expects it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct FrameUniforms {
    view_projection: [[f32; 4]; 4],
    inverse_view_projection: [[f32; 4]; 4],
    camera: [f32; 4],
    focus: [f32; 4],
    grid: [f32; 4],
    grid_strength: [f32; 4],
    grid_u: [f32; 4],
    grid_v: [f32; 4],
    view: [f32; 4],
    sky_top: [f32; 4],
    sky_horizon: [f32; 4],
    ground: [f32; 4],
    /// Which way is right and up on screen, in the world (unit; w unused):
    /// cavity reads surfaces' facing across the screen with them.
    screen_right: [f32; 4],
    screen_up: [f32; 4],
}

/// Which way is right and up on screen, in the world, for a view: the
/// view-projection's first two rows, made unit length.
fn screen_axes(view_projection: Mat4) -> (Vec3, Vec3) {
    let row = |i: usize| {
        Vec3::new(
            view_projection.x_axis[i],
            view_projection.y_axis[i],
            view_projection.z_axis[i],
        )
        .normalize_or_zero()
    };
    (row(0), row(1))
}

impl FrameUniforms {
    pub(crate) fn new(params: &FrameParams) -> Self {
        let [_, major, super_level] = GRID_SPACINGS;
        let (minor, strengths) = grid_levels(params.fine_spacing);
        let p = params.camera_position;
        let f = params.focus;
        let reach = match params.flat_height {
            Some(height) => flat_grid_reach(height),
            None => grid_reach(p.distance(f)),
        };
        let [u, v] = usable_plane(params.grid_plane);
        let inverse = params.view_projection.inverse();
        let inverse = if inverse.is_finite() {
            inverse
        } else {
            Mat4::IDENTITY
        };
        Self {
            view_projection: params.view_projection.to_cols_array_2d(),
            inverse_view_projection: inverse.to_cols_array_2d(),
            camera: [p.x, p.y, p.z, 1.0],
            focus: [f.x, f.y, f.z, reach],
            grid: [minor, major, super_level, GRID_HALF_EXTENT],
            grid_strength: strengths,
            grid_u: [u.x, u.y, u.z, 0.0],
            grid_v: [v.x, v.y, v.z, 0.0],
            view: [
                if params.flat_height.is_some() {
                    1.0
                } else {
                    0.0
                },
                0.0,
                0.0,
                0.0,
            ],
            sky_top: linear(params.sky_top),
            sky_horizon: linear(params.sky_horizon),
            ground: linear(params.ground),
            screen_right: {
                let r = screen_axes(params.view_projection).0;
                [r.x, r.y, r.z, 0.0]
            },
            screen_up: {
                let u = screen_axes(params.view_projection).1;
                [u.x, u.y, u.z, 0.0]
            },
        }
    }

    /// The same uniforms for an image of `size` pixels: the overlay draws
    /// points and thick lines a fixed number of pixels wide.
    pub(crate) fn with_image_size(mut self, size: [u32; 2]) -> Self {
        self.view[1] = size[0].max(1) as f32;
        self.view[2] = size[1].max(1) as f32;
        self
    }
}

/// How far around the focus point a flat view's grid reaches, for a view
/// showing `height` centimetres: past the view's edges, so a flat view's
/// grid looks endless.
pub(crate) fn flat_grid_reach(height: f32) -> f32 {
    let reach = height * 2.0;
    if reach.is_finite() {
        reach.clamp(GRID_MIN_REACH, GRID_HALF_EXTENT)
    } else {
        GRID_MIN_REACH
    }
}

/// The grid plane's two directions, made unit length; the ground's if
/// they are broken or point the same way.
fn usable_plane([u, v]: [Vec3; 2]) -> [Vec3; 2] {
    let (u, v) = (u.normalize_or_zero(), v.normalize_or_zero());
    if u == Vec3::ZERO || v == Vec3::ZERO || u.cross(v).length() < 1e-3 {
        GROUND_PLANE
    } else {
        [u, v]
    }
}

/// How far around the focus point the grid reaches before it has faded
/// out, for a camera `distance` away from it.
pub(crate) fn grid_reach(distance: f32) -> f32 {
    let reach = distance * GRID_REACH_PER_DISTANCE;
    if reach.is_finite() {
        reach.clamp(GRID_MIN_REACH, GRID_HALF_EXTENT)
    } else {
        GRID_MIN_REACH
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniforms_match_the_shader_layout() {
        // Two mat4x4<f32> and twelve vec4<f32>, no padding.
        assert_eq!(std::mem::size_of::<FrameUniforms>(), 320);
        assert_eq!(std::mem::size_of::<FrameUniforms>() % 16, 0);
    }

    #[test]
    fn the_screen_axes_point_right_and_up() {
        let camera = tools::Camera::looking_at(Vec3::new(0.0, -500.0, 0.0), Vec3::ZERO);
        let (right, up) = screen_axes(camera.view_projection(glam::Vec2::new(1600.0, 900.0)));
        assert!(right.distance(Vec3::X) < 1e-4, "{right}");
        assert!(up.distance(Vec3::Z) < 1e-4, "{up}");
    }

    #[test]
    fn grid_levels_grow_tenfold() {
        let [a, b, c] = GRID_SPACINGS;
        assert_eq!(b, a * 10.0);
        assert_eq!(c, b * 10.0);
        assert!(c < GRID_HALF_EXTENT);
    }

    #[test]
    fn srgb_conversion_matches_known_values() {
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-6);
        // Mid grey in sRGB is about 21% light.
        assert!((srgb_to_linear(0.5) - 0.214).abs() < 0.001);
        assert_eq!(srgb_to_linear(-3.0), 0.0);
        assert!((srgb_to_linear(9.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn uniforms_carry_the_camera_and_sky() {
        let u = FrameUniforms::new(&FrameParams {
            view_projection: Mat4::IDENTITY,
            camera_position: Vec3::new(1.0, 2.0, 3.0),
            focus: Vec3::new(1.0, 2.0, 1003.0),
            sky_top: [1.0, 0.5, 0.0],
            sky_horizon: [0.0, 0.0, 0.0],
            ground: [0.5, 0.5, 0.5],
            fine_spacing: Some(10.0),
            flat_height: None,
            grid_plane: GROUND_PLANE,
        });
        assert_eq!(u.camera, [1.0, 2.0, 3.0, 1.0]);
        assert_eq!(u.focus[..3], [1.0, 2.0, 1003.0]);
        assert_eq!(u.focus[3], grid_reach(1000.0));
        assert_eq!(u.inverse_view_projection, Mat4::IDENTITY.to_cols_array_2d());
        assert!((u.ground[0] - 0.214).abs() < 0.001);
        assert_eq!(u.grid[3], GRID_HALF_EXTENT);
        assert_eq!(u.sky_top[0], 1.0);
        assert!((u.sky_top[1] - 0.214).abs() < 0.001);
        assert_eq!(u.view_projection, Mat4::IDENTITY.to_cols_array_2d());
        assert_eq!(u.grid_strength[0], GRID_STRENGTHS[0]);
    }

    #[test]
    fn the_finest_level_can_be_off_half_metres_or_ten_centimetres() {
        let [minor, major, super_level] = GRID_STRENGTHS;
        assert_eq!(grid_levels(None), (10.0, [0.0, major, super_level, 0.0]));
        assert_eq!(
            grid_levels(Some(10.0)),
            (10.0, [minor, major, super_level, 0.0])
        );
        assert_eq!(
            grid_levels(Some(50.0)),
            (50.0, [HALF_METRE_STRENGTH, major, super_level, 0.0])
        );
        for broken in [0.0, -5.0, f32::NAN, 100.0, 5000.0] {
            assert_eq!(grid_levels(Some(broken)).1[0], 0.0, "{broken}");
        }
    }

    #[test]
    fn the_grid_reaches_farther_when_zoomed_out() {
        assert_eq!(grid_reach(0.0), GRID_MIN_REACH);
        assert!(grid_reach(5000.0) > grid_reach(1500.0));
        assert_eq!(grid_reach(1e9), GRID_HALF_EXTENT);
        assert_eq!(grid_reach(f32::NAN), GRID_MIN_REACH);
    }

    #[test]
    fn a_broken_matrix_still_gives_usable_uniforms() {
        let u = FrameUniforms::new(&FrameParams {
            view_projection: Mat4::ZERO,
            camera_position: Vec3::ZERO,
            focus: Vec3::ZERO,
            sky_top: [0.0; 3],
            sky_horizon: [0.0; 3],
            ground: [0.0; 3],
            fine_spacing: None,
            flat_height: None,
            grid_plane: GROUND_PLANE,
        });
        assert!(
            u.inverse_view_projection
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
    }

    #[test]
    fn flat_views_get_their_own_grid_plane_and_reach() {
        let u = FrameUniforms::new(&FrameParams {
            view_projection: Mat4::IDENTITY,
            camera_position: Vec3::new(0.0, -50_000.0, 0.0),
            focus: Vec3::ZERO,
            sky_top: [0.0; 3],
            sky_horizon: [0.0; 3],
            ground: [0.0; 3],
            fine_spacing: None,
            flat_height: Some(1500.0),
            grid_plane: [Vec3::X, Vec3::Z * 3.0],
        });
        assert_eq!(u.view[0], 1.0, "flat");
        assert_eq!(u.grid_v[..3], [0.0, 0.0, 1.0], "made unit length");
        assert_eq!(
            u.focus[3],
            flat_grid_reach(1500.0),
            "sized to the view, not the standoff"
        );
    }

    #[test]
    fn a_broken_grid_plane_falls_back_to_the_ground() {
        assert_eq!(usable_plane([Vec3::ZERO, Vec3::Z]), GROUND_PLANE);
        assert_eq!(usable_plane([Vec3::X, Vec3::X * 2.0]), GROUND_PLANE);
        assert_eq!(usable_plane([Vec3::NAN, Vec3::Y]), GROUND_PLANE);
        assert_eq!(usable_plane([Vec3::Y, Vec3::Z]), [Vec3::Y, Vec3::Z]);
    }

    #[test]
    fn coarser_grid_levels_are_stronger() {
        let [a, b, c] = GRID_STRENGTHS;
        assert!(0.0 < a && a < b && b < c && c < 0.5, "a quiet grid");
    }
}
