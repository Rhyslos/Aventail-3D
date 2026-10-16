//! Tests for splitting off and combining shapes.

use super::*;
use crate::attributes::Value;
use crate::{MeshReport, Primitive};
use glam::{Vec2, Vec3};

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

#[test]
fn keep_only_leaves_just_those_polygons() {
    let mesh = cube();
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).z > 0.9)
        .unwrap();
    let part = keep_only(&mesh, &[top]);
    assert_eq!(part.face_count(), 1);
    assert_eq!(part.vertex_count(), 4);
    assert!(part.face(top).is_some(), "it keeps its id");
    // Nothing named: nothing left.
    assert_eq!(keep_only(&mesh, &[]).face_count(), 0);
    assert_eq!(keep_only(&mesh, &[]).vertex_count(), 0);
}

#[test]
fn combining_places_each_part_and_keeps_hard_edges() {
    let a = cube();
    let b = cube();
    let joined = combine(&[
        (&a, Mat4::IDENTITY),
        (&b, Mat4::from_translation(Vec3::new(300.0, 0.0, 0.0))),
    ]);
    assert_eq!(joined.face_count(), 12);
    assert_eq!(joined.vertex_count(), 16);
    assert_eq!(joined.hard_edge_count(), 24);
    let report = MeshReport::new(&joined);
    assert!(report.is_closed());
    assert_eq!(report.flipped_edges, 0);
    let far = joined.vertices().map(|(_, p)| p.x).fold(f32::MIN, f32::max);
    assert!((far - 350.0).abs() < 1e-3);
}

#[test]
fn a_mirrored_part_still_faces_out_and_values_come_along() {
    let mut a = cube();
    a.add_attribute("uv", Domain::Corner, Value::Vec2(Vec2::ZERO))
        .unwrap();
    let (f, corners) = a.faces().next().map(|(f, c)| (f, c.to_vec())).unwrap();
    a.set_attribute("uv", AttributeElement::Corner(f, 1), Value::Vec2(Vec2::ONE))
        .unwrap();
    let mirrored = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
    let joined = combine(&[(&a, mirrored)]);
    let report = MeshReport::new(&joined);
    assert_eq!(report.flipped_edges, 0);
    // The cube still faces out: its top's normal points up.
    let up = joined
        .faces()
        .map(|(f, _)| joined.face_normal(f))
        .fold(f32::MIN, |m, n| m.max(n.z));
    assert!(up > 0.9);
    // The corner's value is on the same point, whichever corner it now is.
    let point = corners[1];
    let at = a.position(point).unwrap() * Vec3::new(-1.0, 1.0, 1.0);
    let (nf, ncorners) = joined.faces().next().map(|(f, c)| (f, c.to_vec())).unwrap();
    let index = ncorners
        .iter()
        .position(|v| joined.position(*v).unwrap().distance(at) < 1e-3)
        .unwrap();
    assert_eq!(
        joined.attribute("uv", AttributeElement::Corner(nf, index)),
        Some(Value::Vec2(Vec2::ONE))
    );
    // Nothing to combine: an empty shape.
    assert_eq!(combine(&[]).face_count(), 0);
}

#[test]
fn placing_a_shape_moves_every_point_and_a_mirror_keeps_it_facing_out() {
    use glam::Vec3;
    let cube = Primitive::Cube(Default::default()).mesh();
    let mut moved = cube.clone();
    let matrix = Mat4::from_scale_rotation_translation(
        Vec3::new(2.0, 1.0, 0.5),
        glam::Quat::from_rotation_z(0.7),
        Vec3::new(10.0, -5.0, 3.0),
    );
    let out = place(&mut moved, matrix);
    assert_eq!(out.matched, cube.vertex_count());
    for (v, p) in cube.vertices() {
        assert!(
            moved
                .position(v)
                .unwrap()
                .distance(matrix.transform_point3(p))
                < 1e-3
        );
    }
    assert_eq!(moved.face_count(), cube.face_count());
    // Mirrored: still closed and facing out.
    let mut mirrored = cube.clone();
    place(&mut mirrored, Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)));
    let report = MeshReport::new(&mirrored);
    assert!(report.is_closed() && !report.has_problems(), "{report:?}");
    assert!(crate::cleanup::faces_facing_wrong(&mirrored).is_empty());
    // Flat or broken: nothing changes.
    for broken in [Mat4::from_scale(Vec3::new(1.0, 0.0, 1.0)), Mat4::NAN] {
        let mut same = cube.clone();
        assert_eq!(place(&mut same, broken).skipped, 1);
        assert_eq!(same, cube);
    }
}
