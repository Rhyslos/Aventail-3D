//! Tests for saving and putting back UV layouts (decision 0114).

use super::*;
use crate::{CubeParams, Primitive, SphereParams};
use glam::Vec3;

fn cube() -> Mesh {
    Primitive::Cube(CubeParams::default()).mesh()
}

/// Every UV at 0 and no seams: a layout to put back over.
fn scrambled(mut mesh: Mesh) -> Mesh {
    let n = mesh.face_layout().1.len();
    assert!(set_uvs(&mut mesh, vec![Vec2::ZERO; n]));
    let seams = super::super::seams(&mesh);
    set_seams(&mut mesh, &seams, false);
    mesh
}

#[test]
fn a_layout_comes_back_exactly() {
    for mesh in [cube(), Primitive::Sphere(SphereParams::default()).mesh()] {
        let layout = UvLayout::of(&mesh).unwrap();
        assert_eq!(layout.polygon_count(), mesh.face_count());
        assert!(!layout.seams.is_empty());
        assert!(layout.is_real());
        let mut m = scrambled(mesh.clone());
        assert_ne!(UvLayout::of(&m).unwrap(), layout);
        let outcome = restore_layout(&mut m, &layout);
        assert_eq!(outcome.matched, mesh.face_count());
        assert_eq!((outcome.missing, outcome.skipped), (0, 0));
        assert_eq!(UvLayout::of(&m).unwrap(), layout);
    }
}

#[test]
fn putting_back_the_same_layout_changes_nothing() {
    let mesh = cube();
    let layout = UvLayout::of(&mesh).unwrap();
    let mut m = mesh.clone();
    restore_layout(&mut m, &layout);
    assert_eq!(UvLayout::of(&m).unwrap(), layout);
    assert_eq!(every_uv(&m), every_uv(&mesh));
}

fn every_uv(mesh: &Mesh) -> Vec<Vec2> {
    let n = mesh.face_layout().1.len();
    Uvs::of(mesh).unwrap().into_values(n)
}

#[test]
fn a_mesh_without_uvs_has_none_to_save() {
    let mut mesh = Mesh::new();
    let a = mesh.add_vertex(Vec3::ZERO).unwrap();
    let b = mesh.add_vertex(Vec3::X).unwrap();
    let c = mesh.add_vertex(Vec3::Y).unwrap();
    mesh.add_face(&[a, b, c]).unwrap();
    assert!(UvLayout::of(&mesh).is_none());
}

#[test]
fn putting_a_layout_on_a_mesh_without_uvs_gives_it_uvs() {
    // A pentagon (an n-gon) and a triangle sharing an edge, open around.
    let mut mesh = Mesh::new();
    let p: Vec<_> = (0..6)
        .map(|i| {
            let a = i as f32;
            mesh.add_vertex(Vec3::new(a.cos(), a.sin(), 0.0)).unwrap()
        })
        .collect();
    let pentagon = mesh.add_face(&[p[0], p[1], p[2], p[3], p[4]]).unwrap();
    let triangle = mesh.add_face(&[p[0], p[4], p[5]]).unwrap();
    let layout = UvLayout {
        faces: vec![
            (
                pentagon,
                (0..5).map(|k| Vec2::splat(k as f32 / 10.0)).collect(),
            ),
            (triangle, vec![Vec2::ONE, Vec2::X, Vec2::Y]),
        ],
        seams: vec![EdgeKey::new(p[0], p[4])],
    };
    let outcome = restore_layout(&mut mesh, &layout);
    assert_eq!(outcome.matched, 2);
    assert_eq!(UvLayout::of(&mesh).unwrap(), layout);
}

#[test]
fn polygons_made_since_keep_their_uvs() {
    let mesh = cube();
    let layout = UvLayout::of(&mesh).unwrap();
    let mut m = scrambled(mesh);
    // A new polygon, away from the cube, with its own UVs.
    let a = m.add_vertex(Vec3::new(5.0, 0.0, 0.0)).unwrap();
    let b = m.add_vertex(Vec3::new(6.0, 0.0, 0.0)).unwrap();
    let c = m.add_vertex(Vec3::new(5.0, 1.0, 0.0)).unwrap();
    let new = m.add_face(&[a, b, c]).unwrap();
    let n = m.face_layout().1.len();
    let mut values = vec![Vec2::ZERO; n];
    let start = m.face_layout().0[new.get() as usize] as usize;
    values[start..start + 3].copy_from_slice(&[Vec2::splat(0.9), Vec2::ONE, Vec2::X]);
    assert!(set_uvs(&mut m, values));
    let outcome = restore_layout(&mut m, &layout);
    assert_eq!(outcome.matched, layout.polygon_count());
    let after = UvLayout::of(&m).unwrap();
    let kept = after.faces.iter().find(|(f, _)| *f == new).unwrap();
    assert_eq!(kept.1, vec![Vec2::splat(0.9), Vec2::ONE, Vec2::X]);
}

