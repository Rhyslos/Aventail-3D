//! Tests for hard and soft edges.

use super::*;
use crate::shading::corner_normals;
use crate::{CylinderParams, FaceId, PlaneParams, Primitive, VertexId};
use glam::Vec3;

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

fn cylinder(sides: u32) -> Mesh {
    Primitive::Cylinder(CylinderParams {
        sides,
        ..Default::default()
    })
    .mesh()
}

#[test]
fn edges_can_be_made_soft_and_hard_again() {
    let mut mesh = cube();
    assert_eq!(mesh.hard_edge_count(), 12, "a cube starts with hard edges");
    let edges: Vec<EdgeKey> = mesh.edges().to_vec();
    let soft = set_hard_edges(&mut mesh, &edges[..4], false);
    assert_eq!((soft.matched, soft.skipped), (4, 0));
    assert_eq!(mesh.hard_edge_count(), 8);
    // Again: nothing to change.
    let again = set_hard_edges(&mut mesh, &edges[..4], false);
    assert_eq!((again.matched, again.skipped), (0, 4));
    let hard = set_hard_edges(&mut mesh, &edges, true);
    assert_eq!(hard.matched, 4);
    assert_eq!(mesh.hard_edge_count(), 12);
}

#[test]
fn a_soft_edge_shares_its_shading_and_a_hard_one_breaks_it() {
    let mut mesh = cube();
    let edge = mesh.edges()[0];
    let (a, _) = edge.ends();
    let faces: Vec<FaceId> = mesh.topology().edge_faces(edge).collect();
    let at = |mesh: &Mesh| {
        let normals = corner_normals(mesh);
        (normals[&(faces[0], a)], normals[&(faces[1], a)])
    };
    let (n1, n2) = at(&mesh);
    assert!(n1.dot(n2) < 0.1, "hard: each polygon its own facing");
    set_hard_edges(&mut mesh, &[edge], false);
    let (n1, n2) = at(&mesh);
    assert!(n1.distance(n2) < 1e-5, "soft: one shading across the edge");
    // The shape didn't change.
    assert_eq!(
        mesh.vertices().map(|(_, p)| p).collect::<Vec<_>>(),
        cube().vertices().map(|(_, p)| p).collect::<Vec<_>>()
    );
}

#[test]
fn by_angle_hardens_corners_and_softens_curves() {
    // A 32-sided cylinder: sides meet at about 11°, the caps at 90°.
    let mut mesh = cylinder(32);
    let edges: Vec<EdgeKey> = mesh.edges().to_vec();
    set_hard_edges(&mut mesh, &edges, true);
    let outcome = hard_edges_by_angle(&mut mesh, 30.0);
    assert!(outcome.matched > 0);
    let sharp = sharp_edges(&mesh, 30.0);
    assert_eq!(sharp.len(), 64, "the two rims");
    assert_eq!(mesh.hard_edge_count(), 64);
    // A small angle hardens everything; 180° nothing.
    hard_edges_by_angle(&mut mesh, 5.0);
    assert_eq!(mesh.hard_edge_count(), mesh.edges().len());
    hard_edges_by_angle(&mut mesh, MAX_ANGLE);
    assert_eq!(mesh.hard_edge_count(), 0);
    // Nothing to change is skipped.
    assert_eq!(hard_edges_by_angle(&mut mesh, 400.0).skipped, 1);
}

#[test]
fn border_edges_are_left_alone() {
    let mut mesh = Primitive::Plane(PlaneParams::default()).mesh();
    assert!(sharp_edges(&mesh, 0.0).is_empty(), "a plane has only borders");
    let border = mesh.edges()[0];
    let (a, b) = border.ends();
    mesh.set_hard(a, b, true);
    hard_edges_by_angle(&mut mesh, 30.0);
    assert!(mesh.is_hard(a, b), "untouched");
}

#[test]
fn missing_and_degenerate_edges() {
    let mut mesh = cube();
    let ghost = EdgeKey::new(VertexId::from_raw(0), VertexId::from_raw(999));
    let outcome = set_hard_edges(&mut mesh, &[ghost], true);
    assert_eq!((outcome.matched, outcome.missing), (0, 1));
    // A polygon squashed flat has no facing: its edges stay soft.
    let mut flat = Mesh::new();
    let v: Vec<VertexId> = [
        Vec3::ZERO,
        Vec3::X,
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
    ]
    .into_iter()
    .map(|p| flat.add_vertex(p).unwrap())
    .collect();
    flat.add_face(&[v[0], v[1], v[2]]).unwrap();
    flat.add_face(&[v[1], v[0], v[3]]).unwrap();
    assert!(sharp_edges(&flat, 30.0).is_empty());
    // An empty mesh has nothing to do.
    assert_eq!(hard_edges_by_angle(&mut Mesh::new(), 30.0).skipped, 1);
}
