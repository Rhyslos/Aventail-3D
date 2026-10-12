//! Shapes kept on the GPU one object at a time, each in its own space and
//! placed in the world when drawn ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)).
//!
//! A [`MeshGeometry`] is a mesh turned into what the GPU draws: one vertex
//! per smooth group at each point (so soft edges share a vertex and hard
//! edges split it), triangles and edges as lists of vertex numbers, and
//! enough bookkeeping to update only the vertices around points that
//! moved ([`MeshGeometry::moved`]). Selecting, moving an object or
//! changing what is selected never rebuilds it: those only change how it
//! is drawn ([`MeshDraw`]).

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};
use mesh::{Mesh, VertexId};
use std::ops::Range;

/// One vertex of a mesh on the GPU, in the mesh's own space.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct MeshVertex {
    /// Position in centimetres.
    pub position: [f32; 3],
    /// The shading direction; unit length.
    pub normal: [f32; 3],
}

impl MeshVertex {
    /// Vertex layout for drawing faces and edges.
    pub(crate) const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
    };

    /// The same vertices read once per drawn point (each a small square).
    pub(crate) const POINT_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
    };
}

/// A mesh ready to upload: vertices, triangles and edges.
#[derive(Debug, Clone, Default)]
pub struct MeshGeometry {
    /// The vertices.
    pub vertices: Vec<MeshVertex>,
    /// Triangles, three vertex numbers each.
    pub triangles: Vec<u32>,
    /// Edges, two vertex numbers each.
    pub edges: Vec<u32>,
    /// The average edge length, in the mesh's own space: how dense its
    /// wireframe is, so a dense one can fade when seen from afar.
    pub edge_length: f32,
    /// The smooth group of each face corner (see [`mesh::smooth_groups`]).
    groups: Vec<u32>,
    /// The vertex of each face corner.
    corner_vertex: Vec<u32>,
    /// One vertex at each point, by point id number (`u32::MAX` for none).
    point_vertex: Vec<u32>,
}

/// Merges sorted vertex numbers into ranges, joining ranges with small
/// gaps (writing a few unchanged vertices is cheaper than another upload).
fn ranges_of(mut indices: Vec<u32>) -> Vec<Range<usize>> {
    const GAP: u32 = 64;
    indices.sort_unstable();
    indices.dedup();
    let mut out: Vec<Range<usize>> = Vec::new();
    for i in indices {
        let i = i as usize;
        match out.last_mut() {
            Some(last) if i <= last.end + GAP as usize => last.end = i + 1,
            _ => out.push(i..i + 1),
        }
    }
    out
}

impl MeshGeometry {
    /// The geometry of `mesh`.
    pub fn new(mesh: &Mesh) -> Self {
        let (groups, normals) = mesh::corner_shading(mesh);
        let (starts, corners) = mesh.face_layout();
        let mut vertices = Vec::new();
        // The vertex made for each group's first corner.
        let mut group_vertex = vec![u32::MAX; corners.len()];
        let mut corner_vertex = vec![0u32; corners.len()];
        let mut point_vertex = vec![u32::MAX; mesh.point_slot_count()];
        for k in 0..corners.len() {
            let rep = groups[k] as usize;
            if group_vertex[rep] == u32::MAX {
                group_vertex[rep] = vertices.len() as u32;
                let position = mesh.position(corners[k]).unwrap_or(Vec3::ZERO);
                vertices.push(MeshVertex {
                    position: position.to_array(),
                    normal: normals[k].to_array(),
                });
            }
            corner_vertex[k] = group_vertex[rep];
            let slot = &mut point_vertex[corners[k].get() as usize];
            if *slot == u32::MAX {
                *slot = group_vertex[rep];
            }
        }
        // Points no face uses still get a vertex, so they can be shown.
        for (v, p) in mesh.vertices() {
            let slot = &mut point_vertex[v.get() as usize];
            if *slot == u32::MAX {
                *slot = vertices.len() as u32;
                vertices.push(MeshVertex {
                    position: p.to_array(),
                    normal: [0.0, 0.0, 1.0],
                });
            }
        }
        let mut triangles = Vec::with_capacity(corners.len() * 3);
        for pair in starts.windows(2) {
            let (a, b) = (pair[0] as usize, pair[1] as usize);
            for i in a + 1..b.saturating_sub(1) {
                triangles.extend([corner_vertex[a], corner_vertex[i], corner_vertex[i + 1]]);
            }
        }
        let mut edges = Vec::with_capacity(mesh.edges().len() * 2);
        let mut total_length = 0.0f64;
        for edge in mesh.edges() {
            let (a, b) = edge.ends();
            let (va, vb) = (
                point_vertex[a.get() as usize],
                point_vertex[b.get() as usize],
            );
            if va != u32::MAX && vb != u32::MAX {
                edges.extend([va, vb]);
                let (pa, pb) = (
                    vertices[va as usize].position,
                    vertices[vb as usize].position,
                );
                total_length += f64::from(Vec3::from(pa).distance(Vec3::from(pb)));
            }
        }
        let edge_length = (total_length / (edges.len() / 2).max(1) as f64) as f32;
        Self {
            vertices,
            triangles,
            edges,
            edge_length,
            groups,
            corner_vertex,
            point_vertex,
        }
    }