#[test]
fn deleted_polygons_are_missing_and_the_rest_come_back() {
    let mesh = cube();
    let layout = UvLayout::of(&mesh).unwrap();
    let mut m = scrambled(mesh);
    let gone = layout.faces[0].0;
    assert_eq!(m.delete_faces(&[gone]), 1);
    let outcome = restore_layout(&mut m, &layout);
    assert_eq!(outcome.missing, 1);
    assert_eq!(outcome.matched, layout.polygon_count() - 1);
    for (f, uvs) in UvLayout::of(&m).unwrap().faces {
        let saved = layout.faces.iter().find(|(g, _)| *g == f).unwrap();
        assert_eq!(&saved.1, &uvs);
    }
}

#[test]
fn a_polygon_with_other_corners_is_skipped() {
    let mesh = cube();
    let mut layout = UvLayout::of(&mesh).unwrap();
    layout.faces[0].1.push(Vec2::ONE);
    let mut m = scrambled(mesh);
    let outcome = restore_layout(&mut m, &layout);
    assert_eq!(outcome.skipped, 1);
    assert_eq!(outcome.matched, layout.polygon_count() - 1);
    let after = UvLayout::of(&m).unwrap();
    assert!(after.faces[0].1.iter().all(|uv| *uv == Vec2::ZERO));
}

#[test]
fn seams_next_to_unknown_polygons_are_left_alone() {
    let mut mesh = Mesh::new();
    let p: Vec<_> = (0..6)
        .map(|i| {
            mesh.add_vertex(Vec3::new(i as f32, (i % 2) as f32, 0.0))
                .unwrap()
        })
        .collect();
    let first = mesh.add_face(&[p[0], p[1], p[2]]).unwrap();
    let n = mesh.face_layout().1.len();
    assert!(set_uvs(&mut mesh, vec![Vec2::ZERO; n]));
    let layout = UvLayout::of(&mesh).unwrap();
    let second = mesh.add_face(&[p[1], p[3], p[2]]).unwrap();
    let third = mesh.add_face(&[p[2], p[3], p[4]]).unwrap();
    let n = mesh.face_layout().1.len();
    assert!(set_uvs(&mut mesh, vec![Vec2::ZERO; n]));
    // Between a known and a new polygon, and between two new ones.
    let shared = EdgeKey::new(p[1], p[2]);
    let new_only = EdgeKey::new(p[2], p[3]);
    set_seams(&mut mesh, &[shared, new_only], true);
    let outcome = restore_layout(&mut mesh, &layout);
    assert_eq!(outcome.matched, 1);
    assert!(is_seam(&mesh, shared) && is_seam(&mesh, new_only));
    let _ = (first, second, third);
}

#[test]
fn a_layout_that_isnt_real_changes_nothing() {
    let mesh = cube();
    let mut layout = UvLayout::of(&mesh).unwrap();
    layout.faces[2].1[1] = Vec2::new(f32::NAN, 0.0);
    assert!(!layout.is_real());
    let mut m = scrambled(mesh);
    let before = m.clone();
    let outcome = restore_layout(&mut m, &layout);
    assert_eq!(outcome.matched, 0);
    assert_eq!(every_uv(&m), every_uv(&before));
}

#[test]
fn an_empty_layout_changes_nothing() {
    let mesh = cube();
    let mut m = mesh.clone();
    let outcome = restore_layout(&mut m, &UvLayout::default());
    assert_eq!(outcome, crate::edit::Outcome::default());
    assert_eq!(every_uv(&m), every_uv(&mesh));
    assert_eq!(UvLayout::default().memory_size(), 0);
    assert!(UvLayout::of(&mesh).unwrap().memory_size() > 0);
}
