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
use std::collections::HashMap;
use std::ops::Range;

/// One vertex of a mesh on the GPU, in the mesh's own space.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct MeshVertex {
    /// Position in centimetres.
    pub position: [f32; 3],
    /// The shading direction; unit length.
    pub normal: [f32; 3],
    /// The material slot of the polygons it belongs to: its colour comes
    /// from the object's palette ([`MeshDraw::palette`]).
    pub material: u32,
    /// Its UV (decision 0102): vertices split where the UVs do.
    pub uv: [f32; 2],
}

impl MeshVertex {
    /// Vertex layout for drawing faces and edges.
    pub(crate) const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Uint32, 3 => Float32x2],
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
    /// The further vertices of smooth groups that span polygons of more
    /// than one material (one vertex per material), by the group's first
    /// corner. Usually empty.
    split_vertices: HashMap<u32, Vec<u32>>,
    /// What decided where the shading breaks.
    hardness: mesh::Hardness,
    /// The mesh's texel density as a length ratio (texture widths per
    /// centimetre), 0 without UVs: what the Stretch view compares each
    /// spot with.
    pub uv_density: f32,
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
    /// The geometry of `mesh`, shaded by its hard edges.
    pub fn new(mesh: &Mesh) -> Self {
        Self::shaded(mesh, mesh::Hardness::Marked)
    }

    /// The geometry of `mesh`, with the shading breaking where `hardness`
    /// says (by angle while Automatic hard edges is on,
    /// [decision 0064](../../../docs/decisions/0064-automatic-hard-edges.md)).
    pub fn shaded(mesh: &Mesh, hardness: mesh::Hardness) -> Self {
        let (groups, normals) = mesh::corner_shading_with(mesh, hardness);
        let (starts, corners) = mesh.face_layout();
        let has_materials = mesh.attributes().any(|(name, domain, _)| {
            name == mesh::material::MATERIAL && domain == mesh::Domain::Face
        });
        let uvs = mesh::uvs::Uvs::of(mesh);
        let uv_at = |k: usize| uvs.as_ref().map_or([0.0; 2], |u| u.at(k).to_array());
        let mut vertices = Vec::new();
        // The vertex made for each group's first corner, its material and
        // its UV.
        let mut group_vertex = vec![u32::MAX; corners.len()];
        let mut group_material = vec![0u32; corners.len()];
        let mut group_uv = vec![[0.0f32; 2]; corners.len()];
        // A group's vertices for its other materials and UVs.
        let mut split: HashMap<(u32, u32, [u32; 2]), u32> = HashMap::new();
        let mut split_vertices: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut corner_vertex = vec![0u32; corners.len()];
        let mut point_vertex = vec![u32::MAX; mesh.point_slot_count()];
        for (f, pair) in starts.windows(2).enumerate() {
            let material = if has_materials {
                mesh::material::slot_of(mesh, mesh::FaceId::from_raw(f as u32))
            } else {
                0
            };
            for k in pair[0] as usize..pair[1] as usize {
                let rep = groups[k] as usize;
                let uv = uv_at(k);
                let make = |vertices: &mut Vec<MeshVertex>| {
                    let position = mesh.position(corners[k]).unwrap_or(Vec3::ZERO);
                    vertices.push(MeshVertex {
                        position: position.to_array(),
                        normal: normals[k].to_array(),
                        material,
                        uv,
                    });
                    vertices.len() as u32 - 1
                };
                let vertex = if group_vertex[rep] == u32::MAX {
                    group_vertex[rep] = make(&mut vertices);
                    group_material[rep] = material;
                    group_uv[rep] = uv;
                    group_vertex[rep]
                } else if group_material[rep] == material && group_uv[rep] == uv {
                    group_vertex[rep]
                } else {
                    // Where materials or UVs meet on a soft edge: the same
                    // shading, a vertex for each material and UV.
                    let key = (rep as u32, material, uv.map(f32::to_bits));
                    *split.entry(key).or_insert_with(|| {
                        let vertex = make(&mut vertices);
                        split_vertices.entry(rep as u32).or_default().push(vertex);
                        vertex
                    })
                };
                corner_vertex[k] = vertex;
                let slot = &mut point_vertex[corners[k].get() as usize];
                if *slot == u32::MAX {
                    *slot = vertex;
                }
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
                    material: 0,
                    uv: [0.0; 2],
                });
            }
        }
        let mut triangles = Vec::with_capacity(corners.len() * 3);
        for pair in starts.windows(2) {
            let (a, b) = (pair[0] as usize, pair[1] as usize);
            // Bent and dented polygons are cut along lines inside them.
            mesh::face_triangles(mesh, &corners[a..b], |[i, j, k]| {
                triangles.extend([
                    corner_vertex[a + i],
                    corner_vertex[a + j],
                    corner_vertex[a + k],
                ]);
            });
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
            split_vertices,
            hardness,
            uv_density: mesh::uvs::average_density(mesh).unwrap_or(0.0),
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
                for vertex in self.split_vertices.get(&rep).into_iter().flatten() {
                    self.vertices[*vertex as usize].normal = normal.to_array();
                    touched.push(*vertex);
                }
            }
        }
        touched
    }

    /// What decided where its shading breaks.
    pub fn hardness(&self) -> mesh::Hardness {
        self.hardness
    }

    /// Whether moving points from where they are in `before` to `after`
    /// can be brought up to date with [`Self::moved`]: always for shading
    /// by the marks; by angle, only while no edge around them turns
    /// sharp or soft (otherwise build it afresh).
    pub fn can_follow(&self, before: &Mesh, after: &Mesh, moved: &[VertexId]) -> bool {
        match self.hardness {
            mesh::Hardness::Marked => true,
            mesh::Hardness::ByAngle(angle) => {
                let around = mesh::shading_reach(after, moved);
                !mesh::angle_hardness_changed(before, after, &around, angle)
            }
        }
    }

    /// Brings the geometry up to date after `moved` points of the mesh it
    /// was made from moved to where they are in `mesh` (which must have
    /// the same faces: [`Mesh::same_structure`]). Only the vertices around
    /// them are worked out again; returns the ranges of vertices to upload.
    /// Shaded by angle, check [`Self::can_follow`] first.
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
    /// Its materials' colours by slot ([`palette`]): all 0 for the
    /// standard surface colour.
    pub palette: Palette,
    /// What its surface shows (decision 0102).
    pub surface: SurfaceDraw,
}

