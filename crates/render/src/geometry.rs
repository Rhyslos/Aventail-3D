//! Geometry as the GPU reads it: coloured lines, shaded faces, and the
//! fixed pieces every viewport shows (the ground axes, the test cube).

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// One end of a line, as the line shader reads it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct LineVertex {
    /// Position in centimetres.
    pub position: [f32; 3],
    /// Linear-space colour with alpha.
    pub color: [f32; 4],
}

impl LineVertex {
    /// Vertex layout for the line pipeline.
    pub(crate) const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
    };
}

/// One corner of a face triangle, as the face shader reads it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct FaceVertex {
    /// Position in centimetres.
    pub position: [f32; 3],
    /// The face's outward direction; unit length.
    pub normal: [f32; 3],
    /// Linear-space colour with alpha.
    pub color: [f32; 4],
}

impl FaceVertex {
    /// Vertex layout for the face pipeline.
    pub(crate) const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<FaceVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4],
    };
}

/// The X axis on the ground, in red (linear space, partly see-through so it
/// stays calm, as in the mockups).
pub const X_AXIS_COLOR: [f32; 4] = [0.80, 0.12, 0.10, 0.55];
/// The Y axis on the ground, in blue.
pub const Y_AXIS_COLOR: [f32; 4] = [0.10, 0.32, 0.85, 0.55];
/// Surface colour of the test cube, linear space (a light, neutral grey).
pub const CUBE_COLOR: [f32; 4] = [0.56, 0.57, 0.585, 1.0];
/// Colour of shape edges, linear space: a faint white line.
pub const EDGE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.10];
/// Edge length of the test cube, in centimetres (2 m, as Cinema 4D's
/// default cube).
pub const TEST_CUBE_SIZE: f32 = 200.0;

fn line(from: Vec3, to: Vec3, color: [f32; 4]) -> [LineVertex; 2] {
    [
        LineVertex {
            position: from.to_array(),
            color,
        },
        LineVertex {
            position: to.to_array(),
            color,
        },
    ]
}

/// Length of each piece the ground axes are drawn in, in centimetres.
///
/// One long line from one side of the grid to the other often passes
/// behind the camera, and some graphics drivers then drop it entirely;
/// short pieces are always drawn, and fade smoothly with distance.
pub const AXIS_SEGMENT: f32 = 100.0;

/// The X and Y axes through the origin on the ground, as a line list of
/// short pieces, reaching `half_extent` each way.
pub fn ground_axes(half_extent: f32) -> Vec<LineVertex> {
    let pieces = (half_extent / AXIS_SEGMENT).ceil().max(1.0) as i32;
    let mut lines = Vec::with_capacity(pieces as usize * 8);
    for i in -pieces..pieces {
        let a = (i as f32 * AXIS_SEGMENT).max(-half_extent);
        let b = ((i + 1) as f32 * AXIS_SEGMENT).min(half_extent);
        lines.extend(line(
            Vec3::new(a, 0.0, 0.0),
            Vec3::new(b, 0.0, 0.0),
            X_AXIS_COLOR,
        ));
        lines.extend(line(
            Vec3::new(0.0, a, 0.0),
            Vec3::new(0.0, b, 0.0),
            Y_AXIS_COLOR,
        ));
    }
    lines
}

/// A box's shaded faces and its edges, ready for the GPU.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shape {
    /// Triangles, three vertices each, wound counter-clockwise seen from
    /// outside.
    pub faces: Vec<FaceVertex>,
    /// Edges as a line list.
    pub edges: Vec<LineVertex>,
}