    /// Works out the normals of the smooth groups at `points` again, and
    /// returns the vertices that changed.
    fn reshade(&mut self, mesh: &Mesh, points: &[VertexId]) -> Vec<u32> {
        let mut touched = Vec::new();
        for v in points {
            for (rep, normal) in mesh::group_normals_at(mesh, &self.groups, *v) {
                let vertex = self.corner_vertex[rep as usize];
                self.vertices[vertex as usize].normal = normal.to_array();
                touched.push(vertex);
            }
        }
        touched
    }

    /// Brings the geometry up to date after `moved` points of the mesh it
    /// was made from moved to where they are in `mesh` (which must have
    /// the same faces: [`Mesh::same_structure`]). Only the vertices around
    /// them are worked out again; returns the ranges of vertices to upload.
    pub fn moved(&mut self, mesh: &Mesh, moved: &[VertexId]) -> Vec<Range<usize>> {
        let topology = mesh.topology();
        let mut touched = Vec::new();
        let (_, corners) = mesh.face_layout();
        for v in moved {
            let Some(p) = mesh.position(*v) else {
                continue;
            };
            let at = |k: usize| corners[k] == *v;
            // Every vertex of the point: one per smooth group around it.
            for face in topology.vertex_faces(*v) {
                let (starts, _) = mesh.face_layout();
                let f = face.get() as usize;
                for k in starts[f] as usize..starts[f + 1] as usize {
                    if at(k) {
                        let vertex = self.corner_vertex[k];
                        self.vertices[vertex as usize].position = p.to_array();
                        touched.push(vertex);
                    }
                }
            }
            if let Some(&vertex) = self.point_vertex.get(v.get() as usize)
                && vertex != u32::MAX
            {
                self.vertices[vertex as usize].position = p.to_array();
                touched.push(vertex);
            }
        }
        let reach = mesh::shading_reach(mesh, moved);
        touched.extend(self.reshade(mesh, &reach));
        ranges_of(touched)
    }

    /// Lets go of the triangle and edge lists once they are on the GPU:
    /// moving points never changes them, and on a heavy shape they take
    /// a lot of memory.
    pub fn forget_indices(&mut self) {
        self.triangles = Vec::new();
        self.edges = Vec::new();
    }
}

/// How one object's mesh is drawn this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshDraw {
    /// Which mesh (as given to [`crate::ViewportRenderer::set_mesh`]).
    pub key: u64,
    /// Where it is in the world.
    pub model: Mat4,
    /// Its edges' colour, linear space with alpha.
    pub edge_color: [f32; 4],
    /// Whether to draw its edges at all (they can be left out when they
    /// would be too dense to see).
    pub edges: bool,
    /// Whether to show every point as a small dot, and how: colour (linear,
    /// with alpha) and size in pixels.
    pub points: Option<([f32; 4], f32)>,
    /// How faint a dense wireframe may fade, seen from afar (0: away
    /// entirely; a selected object keeps a little of its colour).
    pub faintest: f32,
}

