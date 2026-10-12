//! Tests for finding an edit's elements again by position.

use super::*;
use crate::{CubeParams, PlaneParams, Primitive};

fn cube(size: Vec3, segments: u32) -> Mesh {
    Primitive::Cube(CubeParams {
        size,
        segments: [segments; 3],
    })
    .mesh()
}

const SIZE: Vec3 = Vec3::new(100.0, 100.0, 100.0);

/// The points of `mesh` within `filter`.
fn points_where(mesh: &Mesh, filter: impl Fn(Vec3) -> bool) -> Vec<VertexId> {
    mesh.vertices()
        .filter(|(_, p)| filter(*p))
        .map(|(v, _)| v)
        .collect()
}

/// The top polygon of a one-segment cube, as its corners.
fn top_polygon(mesh: &Mesh) -> Vec<VertexId> {
    mesh.faces()
        .find(|(f, _)| mesh.face_normal(*f).z > 0.9)
        .map(|(_, c)| c.to_vec())
        .unwrap()
}

#[test]
fn ids_still_fit_a_resized_shape_but_not_a_renumbered_one() {
    let small = cube(SIZE, 1);
    let anchors = Anchors::record(&small, ElementKind::Polygons, &[top_polygon(&small)]).unwrap();
    assert!(anchors.still_fit(&small));
    // Taller: the same points, in the same places relative to the box.
    assert!(anchors.still_fit(&cube(Vec3::new(100.0, 100.0, 250.0), 1)));
    // More segments: the IDs now name other points.
    assert!(!anchors.still_fit(&cube(SIZE, 2)));
    assert!(!anchors.still_fit(&Mesh::new()));
}

#[test]
fn a_polygon_finds_every_polygon_inside_it() {
    let small = cube(SIZE, 1);
    let anchors = Anchors::record(&small, ElementKind::Polygons, &[top_polygon(&small)]).unwrap();
    let finer = cube(SIZE, 2);
    let (found, missing) = anchors.find(&finer, &Topology::new(&finer));
    let top = points_where(&finer, |p| (p.z - 100.0).abs() < 1e-3);
    assert_eq!(missing, 0);
    assert_eq!(top.len(), 9, "a 3 × 3 grid of points on top");
    assert_eq!(found.len(), 9);
    assert!(top.iter().all(|v| found.contains(v)));
}

#[test]
fn an_edge_finds_the_edges_along_it() {
    let small = cube(SIZE, 1);
    let front_top: Vec<VertexId> = points_where(&small, |p| {
        (p.z - 100.0).abs() < 1e-3 && (p.y + 50.0).abs() < 1e-3
    });
    assert_eq!(front_top.len(), 2);
    let anchors = Anchors::record(&small, ElementKind::Edges, &[front_top]).unwrap();
    let finer = cube(Vec3::new(200.0, 100.0, 100.0), 3);
    let (found, missing) = anchors.find(&finer, &Topology::new(&finer));
    assert_eq!(missing, 0);
    assert_eq!(
        found.len(),
        4,
        "the line, now in three pieces, on the wider cube"
    );
    for v in &found {
        let p = finer.position(*v).unwrap();
        assert!(
            (p.z - 100.0).abs() < 1e-3 && (p.y + 50.0).abs() < 1e-3,
            "{p}"
        );
    }
}

#[test]
fn a_point_finds_the_point_in_its_place() {
    let small = cube(SIZE, 1);
    let corner = points_where(&small, |p| p == Vec3::new(50.0, -50.0, 100.0));
    let anchors = Anchors::record(&small, ElementKind::Points, &[corner]).unwrap();
    let finer = cube(SIZE, 4);
    let (found, missing) = anchors.find(&finer, &Topology::new(&finer));
    assert_eq!(missing, 0);
    assert_eq!(found.len(), 1);
    assert_eq!(
        finer.position(found[0]),
        Some(Vec3::new(50.0, -50.0, 100.0))
    );
}

#[test]
fn elements_with_no_place_left_are_counted_missing() {
    // A point in the middle of the top of a two-segment cube has no place
    // on a one-segment cube.
    let finer = cube(SIZE, 2);
    let middle = points_where(&finer, |p| p == Vec3::new(0.0, 0.0, 100.0));
    let anchors = Anchors::record(&finer, ElementKind::Points, &[middle]).unwrap();
    let coarse = cube(SIZE, 1);
    let (found, missing) = anchors.find(&coarse, &Topology::new(&coarse));
    assert!(found.is_empty());
    assert_eq!(missing, 1);
    assert_eq!(anchors.element_count(), 1);
    // An empty shape has nothing at all.
    let (found, missing) = anchors.find(&Mesh::new(), &Topology::new(&Mesh::new()));
    assert!(found.is_empty() && missing == 1);
}

#[test]
fn flat_shapes_keep_their_height() {
    // A plane has no height: its box is flat, which must not divide by
    // zero.
    let plane = |segments| {
        Primitive::Plane(PlaneParams {
            width: 100.0,
            depth: 100.0,
            segments: [segments, segments],
        })
        .mesh()
    };
    let one = plane(1);
    let face = one.faces().next().unwrap().1.to_vec();
    let anchors = Anchors::record(&one, ElementKind::Polygons, &[face]).unwrap();
    assert!(anchors.still_fit(&one));
    let finer = plane(3);
    let (found, missing) = anchors.find(&finer, &Topology::new(&finer));
    assert_eq!(missing, 0);
    assert_eq!(found.len(), 16, "every point of the 3 × 3 plane");
    assert!(anchors.memory_size() > 0);
    assert!(Anchors::record(&Mesh::new(), ElementKind::Points, &[]).is_none());
}
