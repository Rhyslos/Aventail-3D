//! A mesh's health check: how big it is, and anything that would cause
//! trouble later (in modelling tools, UV unwrapping, baking or a game
//! engine). Nothing is changed; the report only describes.

use crate::{Mesh, Topology};

/// Points closer than this (in centimetres) count as on the same spot.
const SAME_SPOT: f32 = 1e-4;
/// Faces with less area than this (in square centimetres) count as flat.
const NO_AREA: f32 = 1e-8;

/// What a mesh is made of, and its problems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeshReport {
    /// Points used by faces or not.
    pub points: usize,
    /// Edges of faces.
    pub edges: usize,
    /// Faces (polygons), any number of corners.
    pub faces: usize,
    /// Triangles once every face is cut into triangles, as a game engine
    /// draws it: the usual measure of a model's cost.
    pub triangles: usize,
    /// Separate pieces (faces linked through shared points).
    pub pieces: usize,
    /// Edges on open borders (used by one face). Normal for a plane or an
    /// open pipe; a hole in a shape meant to be solid.
    pub border_edges: usize,
    /// Edges shared by three or more faces: impossible for a real solid,
    /// and confusing for most tools.
    pub non_manifold_edges: usize,
    /// Edges whose two faces run the same way along them: one of the two
    /// faces points the wrong way ("flipped normal").
    pub flipped_edges: usize,
    /// Edges with no length: both ends on the same spot.
    pub zero_length_edges: usize,
    /// Faces with no area (flat as a line or a point).
    pub flat_faces: usize,
    /// Points no face uses.
    pub loose_points: usize,
}

impl MeshReport {
    /// Checks `mesh`.
    pub fn new(mesh: &Mesh) -> Self {
        Self::with_topology(mesh, &Topology::new(mesh))
    }

    /// Checks `mesh`, whose topology is already known.
    pub fn with_topology(mesh: &Mesh, topology: &Topology) -> Self {
        let zero_length_edges = topology
            .edges()
            .filter(|e| {
                let (a, b) = e.ends();
                match (mesh.position(a), mesh.position(b)) {
                    (Some(p), Some(q)) => p.distance(q) < SAME_SPOT,
                    _ => false,
                }
            })
            .count();
        let flat_faces = mesh
            .faces()
            .filter(|(id, _)| face_area(mesh, *id) < NO_AREA)
            .count();
        Self {
            points: mesh.vertex_count(),
            edges: topology.edge_count(),
            faces: mesh.face_count(),
            triangles: mesh.faces().map(|(_, c)| c.len().saturating_sub(2)).sum(),
            pieces: topology.pieces().len(),
            border_edges: topology.border_edges().len(),
            non_manifold_edges: topology.non_manifold_edges().len(),
            flipped_edges: topology.flipped_edges().len(),
            zero_length_edges,
            flat_faces,
            loose_points: mesh
                .vertices()
                .filter(|(v, _)| topology.vertex_faces(*v).is_empty())
                .count(),
        }
    }

    /// Whether the surface is closed: no open borders.
    pub fn is_closed(&self) -> bool {
        self.border_edges == 0
    }

    /// Whether anything is wrong (open borders aren't counted: an open
    /// shape can be meant to be open).
    pub fn has_problems(&self) -> bool {
        self.non_manifold_edges
            + self.flipped_edges
            + self.zero_length_edges
            + self.flat_faces
            + self.loose_points
            > 0
    }
}

