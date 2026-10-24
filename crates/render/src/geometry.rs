//! Geometry as the GPU reads it: coloured lines, shaded faces, and the
//! fixed pieces every viewport shows (the ground axes, the test cube).

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};
use mesh::Mesh;

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

/// One overlay sprite: a point drawn as a small dot, or a thick line, both
/// a fixed number of pixels wide whatever the distance (see [`Overlay`]).
/// The GPU makes its six corners (two triangles), so each costs one record.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Sprite {
    /// The point, or the line's first end, in the world (centimetres).
    pub a: [f32; 3],
    /// The line's other end; the same as `a` for a point.
    pub b: [f32; 3],
    /// Width in pixels.
    pub width: f32,
    /// Linear-space colour with alpha.
    pub color: [f32; 4],
}

impl Sprite {
    /// Layout for the sprite pipeline: one record per sprite.
    pub(crate) const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Sprite>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32,
            3 => Float32x4
        ],
    };
}

/// What the Points, Edges and Polygons modes draw over the scene: points
/// as small squares, lines a few pixels thick, and see-through tinted
/// triangles. Kept apart from the [`Shape`] so moving the mouse only
/// rebuilds this small layer. Everything stays hidden behind surfaces in
/// front of it, except the lines in [`Overlay::on_top`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Overlay {
    /// Points and thick lines.
    pub sprites: Vec<Sprite>,
    /// Thick lines drawn over everything, never hidden behind surfaces
    /// (decision 0158: a deformer's axis runs through its shape).
    pub on_top: Vec<Sprite>,
    /// See-through triangles laid on faces.
    pub tints: Vec<LineVertex>,
}

impl Overlay {
    /// Adds a point at `at`, drawn as a dot `size` pixels across.
    pub fn point(&mut self, at: Vec3, size: f32, color: [f32; 4]) {
        self.sprites.push(Sprite {
            a: at.to_array(),
            b: at.to_array(),
            width: size,
            color,
        });
    }

    /// Adds a line from `a` to `b`, `width` pixels thick. A line with no
    /// length is drawn as a point.
    pub fn line(&mut self, a: Vec3, b: Vec3, width: f32, color: [f32; 4]) {
        self.sprites.push(Sprite {
            a: a.to_array(),
            b: b.to_array(),
            width,
            color,
        });
    }

    /// Adds a line from `a` to `b`, `width` pixels thick, drawn over
    /// everything: never hidden behind a surface.
    pub fn line_on_top(&mut self, a: Vec3, b: Vec3, width: f32, color: [f32; 4]) {
        self.on_top.push(Sprite {
            a: a.to_array(),
            b: b.to_array(),
            width,
            color,
        });
    }

    /// Adds a see-through triangle.
    pub fn triangle(&mut self, corners: [Vec3; 3], color: [f32; 4]) {
        for c in corners {
            self.tints.push(LineVertex {
                position: c.to_array(),
                color,
            });
        }
    }

    /// Whether there is nothing to draw.
    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty() && self.tints.is_empty() && self.on_top.is_empty()
    }
}

/// The X axis on the ground, in red (linear space, partly see-through so it
/// stays calm, as in the mockups).
pub const X_AXIS_COLOR: [f32; 4] = [0.80, 0.12, 0.10, 0.55];
/// The Y axis on the ground, in blue.
pub const Y_AXIS_COLOR: [f32; 4] = [0.10, 0.32, 0.85, 0.55];
/// The Z axis, straight up from the origin, in green.
pub const Z_AXIS_COLOR: [f32; 4] = [0.18, 0.62, 0.10, 0.6];
/// How high the Z axis line reaches, in centimetres (100 m: it reads as
/// endless from any normal view).
pub const Z_AXIS_HEIGHT: f32 = 10_000.0;
/// Surface colour of the test cube, linear space (a neutral mid grey, so
/// light and shade both read clearly).
pub const CUBE_COLOR: [f32; 4] = [0.5, 0.5, 0.51, 1.0];
/// Colour of shape edges, linear space: a faint white line.
pub const EDGE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.10];