impl Shape {
    /// An axis-aligned box from `min` to `max`.
    pub fn cuboid(min: Vec3, max: Vec3, color: [f32; 4]) -> Self {
        let corner = |i: usize| {
            Vec3::new(
                if i & 1 == 0 { min.x } else { max.x },
                if i & 2 == 0 { min.y } else { max.y },
                if i & 4 == 0 { min.z } else { max.z },
            )
        };
        // Each side as four corners, counter-clockwise seen from outside.
        let sides: [([usize; 4], Vec3); 6] = [
            ([0, 2, 3, 1], Vec3::NEG_Z),
            ([4, 5, 7, 6], Vec3::Z),
            ([0, 1, 5, 4], Vec3::NEG_Y),
            ([2, 6, 7, 3], Vec3::Y),
            ([0, 4, 6, 2], Vec3::NEG_X),
            ([1, 3, 7, 5], Vec3::X),
        ];
        let mut faces = Vec::with_capacity(36);
        for (quad, normal) in sides {
            for i in [0, 1, 2, 0, 2, 3] {
                faces.push(FaceVertex {
                    position: corner(quad[i]).to_array(),
                    normal: normal.to_array(),
                    color,
                });
            }
        }
        let pairs = [
            (0, 1),
            (2, 3),
            (4, 5),
            (6, 7),
            (0, 2),
            (1, 3),
            (4, 6),
            (5, 7),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];
        let edges = pairs
            .iter()
            .flat_map(|&(a, b)| line(corner(a), corner(b), EDGE_COLOR))
            .collect();
        Self { faces, edges }
    }

    /// The test cube shown until real objects exist (Phase 2): 2 m on each
    /// side, standing on the grid at the origin.
    pub fn test_cube() -> Self {
        let half = TEST_CUBE_SIZE / 2.0;
        Self::cuboid(
            Vec3::new(-half, -half, 0.0),
            Vec3::new(half, half, TEST_CUBE_SIZE),
            CUBE_COLOR,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_layouts_match_the_shader() {
        assert_eq!(std::mem::size_of::<LineVertex>(), 28);
        assert_eq!(LineVertex::LAYOUT.attributes[1].offset, 12);
        assert_eq!(std::mem::size_of::<FaceVertex>(), 40);
        assert_eq!(FaceVertex::LAYOUT.attributes[2].offset, 24);
    }

    #[test]
    fn the_axes_lie_on_the_ground_in_short_pieces() {
        let axes = ground_axes(500.0);
        assert!(axes.iter().all(|v| v.position[2] == 0.0));
        for pair in axes.chunks(2) {
            let a = Vec3::from_array(pair[0].position);
            let b = Vec3::from_array(pair[1].position);
            assert!((a - b).length() <= AXIS_SEGMENT + 1e-3);
        }
        let reach = |color: [f32; 4], axis: usize| {
            let values: Vec<f32> = axes
                .iter()
                .filter(|v| v.color == color)
                .map(|v| v.position[axis])
                .collect();
            let min = values.iter().cloned().fold(f32::MAX, f32::min);
            let max = values.iter().cloned().fold(f32::MIN, f32::max);
            (min, max)
        };
        assert_eq!(reach(X_AXIS_COLOR, 0), (-500.0, 500.0));
        assert_eq!(reach(Y_AXIS_COLOR, 1), (-500.0, 500.0));
    }

    #[test]
    fn a_cuboid_has_twelve_triangles_and_twelve_edges() {
        let shape = Shape::test_cube();
        assert_eq!(shape.faces.len(), 36);
        assert_eq!(shape.edges.len(), 24);
    }

    #[test]
    fn faces_wind_outwards() {
        // The cross product of each triangle's edges points along its normal,
        // so back-face culling hides the inside, never the outside.
        let shape = Shape::test_cube();
        for tri in shape.faces.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from_array(tri[i].position));
            let winding = (b - a).cross(c - a).normalize();
            let normal = Vec3::from_array(tri[0].normal);
            assert!((winding - normal).length() < 1e-5, "{winding} vs {normal}");
        }
    }

    #[test]
    fn the_test_cube_stands_on_the_grid() {
        let shape = Shape::test_cube();
        let zs: Vec<f32> = shape.faces.iter().map(|v| v.position[2]).collect();
        let min = zs.iter().cloned().fold(f32::MAX, f32::min);
        let max = zs.iter().cloned().fold(f32::MIN, f32::max);
        assert_eq!(min, 0.0);
        assert_eq!(max, TEST_CUBE_SIZE);
    }

    #[test]
    fn every_edge_has_length() {
        let shape = Shape::test_cube();
        for pair in shape.edges.chunks(2) {
            let a = Vec3::from_array(pair[0].position);
            let b = Vec3::from_array(pair[1].position);
            assert!((a - b).length() > 1.0);
        }
    }
}