/// A face's area, from its Newell normal (works for any polygon).
fn face_area(mesh: &Mesh, id: crate::FaceId) -> f32 {
    let Some(corners) = mesh.face(id) else {
        return 0.0;
    };
    let mut sum = glam::Vec3::ZERO;
    for (a, b) in crate::polygon::sides(corners) {
        if let (Some(p), Some(q)) = (mesh.position(a), mesh.position(b)) {
            sum += p.cross(q);
        }
    }
    sum.length() / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CylinderParams, Primitive, PrimitiveKind, VertexId};
    use glam::Vec3;

    fn build(points: &[[f32; 3]], faces: &[&[u32]]) -> Mesh {
        let mut mesh = Mesh::new();
        for p in points {
            mesh.add_vertex(Vec3::from_array(*p)).unwrap();
        }
        for f in faces {
            let c: Vec<VertexId> = f.iter().map(|n| VertexId::from_raw(*n)).collect();
            mesh.add_face(&c).unwrap();
        }
        mesh
    }

    #[test]
    fn a_cube_is_closed_clean_and_twelve_triangles() {
        let report = MeshReport::new(&PrimitiveKind::Cube.standard().mesh());
        assert_eq!(
            report,
            MeshReport {
                points: 8,
                edges: 12,
                faces: 6,
                triangles: 12,
                pieces: 1,
                ..MeshReport::default()
            }
        );
        assert!(report.is_closed());
        assert!(!report.has_problems());
    }

    #[test]
    fn every_primitive_is_clean_and_counts_its_triangles() {
        for primitive in Primitive::all() {
            let mesh = primitive.mesh();
            let report = MeshReport::new(&mesh);
            assert!(!report.has_problems(), "{primitive:?}: {report:?}");
            let fan: usize = mesh.faces().map(|(_, c)| c.len() - 2).sum();
            assert_eq!(report.triangles, fan);
            let open = matches!(primitive.kind(), PrimitiveKind::Plane | PrimitiveKind::Disc);
            assert_eq!(report.is_closed(), !open, "{primitive:?}");
        }
    }

    #[test]
    fn an_n_gon_counts_as_its_triangles() {
        let disc = PrimitiveKind::Disc.standard().mesh();
        let report = MeshReport::new(&disc);
        assert_eq!(report.faces, 1);
        assert_eq!(report.triangles, report.points - 2);
    }

    #[test]
    fn open_ends_are_borders_not_problems() {
        let pipe = Primitive::Cylinder(CylinderParams {
            caps: false,
            ..CylinderParams::default()
        })
        .mesh();
        let report = MeshReport::new(&pipe);
        assert!(!report.is_closed());
        assert_eq!(report.border_edges, 32);
        assert!(!report.has_problems());
    }

    #[test]
    fn problems_are_counted() {
        let mesh = build(
            &[
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.5, 1.0, 0.0],
                [0.5, -1.0, 0.0],
                [0.5, 0.0, 1.0],
                // A spot shared by two points, and a line-thin triangle.
                [3.0, 0.0, 0.0],
                [3.0, 0.0, 0.0],
                [4.0, 0.0, 0.0],
                // A loose point.
                [9.0, 9.0, 9.0],
            ],
            &[&[0, 1, 2], &[1, 0, 3], &[0, 1, 4], &[5, 6, 7]],
        );
        let report = MeshReport::new(&mesh);
        assert_eq!(report.non_manifold_edges, 1);
        assert_eq!(report.zero_length_edges, 1);
        assert_eq!(report.flat_faces, 1);
        assert_eq!(report.loose_points, 1);
        assert_eq!(report.pieces, 2);
        assert!(report.has_problems());
    }

    #[test]
    fn a_flipped_face_is_found() {
        let mesh = build(
            &[
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [2.0, 0.0, 0.0],
                [2.0, 1.0, 0.0],
            ],
            &[&[0, 1, 2, 3], &[1, 2, 5, 4]],
        );
        let report = MeshReport::new(&mesh);
        assert_eq!(report.flipped_edges, 1);
        assert!(report.has_problems());
    }

    #[test]
    fn an_empty_mesh_reports_nothing() {
        let report = MeshReport::new(&Mesh::new());
        assert_eq!(report, MeshReport::default());
        assert!(report.is_closed());
        assert!(!report.has_problems());
    }
}