/// One object's mesh, placed in the world, for [`Shape::from_meshes`].
#[derive(Debug, Clone, Copy)]
pub struct MeshInstance<'a> {
    /// The shape, in its own space.
    pub mesh: &'a Mesh,
    /// Where it is in the world.
    pub transform: Mat4,
    /// Whether it is selected (its edges are drawn in the selection colour).
    pub selected: bool,
}
/// Edge length of the test cube, in centimetres (2 m).
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

/// The X and Y axes through the origin on the ground, reaching
/// `half_extent` each way, and the Z axis straight up from the origin to
/// [`Z_AXIS_HEIGHT`], as a line list of short pieces.
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
    let up = (Z_AXIS_HEIGHT / AXIS_SEGMENT).ceil() as i32;
    for i in 0..up {
        let a = i as f32 * AXIS_SEGMENT;
        let b = ((i + 1) as f32 * AXIS_SEGMENT).min(Z_AXIS_HEIGHT);
        lines.extend(line(
            Vec3::new(0.0, 0.0, a),
            Vec3::new(0.0, 0.0, b),
            Z_AXIS_COLOR,
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

    /// Every object's faces and edges, placed in the world, ready to draw.
    /// Shading follows each mesh's hard and soft edges; selected objects'
    /// edges are drawn in `selected_edges` (linear-space colour with alpha;
    /// the interface's accent colour).
    pub fn from_meshes(instances: &[MeshInstance<'_>], selected_edges: [f32; 4]) -> Self {
        let mut shape = Self::default();
        for instance in instances {
            let matrix = instance.transform;
            if !matrix.is_finite() {
                continue;
            }
            let normal_matrix = Mat3::from_mat4(matrix).inverse().transpose();
            // A mirrored placement turns faces inside out; swap two corners
            // so they still face outward.
            let mirrored = matrix.determinant() < 0.0;
            for triangle in mesh::shaded_triangles(instance.mesh) {
                let mut order = [0, 1, 2];
                if mirrored {
                    order.swap(1, 2);
                }
                for i in order {
                    let normal = (normal_matrix * triangle.normals[i])
                        .try_normalize()
                        .unwrap_or(Vec3::Z);
                    shape.faces.push(FaceVertex {
                        position: matrix.transform_point3(triangle.positions[i]).to_array(),
                        normal: normal.to_array(),
                        color: CUBE_COLOR,
                    });
                }
            }
            let color = if instance.selected {
                selected_edges
            } else {
                EDGE_COLOR
            };
            for edge in instance.mesh.edges() {
                let (a, b) = edge.ends();
                if let (Some(a), Some(b)) = (instance.mesh.position(a), instance.mesh.position(b)) {
                    shape.edges.extend(line(
                        matrix.transform_point3(a),
                        matrix.transform_point3(b),
                        color,
                    ));
                }
            }
        }
        shape
    }

    /// The 2 m test cube the renderer starts with (and its tests use): 2 m
    /// on each side, standing on the grid at the origin.
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
        assert!(
            axes.iter()
                .filter(|v| v.color != Z_AXIS_COLOR)
                .all(|v| v.position[2] == 0.0)
        );
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
        assert_eq!(reach(Z_AXIS_COLOR, 2), (0.0, Z_AXIS_HEIGHT));
    }

    #[test]
    fn the_z_axis_stands_straight_up_from_the_origin() {
        let z: Vec<_> = ground_axes(500.0)
            .into_iter()
            .filter(|v| v.color == Z_AXIS_COLOR)
            .collect();
        assert!(!z.is_empty());
        assert!(
            z.iter()
                .all(|v| v.position[0] == 0.0 && v.position[1] == 0.0)
        );
        assert!(
            z.iter().all(|v| v.position[2] >= 0.0),
            "nothing below ground"
        );
    }

    #[test]
    fn overlay_points_and_lines_are_one_record_each() {
        let mut overlay = Overlay::default();
        assert!(overlay.is_empty());
        overlay.point(Vec3::ZERO, 6.0, [1.0; 4]);
        overlay.line(Vec3::ZERO, Vec3::X, 3.0, [1.0; 4]);
        assert_eq!(overlay.sprites.len(), 2);
        assert_eq!(overlay.sprites[0].a, overlay.sprites[0].b, "a point");
        assert_ne!(overlay.sprites[1].a, overlay.sprites[1].b, "a line");
        // A line with no length is drawn as a point.
        overlay.line(Vec3::Y, Vec3::Y, 3.0, [1.0; 4]);
        assert_eq!(overlay.sprites.len(), 3);
        assert_eq!(overlay.sprites[2].a, overlay.sprites[2].b);
        overlay.triangle([Vec3::ZERO, Vec3::X, Vec3::Y], [1.0; 4]);
        assert_eq!(overlay.tints.len(), 3);
        assert_eq!(std::mem::size_of::<Sprite>(), 44);
        assert_eq!(Sprite::LAYOUT.attributes[3].offset, 28);
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

    /// A selection colour for the tests.
    const RED: [f32; 4] = [0.8, 0.1, 0.1, 0.9];

    #[test]
    fn meshes_become_faces_and_edges_in_the_world() {
        let cube = mesh::Primitive::Cube(Default::default()).mesh();
        let moved = Mat4::from_translation(Vec3::new(500.0, 0.0, 0.0));
        let shape = Shape::from_meshes(
            &[MeshInstance {
                mesh: &cube,
                transform: moved,
                selected: false,
            }],
            RED,
        );
        assert_eq!(
            shape.faces.len(),
            6 * 2 * 3,
            "six quads, two triangles each"
        );
        assert_eq!(shape.edges.len(), 12 * 2);
        let xs: Vec<f32> = shape.faces.iter().map(|v| v.position[0]).collect();
        assert!(
            xs.iter().all(|x| (400.0..=600.0).contains(x)),
            "moved along X"
        );
        // The same as the old test cube, apart from the move.
        let test = Shape::test_cube();
        assert_eq!(test.faces.len(), shape.faces.len());
        assert!(shape.edges.iter().all(|e| e.color == EDGE_COLOR));
    }

    #[test]
    fn selected_meshes_have_coloured_edges_and_several_meshes_add_up() {
        let cube = mesh::Primitive::Cube(Default::default()).mesh();
        let sphere = mesh::Primitive::Sphere(Default::default()).mesh();
        let shape = Shape::from_meshes(
            &[
                MeshInstance {
                    mesh: &cube,
                    transform: Mat4::IDENTITY,
                    selected: true,
                },
                MeshInstance {
                    mesh: &sphere,
                    transform: Mat4::IDENTITY,
                    selected: false,
                },
            ],
            RED,
        );
        let coloured = shape.edges.iter().filter(|e| e.color == RED).count();
        assert_eq!(
            coloured, 24,
            "the cube's twelve edges, in the selection colour"
        );
        assert!(shape.faces.len() > 36);
        assert_eq!(Shape::from_meshes(&[], RED), Shape::default());
    }

    #[test]
    fn mirrored_and_broken_placements_are_handled() {
        let cube = mesh::Primitive::Cube(Default::default()).mesh();
        let mirrored = Shape::from_meshes(
            &[MeshInstance {
                mesh: &cube,
                transform: Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)),
                selected: false,
            }],
            RED,
        );
        for t in mirrored.faces.chunks(3) {
            let p = |i: usize| Vec3::from(t[i].position);
            let winding = (p(1) - p(0)).cross(p(2) - p(0));
            assert!(
                winding.dot(Vec3::from(t[0].normal)) > 0.0,
                "still facing out"
            );
        }
        let broken = Shape::from_meshes(
            &[MeshInstance {
                mesh: &cube,
                transform: Mat4::from_scale(Vec3::splat(f32::NAN)),
                selected: false,
            }],
            RED,
        );
        assert_eq!(broken, Shape::default(), "skipped");
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