/// What the shaders read about one drawn object.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct ObjectUniforms {
    model: [[f32; 4]; 4],
    /// The model matrix's inverse transposed, for normals.
    normal: [[f32; 4]; 4],
    edge_color: [f32; 4],
    point_color: [f32; 4],
    /// x: point size in pixels; y: -1 when the placement mirrors the
    /// shape (turning its faces inside out), else 1; z: the average edge
    /// length in the world (for fading dense wireframes); w: how faint
    /// they may fade.
    options: [f32; 4],
}

impl ObjectUniforms {
    pub(crate) fn new(draw: &MeshDraw, edge_length: f32) -> Self {
        let scale = (draw.model.x_axis.truncate().length()
            + draw.model.y_axis.truncate().length()
            + draw.model.z_axis.truncate().length())
            / 3.0;
        let normal = Mat4::from_mat3(Mat3::from_mat4(draw.model).inverse().transpose());
        let (point_color, size) = draw.points.unwrap_or(([0.0; 4], 0.0));
        Self {
            model: draw.model.to_cols_array_2d(),
            normal: normal.to_cols_array_2d(),
            edge_color: draw.edge_color,
            point_color,
            options: [
                size,
                if draw.model.determinant() < 0.0 {
                    -1.0
                } else {
                    1.0
                },
                edge_length * scale,
                draw.faintest,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesh::PrimitiveKind;

    #[test]
    fn soft_edges_share_vertices_and_hard_edges_split_them() {
        let sphere = PrimitiveKind::Sphere.standard().mesh();
        let geometry = MeshGeometry::new(&sphere);
        // A smooth sphere: about one vertex per point (the poles' n-gons
        // and seams aside), far fewer than one per corner.
        let corners = sphere.face_layout().1.len();
        assert!(geometry.vertices.len() < corners / 2);
        let cube = PrimitiveKind::Cube.standard().mesh();
        let geometry = MeshGeometry::new(&cube);
        assert_eq!(geometry.vertices.len(), 24, "every corner its own side");
        assert_eq!(geometry.triangles.len(), 12 * 3);
        assert_eq!(geometry.edges.len(), 12 * 2);
        assert!((geometry.edge_length - 100.0).abs() < 1e-3, "a 1 m cube");
        // Each side's normals point straight out.
        for t in geometry.triangles.chunks(3) {
            let [a, b, c] =
                [t[0], t[1], t[2]].map(|i| Vec3::from(geometry.vertices[i as usize].position));
            let face = (b - a).cross(c - a).normalize();
            let n = Vec3::from(geometry.vertices[t[0] as usize].normal);
            assert!((face - n).length() < 1e-5);
        }
    }

    #[test]
    fn moving_points_updates_only_what_is_around_them() {
        let before = mesh::samples::heavy_ball(2000, 100.0);
        let mut geometry = MeshGeometry::new(&before);
        let mut after = before.clone();
        let moved: Vec<VertexId> = before
            .vertices()
            .map(|(v, _)| v)
            .skip(500)
            .take(3)
            .collect();
        for v in &moved {
            let p = after.position(*v).unwrap();
            after.set_position(*v, p + Vec3::Z * 10.0);
        }
        assert!(after.same_structure(&before));
        let ranges = geometry.moved(&after, &moved);
        let updated: usize = ranges.iter().map(|r| r.len()).sum();
        assert!(
            updated > 0 && updated < geometry.vertices.len() / 4,
            "{updated}"
        );
        // The result is the same as building afresh.
        let fresh = MeshGeometry::new(&after);
        assert_eq!(fresh.vertices.len(), geometry.vertices.len());
        for (a, b) in fresh.vertices.iter().zip(&geometry.vertices) {
            assert!((Vec3::from(a.position) - Vec3::from(b.position)).length() < 1e-4);
            assert!((Vec3::from(a.normal) - Vec3::from(b.normal)).length() < 1e-4);
        }
    }

    #[test]
    fn loose_points_and_empty_meshes_are_fine() {
        let mut mesh = PrimitiveKind::Plane.standard().mesh();
        mesh.add_vertex(Vec3::splat(5.0)).unwrap();
        let geometry = MeshGeometry::new(&mesh);
        assert!(
            geometry
                .vertices
                .iter()
                .any(|v| v.position == [5.0, 5.0, 5.0]),
            "the loose point can be shown"
        );
        let empty = MeshGeometry::new(&Mesh::new());
        assert!(empty.vertices.is_empty() && empty.triangles.is_empty());
    }

    #[test]
    fn ranges_join_close_vertices() {
        assert_eq!(ranges_of(vec![5, 3, 4, 200, 3]), vec![3..6, 200..201]);
        assert_eq!(ranges_of(vec![1, 60]), vec![1..61]);
        assert!(ranges_of(Vec::new()).is_empty());
    }

    #[test]
    fn mirrored_placements_are_marked() {
        let draw = |model| MeshDraw {
            key: 1,
            model,
            edge_color: [1.0; 4],
            edges: true,
            points: Some(([1.0; 4], 6.0)),
            faintest: 0.25,
        };
        let plain = ObjectUniforms::new(&draw(Mat4::IDENTITY), 10.0);
        assert_eq!(plain.options, [6.0, 1.0, 10.0, 0.25]);
        let mirrored =
            ObjectUniforms::new(&draw(Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0))), 10.0);
        assert_eq!(mirrored.options[1], -1.0);
        let doubled = ObjectUniforms::new(&draw(Mat4::from_scale(Vec3::splat(2.0))), 10.0);
        assert_eq!(
            doubled.options[2], 20.0,
            "edges look longer on a scaled-up shape"
        );
    }
}

/// Part of the heavy-mesh check (DESIGN.md §4.8): turning a shape of
/// millions of polygons into GPU geometry, and updating it after a move.
/// Run with `tools/heavy/heavy-check.sh`.
#[cfg(test)]
mod heavy_check {
    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore = "heavy: run with tools/heavy/heavy-check.sh"]
    fn heavy_geometry_check() {
        let polygons: usize = std::env::var("HEAVY_POLYGONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3_000_000);
        let mesh = mesh::samples::heavy_ball(polygons, 100.0);
        let start = Instant::now();
        let mut geometry = MeshGeometry::new(&mesh);
        let built = start.elapsed();
        let mut after = mesh.clone();
        let moved: Vec<VertexId> = mesh.faces().nth(polygons / 2).unwrap().1.to_vec();
        for v in &moved {
            let p = after.position(*v).unwrap();
            after.set_position(*v, p + Vec3::Z * 5.0);
        }
        let start = Instant::now();
        let found = after.moved_since(&mesh).expect("same faces");
        let ranges = geometry.moved(&after, &found);
        let updated = start.elapsed();
        let bytes: usize = geometry.vertices.len() * std::mem::size_of::<MeshVertex>()
            + (geometry.triangles.len() + geometry.edges.len()) * 4;
        println!();
        println!("Heavy-mesh check, drawing: {polygons} polygons");
        println!(
            "{:<58} {:>8.1}ms {:>10} {:>8}ms  {}",
            "Turn it into GPU geometry (once per new shape)",
            built.as_secs_f64() * 1000.0,
            "",
            3000,
            if built.as_millis() <= 3000 {
                "ok"
            } else {
                "SLOW"
            }
        );
        println!(
            "{:<58} {:>8.1}ms {:>10} {:>8}ms  {}",
            "Update it after moving a polygon (each drag frame)",
            updated.as_secs_f64() * 1000.0,
            "",
            10,
            if updated.as_millis() <= 10 {
                "ok"
            } else {
                "SLOW"
            }
        );
        println!(
            "GPU memory for it: about {} MB; {} vertices updated",
            bytes / 1_000_000,
            ranges.iter().map(|r| r.len()).sum::<usize>()
        );
        assert!(built.as_millis() <= 3000 && updated.as_millis() <= 10);
    }
}
