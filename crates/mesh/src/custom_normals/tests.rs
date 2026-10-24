//! Tests for custom normals (decision 0148).

use super::*;
use crate::{Hardness, Primitive, VertexId};
use glam::Mat4;

/// Two squares meeting at a right angle along the X axis, the edge soft:
/// one on the ground facing up, one standing facing -Y.
fn folded() -> (Mesh, [VertexId; 6]) {
    let mut mesh = Mesh::new();
    let v = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(1.0, 0.0, -1.0),
        Vec3::new(0.0, 0.0, -1.0),
    ]
    .map(|p| mesh.add_vertex(p).unwrap());
    mesh.add_face(&[v[0], v[1], v[2], v[3]]).unwrap();
    mesh.add_face(&[v[0], v[5], v[4], v[1]]).unwrap();
    (mesh, v)
}

#[test]
fn set_corners_shade_with_their_own_normal_and_the_rest_as_usual() {
    let (mut mesh, _) = folded();
    let usual = crate::corner_normal_list(&mesh);
    // The first square's first corner set; the rest left unset.
    let mut list = vec![Vec3::ZERO; 8];
    list[0] = Vec3::new(0.0, 0.0, 2.0);
    set_custom_normals(&mut mesh, &list);
    assert!(has_custom_normals(&mesh));
    let (groups, normals) = crate::corner_shading_with(&mesh, Hardness::Marked);
    assert_eq!(normals[0], Vec3::Z, "set, made unit length");
    for k in 1..8 {
        assert_eq!(normals[k], usual[k], "corner {k}");
    }
    // Corner 0 shared its group with the other square's corner at that
    // point (corner 4); now they differ, so they are apart.
    assert_ne!(groups[0], groups[4]);
    // The edge's other end still shades as one.
    assert_eq!(groups[1], groups[7]);
}

#[test]
fn every_way_the_shading_is_read_uses_them() {
    let (mut mesh, v) = folded();
    let list = vec![Vec3::Y; 8];
    set_custom_normals(&mut mesh, &list);
    assert!(
        crate::corner_normal_list(&mesh)
            .iter()
            .all(|n| *n == Vec3::Y)
    );
    assert!(
        crate::shaded_triangles(&mesh)
            .iter()
            .all(|t| t.normals == [Vec3::Y; 3])
    );
    let face = mesh.faces().next().unwrap().0;
    assert_eq!(crate::corner_normals(&mesh)[&(face, v[2])], Vec3::Y);
    // By angle too: the custom normals win over the marks and the angle.
    let (_, by_angle) = crate::corner_shading_with(&mesh, Hardness::ByAngle(10.0));
    assert!(by_angle.iter().all(|n| *n == Vec3::Y));
}

#[test]
fn values_that_arent_directions_are_left_unset() {
    let (mut mesh, _) = folded();
    let usual = crate::corner_normal_list(&mesh);
    set_custom_normals(
        &mut mesh,
        &[Vec3::NAN, Vec3::INFINITY, Vec3::ZERO, Vec3::splat(1e-9)],
    );
    let list = custom_normal_list(&mesh).unwrap();
    assert_eq!(list.len(), 8);
    assert!(list.iter().all(Option::is_none));
    assert_eq!(crate::corner_normal_list(&mesh), usual);
}

#[test]
fn forgetting_them_brings_the_usual_shading_back() {
    let (mut mesh, _) = folded();
    let before = mesh.clone();
    assert!(!forget_custom_normals(&mut mesh));
    set_custom_normals(&mut mesh, &[Vec3::X; 8]);
    assert!(forget_custom_normals(&mut mesh));
    assert_eq!(mesh, before);
    assert!(custom_normal_list(&mesh).is_none());
}

#[test]
fn a_layer_of_another_kind_isnt_taken_for_them() {
    let (mut mesh, _) = folded();
    mesh.add_attribute(CUSTOM_NORMAL, Domain::Corner, Value::Float(0.0))
        .unwrap();
    assert!(!has_custom_normals(&mesh));
    assert!(custom_normal_list(&mesh).is_none());
    assert!(split_edges(&mesh).is_none());
}