/// What a surface shows: its colours, a checker laid on by its UVs, or
/// how much its UVs stretch (decision 0102).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SurfaceDraw {
    /// Its materials' colours.
    #[default]
    Plain,
    /// A checker of `squares` squares across the texture.
    Checker {
        /// How many squares across.
        squares: f32,
    },
    /// Green where the UVs are even, through yellow to red where they
    /// stretch one way or are twice too big or small against `density`
    /// (the object's texel density, texture widths per centimetre).
    Stretch {
        /// What each spot's density is compared with.
        density: f32,
    },
    /// The numbered checker image (decision 0107), laid on by the UVs:
    /// the image given to `ViewportRenderer::set_grid_image`.
    NumberedChecker,
}

impl SurfaceDraw {
    fn uniform(self) -> [f32; 4] {
        match self {
            Self::Plain => [0.0; 4],
            Self::Checker { squares } => [1.0, squares, 0.0, 0.0],
            Self::Stretch { density } => [2.0, 0.0, density, 0.0],
            Self::NumberedChecker => [3.0, 0.0, 0.0, 0.0],
        }
    }
}

/// How many material slots an object's palette holds
/// ([`mesh::material::MAX_SLOTS`]).
pub const PALETTE_SIZE: usize = mesh::material::MAX_SLOTS as usize;

/// An object's material colours by slot, each as sRGB bytes (red in the
/// lowest byte, then green, blue and 255): small enough to send with
/// every drawn object. 0 means the standard surface colour.
pub type Palette = [u32; PALETTE_SIZE];

