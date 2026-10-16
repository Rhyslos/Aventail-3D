//! Tests for stable element names.

use crate::builder::{Builder, PointFrom};
use crate::{FaceId, Mesh, PlaneParams, Primitive, VertexId};
use glam::Vec3;

fn cube(segments: u32) -> Mesh {
    let mut mesh = Primitive::Cube(crate::CubeParams {
        segments: [segments, 1, 1],
        ..Default::default()
    })
    .mesh();
    mesh.name_base();
    mesh
}

/// The cube's corner nearest `p`.
fn at(mesh: &Mesh, p: Vec3) -> VertexId {
    mesh.vertices()
        .min_by(|a, b| a.1.distance(p).total_cmp(&b.1.distance(p)))
        .unwrap()
        .0
}

#[test]
fn base_corners_keep_their_names_with_more_segments_or_another_size() {
    let one = cube(1);
    let three = cube(3);
    let corner = Vec3::new(50.0, 50.0, 100.0);
    let name = one.point_name(at(&one, corner)).unwrap();
    assert_eq!(three.point_name(at(&three, corner)), Some(name));
    // A bigger cube: the same spot of its box, the same name.
    let mut big = Primitive::Cube(crate::CubeParams {
        size: Vec3::new(300.0, 100.0, 50.0),
        ..Default::default()
    })
    .mesh();
    big.name_base();
    assert_eq!(
        big.point_name(at(&big, Vec3::new(150.0, 50.0, 50.0))),
        Some(name)
    );
    // Every point's name is different.
    let names: std::collections::BTreeSet<u64> = three
        .vertices()
        .map(|(v, _)| three.point_name(v).unwrap())
        .collect();
    assert_eq!(names.len(), three.vertex_count());
    // A mesh never named is named after its ids (a plain mesh's never
    // change); named, after its points' places.
    let mut plain = Primitive::Plane(PlaneParams::default()).mesh();
    let v = plain.vertices().next().unwrap().0;
    let by_id = plain.trusted_point_name(v);
    assert!(by_id.is_some());
    plain.name_base();
    assert_ne!(plain.trusted_point_name(v), by_id);
    assert!(plain.trusted_point_name(v).is_some());
    assert!(plain.face_name(FaceId::from_raw(0)).is_some());
    assert_eq!(plain.face_name(FaceId::from_raw(999)), None);
}

#[test]
fn an_edits_points_are_named_by_where_they_came_from_not_their_number() {
    // The same copy made after a different number of other points gets
    // the same name.
    let make = |extra: usize| {
        let mut mesh = cube(1);
        let original = mesh.clone();
        let corner = at(&mesh, Vec3::new(50.0, 50.0, 100.0));
        mesh.begin_naming(7);
        let mut build = Builder::new(&original, &mut mesh);
        for _ in 0..extra {
            build.point(Vec3::ZERO, PointFrom::Nothing);
        }
        let copy = build.point(Vec3::ONE, PointFrom::Copy(corner));
        let second = build.point(Vec3::ONE, PointFrom::Copy(corner));
        let between = build.point(
            Vec3::ONE,
            PointFrom::Between(corner, VertexId::from_raw(0), 0.5),
        );
        mesh.end_naming();
        (
            mesh.point_name(copy).unwrap(),
            mesh.point_name(second).unwrap(),
            mesh.point_name(between).unwrap(),
            copy,
        )
    };
    let (a, b, c, id_a) = make(0);
    let (x, y, z, id_x) = make(5);
    assert_ne!(id_a, id_x, "numbered differently");
    assert_eq!((a, b, c), (x, y, z), "named the same");
    assert_ne!(a, b, "two copies of one point told apart");
    // Another edit (salt) names them differently.
    let mut mesh = cube(1);
    let original = mesh.clone();
    let corner = at(&mesh, Vec3::new(50.0, 50.0, 100.0));
    mesh.begin_naming(8);
    let copy = Builder::new(&original, &mut mesh).point(Vec3::ONE, PointFrom::Copy(corner));
    mesh.end_naming();
    assert_ne!(mesh.point_name(copy), Some(a));
    // Outside an edit nothing new is named: a point added to a named
    // shape has no trusted name.
    let mut mesh = cube(1);
    let v = mesh.add_vertex(Vec3::ONE).unwrap();
    assert!(mesh.trusted_point_name(v).is_none());
    assert!(mesh.point_name(v).is_some());
    // Names don't make meshes different.
    let named = cube(2);
    let unnamed = Primitive::Cube(crate::CubeParams {
        segments: [2, 1, 1],
        ..Default::default()
    })
    .mesh();
    assert_eq!(named, unnamed);
}
