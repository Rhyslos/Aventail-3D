//! What the renderer needs to know each frame, and how it is laid out for
//! the GPU.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// Half the width of the drawn grid, in centimetres (100 m each way: far
/// more room than any game asset needs).
pub const GRID_HALF_EXTENT: f32 = 10_000.0;
/// Spacing of the three grid levels, in centimetres: 10 cm, 1 m and 10 m.
pub const GRID_SPACINGS: [f32; 3] = [10.0, 100.0, 1000.0];
/// How strongly each grid level is drawn (opacity of a white line, 0 to 1),
/// in the same order as [`GRID_SPACINGS`]. Kept low so the grid stays a
/// quiet guide rather than a pattern.
pub const GRID_STRENGTHS: [f32; 3] = [0.045, 0.11, 0.18];

/// Everything that changes from frame to frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameParams {
    /// World-to-screen transform (depth 0 at near, 1 at far).
    pub view_projection: Mat4,
    /// Where the camera is.
    pub camera_position: Vec3,
    /// Background colour at the top of the view, sRGB 0 to 1 (as colour
    /// pickers and the interface theme give them).
    pub sky_top: [f32; 3],
    /// Background colour at the bottom of the view, sRGB 0 to 1.
    pub sky_bottom: [f32; 3],
    /// Whether to draw the grid's finest level (10 cm lines).
    pub fine_grid: bool,
}

/// Converts one sRGB channel (0 to 1) to linear light, as the shaders work.
pub fn srgb_to_linear(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The strength of each grid level for the shader (the fourth is unused);
/// the finest level is hidden when `fine_grid` is off.
fn grid_strengths(fine_grid: bool) -> [f32; 4] {
    let [minor, major, super_level] = GRID_STRENGTHS;
    [if fine_grid { minor } else { 0.0 }, major, super_level, 0.0]
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
    camera: [f32; 4],
    grid: [f32; 4],
    grid_strength: [f32; 4],
    sky_top: [f32; 4],
    sky_bottom: [f32; 4],
}

impl FrameUniforms {
    pub(crate) fn new(params: &FrameParams) -> Self {
        let [minor, major, super_level] = GRID_SPACINGS;
        let p = params.camera_position;
        Self {
            view_projection: params.view_projection.to_cols_array_2d(),
            camera: [p.x, p.y, p.z, 1.0],
            grid: [minor, major, super_level, GRID_HALF_EXTENT],
            grid_strength: grid_strengths(params.fine_grid),
            sky_top: linear(params.sky_top),
            sky_bottom: linear(params.sky_bottom),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniforms_match_the_shader_layout() {
        // mat4x4<f32> + five vec4<f32>, no padding.
        assert_eq!(std::mem::size_of::<FrameUniforms>(), 144);
        assert_eq!(std::mem::size_of::<FrameUniforms>() % 16, 0);
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
            sky_top: [1.0, 0.5, 0.0],
            sky_bottom: [0.0, 0.0, 0.0],
            fine_grid: true,
        });
        assert_eq!(u.camera, [1.0, 2.0, 3.0, 1.0]);
        assert_eq!(u.grid[3], GRID_HALF_EXTENT);
        assert_eq!(u.sky_top[0], 1.0);
        assert!((u.sky_top[1] - 0.214).abs() < 0.001);
        assert_eq!(u.view_projection, Mat4::IDENTITY.to_cols_array_2d());
        assert_eq!(u.grid_strength[0], GRID_STRENGTHS[0]);
    }

    #[test]
    fn fine_grid_off_hides_only_the_finest_lines() {
        assert_eq!(
            grid_strengths(false),
            [0.0, GRID_STRENGTHS[1], GRID_STRENGTHS[2], 0.0]
        );
        assert_eq!(
            grid_strengths(true),
            [GRID_STRENGTHS[0], GRID_STRENGTHS[1], GRID_STRENGTHS[2], 0.0]
        );
    }

    #[test]
    fn coarser_grid_levels_are_stronger() {
        let [a, b, c] = GRID_STRENGTHS;
        assert!(0.0 < a && a < b && b < c && c < 0.5, "a quiet grid");
    }
}
