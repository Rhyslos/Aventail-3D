//! The viewport renderer.
//!
//! Draws each viewport with wgpu into its own offscreen image: the calm
//! background gradient, the ground grid on the X–Y plane (Z is up) with
//! the X and Y axes, and shapes with soft studio shading and faint edges.
//! For now the only shape is a test cube; objects arrive in Phase 2.
//!
//! This crate knows nothing about egui or windows: it receives a
//! [`FrameParams`] (camera matrices, background colours) and produces an
//! image in a [`ViewportTarget`].

mod frame;
mod geometry;
mod pipelines;
mod readback;
mod renderer;

pub use frame::{FrameParams, GRID_HALF_EXTENT, GRID_SPACINGS, GRID_STRENGTHS, srgb_to_linear};
pub use geometry::{
    AXIS_SEGMENT, CUBE_COLOR, EDGE_COLOR, FaceVertex, LineVertex, Shape, TEST_CUBE_SIZE,
    X_AXIS_COLOR, Y_AXIS_COLOR, ground_axes,
};
pub use readback::read_pixels;
pub use renderer::{
    COLOR_FORMAT, DEPTH_FORMAT, DISPLAY_FORMAT, SHADER_SOURCE, ViewportRenderer, ViewportTarget,
    best_sample_count,
};

#[cfg(test)]
mod shader_tests {
    use super::SHADER_SOURCE;
    use wgpu::naga;

    /// Parses and validates the WGSL with the same compiler wgpu uses, so
    /// shader mistakes fail the tests even on machines without a GPU.
    #[test]
    fn shader_is_valid() {
        let module = naga::front::wgsl::parse_str(SHADER_SOURCE)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(SHADER_SOURCE)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(SHADER_SOURCE)));
    }

    #[test]
    fn shader_has_the_expected_entry_points() {
        let module = naga::front::wgsl::parse_str(SHADER_SOURCE).unwrap();
        let mut names: Vec<_> = module
            .entry_points
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "face_fs", "face_vs", "grid_fs", "grid_vs", "line_fs", "line_vs", "sky_fs",
                "sky_vs"
            ]
        );
    }
}