#[test]
fn shading_is_unchanged_by_uvs_but_changed_by_moves_and_marks() {
    let cube = Primitive::Cube(Default::default()).mesh();
    let mut uvs = cube.clone();
    let face = cube.faces().next().unwrap().0;
    uvs.add_attribute(
        crate::attributes::UV,
        Domain::Corner,
        Value::Vec2(glam::Vec2::ZERO),
    )
    .unwrap();
    uvs.set_attribute(
        crate::attributes::UV,
        crate::AttributeElement::Corner(face, 0),
        Value::Vec2(glam::Vec2::ONE),
    )
    .unwrap();
    assert!(shading_unchanged(&cube, &uvs));
    let mut moved = cube.clone();
    let (v, p) = cube.vertices().next().unwrap();
    moved.set_position(v, p + Vec3::X);
    assert!(!shading_unchanged(&cube, &moved));
    let mut marked = cube.clone();
    let (a, b) = cube.edges()[0].ends();
    marked.set_hard(a, b, !cube.is_hard(a, b));
    assert!(!shading_unchanged(&cube, &marked));
    let mut fewer = cube.clone();
    fewer.delete_faces(&[face]);
    assert!(!shading_unchanged(&cube, &fewer));
}

#[test]
fn split_edges_are_where_the_normals_part() {
    let (mut mesh, v) = folded();
    assert!(split_edges(&mesh).is_none());
    // The same normal on both sides of the fold: no split.
    set_custom_normals(&mut mesh, &[Vec3::Z; 8]);
    assert_eq!(split_edges(&mesh), Some(Vec::new()));
    // Each square its own facing: the fold is split.
    let mut list = vec![Vec3::Z; 4];
    list.extend([Vec3::NEG_Y; 4]);
    set_custom_normals(&mut mesh, &list);
    assert_eq!(split_edges(&mesh), Some(vec![EdgeKey::new(v[0], v[1])]));
    assert_eq!(
        crate::hardness::shading_hard_edges(&mesh, Hardness::Marked),
        vec![EdgeKey::new(v[0], v[1])]
    );
    // A third polygon on the edge: always split.
    let extra = mesh.add_vertex(Vec3::new(0.5, -1.0, 0.5)).unwrap();
    mesh.add_face(&[v[0], v[1], extra]).unwrap();
    assert!(
        split_edges(&mesh)
            .unwrap()
            .contains(&EdgeKey::new(v[0], v[1]))
    );
}

#[test]
fn appended_parts_turn_their_normals() {
    let (mut part, _) = folded();
    set_custom_normals(&mut part, &[Vec3::Z; 8]);
    let mut out = Mesh::new();
    // Stretched along Z (which doesn't turn a direction along Z), then a
    // quarter turn about X: up becomes +Y.
    let matrix = Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 3.0));
    crate::parts::append(&mut out, &part, matrix);
    let list = custom_normal_list(&out).unwrap();
    for n in list {
        let n = n.unwrap();
        assert!(n.distance(Vec3::Y) < 1e-5, "{n}");
    }
    assert_eq!(placed(Vec3::ZERO, matrix), Vec3::ZERO);
    // A mirror turns them across.
    let mut mirrored = Mesh::new();
    crate::parts::append(
        &mut mirrored,
        &part,
        Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0)),
    );
    for n in custom_normal_list(&mirrored).unwrap() {
        assert!(n.unwrap().distance(Vec3::NEG_Z) < 1e-5);
    }
}

#[test]
fn the_live_mirror_drops_them() {
    let (mut mesh, _) = folded();
    set_custom_normals(&mut mesh, &[Vec3::Z; 8]);
    let copy = crate::mirror::with_copies(
        &mesh,
        &[crate::mirror::reflection([true, false, false], Vec3::ZERO)],
        |_| true,
        0.001,
    );
    assert!(!has_custom_normals(&copy));
    // No copies: the shape as it is, normals and all.
    let same = crate::mirror::with_copies(&mesh, &[], |_| true, 0.001);
    assert!(has_custom_normals(&same));
}
