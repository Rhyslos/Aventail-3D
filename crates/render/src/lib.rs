//! The viewport renderer.
//!
//! Draws each viewport with wgpu into its own offscreen image: a calm
//! backdrop with a horizon, a finite ground grid on the X–Y plane (Z is
//! up) with the X, Y and Z axes, and shapes with soft studio shading and faint edges.
//! Each object's mesh is kept on the GPU in its own space ([`MeshGeometry`],
//! [`ViewportRenderer::set_mesh`]) and placed in the world as it is drawn
//! ([`MeshDraw`]).
//!
//! This crate knows nothing about egui or windows: it receives a
//! [`FrameParams`] (camera matrices, background colours) and produces an
//! image in a [`ViewportTarget`].

mod frame;
mod geometry;
mod mesh_draw;
mod pick;
mod pipelines;
mod readback;
mod renderer;

pub use frame::{
    FrameParams, GRID_HALF_EXTENT, GRID_MIN_REACH, GRID_REACH_PER_DISTANCE, GRID_SPACINGS,
    GRID_STRENGTHS, GROUND_PLANE, HALF_METRE_STRENGTH, srgb_to_linear,
};
pub use geometry::{
    AXIS_SEGMENT, CUBE_COLOR, EDGE_COLOR, FaceVertex, LineVertex, MeshInstance, Overlay, Shape,
    Sprite, TEST_CUBE_SIZE, X_AXIS_COLOR, Y_AXIS_COLOR, Z_AXIS_COLOR, Z_AXIS_HEIGHT, ground_axes,
};
pub use mesh_draw::{MeshDraw, MeshGeometry, MeshVertex};
pub use pick::{PickRequest, PickResult};
pub use readback::read_pixels;
pub use renderer::{
    COLOR_FORMAT, DEPTH_FORMAT, DISPLAY_FORMAT, FACING_FORMAT, SHADER_SOURCE, ViewportRenderer,
    ViewportTarget, best_sample_count,
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
                "cavity_fs",
                "cavity_vs",
                "face_fs",
                "face_vs",
                "grid_fs",
                "grid_vs",
                "line_fs",
                "line_vs",
                "mesh_face_fs",
                "mesh_face_vs",
                "mesh_line_vs",
                "mesh_point_vs",
                "sky_fs",
                "sky_vs",
                "sprite_fs",
                "sprite_vs"
            ]
        );
    }
}