/// The palette for materials of `colors` (sRGB, 0 to 1, by slot). Slots
/// past the last use the first colour, as an export does. No colours:
/// the standard surface colour everywhere.
pub fn palette(colors: &[[f32; 3]]) -> Palette {
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u32;
    let pack = |[r, g, b]: [f32; 3]| byte(r) | byte(g) << 8 | byte(b) << 16 | 255 << 24;
    let mut out = [0; PALETTE_SIZE];
    if let Some(first) = colors.first() {
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = pack(*colors.get(i).unwrap_or(first));
        }
    }
    out
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
    /// What the surface shows (`SurfaceDraw`): x: 0 plain, 1 checker, 2
    /// stretch, 3 the numbered checker; y: checker squares across; z: the
    /// density compared with.
    surface: [f32; 4],
    /// The material colours, four slots to a row.
    materials: [[u32; 4]; PALETTE_SIZE / 4],
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
            surface: match draw.surface {
                // The density in the world: the placement's scale spreads it.
                SurfaceDraw::Stretch { density } => SurfaceDraw::Stretch {
                    density: density / scale.max(1e-9),
                }
                .uniform(),
                other => other.uniform(),
            },
            materials: std::array::from_fn(|row| {
                std::array::from_fn(|i| draw.palette[row * 4 + i])
            }),
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
    fn vertices_split_where_the_uvs_do() {
        // A sphere: smooth all over, but cut open along its seam and poles
        // in the texture; every corner's vertex carries its own UV.
        let sphere = PrimitiveKind::Sphere.standard().mesh();
        let geometry = MeshGeometry::new(&sphere);
        let uvs = mesh::uvs::Uvs::of(&sphere).unwrap();
        let (_, corners) = sphere.face_layout();
        for k in 0..corners.len() {
            let v = geometry.corner_vertex[k] as usize;
            assert_eq!(geometry.vertices[v].uv, uvs.at(k).to_array(), "corner {k}");
        }
        assert!(geometry.uv_density > 0.0);
        // Without UVs, nothing more is split and the density is 0.
        let mut bare = sphere.clone();
        bare.remove_attribute(mesh::attributes::UV);
        let plain = MeshGeometry::new(&bare);
        assert!(plain.vertices.len() < geometry.vertices.len());
        assert_eq!(plain.uv_density, 0.0);
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
    fn shading_by_angle_smooths_marked_edges_and_says_when_to_rebuild() {
        // A cylinder with every edge marked hard, as after an extrude.
        let mut cylinder = PrimitiveKind::Cylinder.standard().mesh();
        let edges: Vec<mesh::EdgeKey> = cylinder.edges().to_vec();
        for e in &edges {
            let (a, b) = e.ends();
            cylinder.set_hard(a, b, true);
        }
        let marked = MeshGeometry::new(&cylinder);
        let by_angle = MeshGeometry::shaded(&cylinder, mesh::Hardness::ByAngle(30.0));
        assert_eq!(by_angle.hardness(), mesh::Hardness::ByAngle(30.0));
        // Smooth sides share vertices: at a rim point, one for the sides
        // and one for the cap, where the marks gave each side its own.
        assert!(
            by_angle.vertices.len() * 4 < marked.vertices.len() * 3,
            "{} vs {}",
            by_angle.vertices.len(),
            marked.vertices.len()
        );
        // Moving a cap up keeps every edge as it was: it can follow.
        let top: Vec<VertexId> = cylinder
            .vertices()
            .filter(|(_, p)| p.z > 50.0)
            .map(|(v, _)| v)
            .collect();
        let mut taller = cylinder.clone();
        for v in &top {
            let p = taller.position(*v).unwrap();
            taller.set_position(*v, p + Vec3::Z * 30.0);
        }
        assert!(by_angle.can_follow(&cylinder, &taller, &top));
        assert!(marked.can_follow(&cylinder, &taller, &top));
        // Pulling one rim point far out folds the side panels beside it
        // sharply: build it afresh.
        let corner = *top
            .iter()
            .max_by(|a, b| {
                let x = |v: &VertexId| cylinder.position(*v).unwrap().x;
                x(a).total_cmp(&x(b))
            })
            .unwrap();
        let mut pulled = cylinder.clone();
        let p = pulled.position(corner).unwrap();
        pulled.set_position(corner, p + Vec3::X * 200.0);
        assert!(!by_angle.can_follow(&cylinder, &pulled, &[corner]));
        assert!(marked.can_follow(&cylinder, &pulled, &[corner]));
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
    fn deleted_polygons_are_left_out() {
        let mut cube = PrimitiveKind::Cube.standard().mesh();
        let top = cube
            .faces()
            .find(|(f, _)| cube.face_normal(*f).z > 0.9)
            .unwrap()
            .0;
        assert_eq!(cube.delete_faces(&[top]), 1);
        let geometry = MeshGeometry::new(&cube);
        assert_eq!(geometry.triangles.len(), 5 * 2 * 3, "five sides left");
    }

    #[test]
    fn materials_split_vertices_where_they_meet_and_keep_shading() {
        let mut sphere = PrimitiveKind::Sphere.standard().mesh();
        let plain = MeshGeometry::new(&sphere);
        assert!(plain.vertices.iter().all(|v| v.material == 0));
        // The top half another material: the vertices along the middle
        // are doubled, one for each material, with the same normal.
        let bounds = sphere.bounds().unwrap();
        let middle = (bounds.min.z + bounds.max.z) / 2.0;
        let top: Vec<mesh::FaceId> = sphere
            .faces()
            .map(|(f, _)| f)
            .filter(|f| {
                let points = sphere.face(*f).unwrap();
                let z = points
                    .iter()
                    .map(|v| sphere.position(*v).unwrap().z)
                    .sum::<f32>();
                z / points.len() as f32 > middle
            })
            .collect();
        let outcome = mesh::material::set_material(&mut sphere, &top, 3);
        let split = MeshGeometry::new(&sphere);
        assert_eq!(outcome.matched, top.len());
        assert!(split.vertices.len() > plain.vertices.len());
        assert_eq!(split.triangles.len(), plain.triangles.len());
        let of = |material| {
            split
                .vertices
                .iter()
                .filter(|v| v.material == material)
                .count()
        };
        assert!(of(3) > 0 && of(0) > 0);
        for pair in split.triangles.chunks(3) {
            let materials: Vec<u32> = pair
                .iter()
                .map(|i| split.vertices[*i as usize].material)
                .collect();
            assert!(
                materials.windows(2).all(|w| w[0] == w[1]),
                "one material each"
            );
        }
        // Moving a point on the seam reshades both its vertices.
        let mut geometry = split.clone();
        let seam = sphere
            .vertices()
            .find(|(_, p)| (p.z - middle).abs() < 1e-3)
            .unwrap()
            .0;
        let mut after = sphere.clone();
        let p = after.position(seam).unwrap();
        after.set_position(seam, p * 1.2);
        let moved = after.moved_since(&sphere).unwrap();
        geometry.moved(&after, &moved);
        let fresh = MeshGeometry::new(&after);
        let at = |g: &MeshGeometry| {
            let mut found: Vec<[i32; 7]> = g
                .vertices
                .iter()
                .map(|v| {
                    let q = |x: f32| (x * 1000.0).round() as i32;
                    [
                        q(v.position[0]),
                        q(v.position[1]),
                        q(v.position[2]),
                        q(v.normal[0]),
                        q(v.normal[1]),
                        q(v.normal[2]),
                        v.material as i32,
                    ]
                })
                .collect();
            found.sort_unstable();
            found
        };
        assert_eq!(at(&geometry), at(&fresh));
    }

    #[test]
    fn palettes_pack_colours_and_fill_past_the_last() {
        assert_eq!(palette(&[]), [0; PALETTE_SIZE]);
        let p = palette(&[[1.0, 0.0, 0.0], [0.0, 0.5, 1.0]]);
        assert_eq!(p[0], 0xFF00_00FF);
        assert_eq!(p[1], 0xFFFF_8000);
        assert_eq!(p[2], p[0], "past the last: the first colour");
        assert_eq!(p[PALETTE_SIZE - 1], p[0]);
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
            palette: [0; PALETTE_SIZE],
            surface: SurfaceDraw::Plain,
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
        // Shaded by angle, as the app draws it while Automatic hard edges
        // is on (the standard, decision 0064).
        let start = Instant::now();
        let mut geometry = MeshGeometry::shaded(&mesh, mesh::Hardness::ByAngle(30.0));
        let built = start.elapsed();
        let mut after = mesh.clone();
        let moved: Vec<VertexId> = mesh.faces().nth(polygons / 2).unwrap().1.to_vec();
        for v in &moved {
            let p = after.position(*v).unwrap();
            after.set_position(*v, p + Vec3::Z * 5.0);
        }
        let start = Instant::now();
        let found = after.moved_since(&mesh).expect("same faces");
        // Whether an edge turned sharp or soft doesn't matter here: a heavy
        // shape follows the move at once and is rebuilt in the background.
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
