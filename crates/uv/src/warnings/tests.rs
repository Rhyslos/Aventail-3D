//! Tests for the UV Editor's always-on warnings (decision 0115).

use super::*;
use glam::{Affine2, Vec2};
use mesh::{PlaneParams, Primitive, PrimitiveKind};

fn one(mesh: &Mesh) -> Warnings {
    warnings(&[mesh], |_, _| 0, &[]).remove(0)
}

/// Moves (or mirrors) a piece's UVs.
fn moved(mesh: &Mesh, piece: &[FaceId], t: Affine2) -> Mesh {
    let mut out = mesh.clone();
    mesh::uvs::transform_uvs(&mut out, piece, &[], t);
    out
}

/// A piece's middle in the texture.
fn middle(mesh: &Mesh, piece: &[FaceId]) -> Vec2 {
    mesh::uvs::uv_centre(mesh, piece, &[]).unwrap()
}

#[test]
fn ready_made_shapes_have_nothing_to_warn_about() {
    for kind in PrimitiveKind::ALL {
        let mesh = kind.standard().mesh();
        let w = one(&mesh);
        assert!(w.is_clean(), "{kind:?}: {w:?}");
        assert_eq!((w.overlapping_pieces, w.mirrored_pieces), (0, 0));
    }
}

#[test]
fn a_piece_moved_onto_another_overlaps_it() {
    let mesh = PrimitiveKind::Cube.standard().mesh();
    let islands = mesh::uvs::islands(&mesh);
    let (a, b) = (&islands[0], &islands[1]);
    let shift = middle(&mesh, b) - middle(&mesh, a) + Vec2::splat(0.01);
    // Nudged off the exact spot, so it isn't a stack.
    let m = moved(&mesh, a, Affine2::from_translation(shift));
    let w = one(&m);
    assert_eq!(w.overlapping_pieces, 2);
    assert!(a.iter().chain(b.iter()).all(|f| w.overlapping.contains(f)));
    assert!(w.mirrored.is_empty());
}

#[test]
fn a_mirrored_piece_is_warned_about() {
    let mesh = PrimitiveKind::Cube.standard().mesh();
    let piece = mesh::uvs::islands(&mesh)[2].clone();
    let c = middle(&mesh, &piece);
    let flip = Affine2::from_translation(c)
        * Affine2::from_scale(Vec2::new(-1.0, 1.0))
        * Affine2::from_translation(-c);
    let m = moved(&mesh, &piece, flip);
    let w = one(&m);
    assert_eq!(w.mirrored_pieces, 1);
    assert_eq!(w.mirrored, piece.iter().copied().collect());
    assert!(w.overlapping.is_empty(), "mirrored in place, over nothing");
    assert_eq!(mirrored_pieces(&m), vec![piece]);
}

#[test]
fn a_folded_polygon_overlaps_its_own_piece() {
    let mesh = Primitive::Plane(PlaneParams {
        segments: [2, 2],
        ..Default::default()
    })
    .mesh();
    let w = one(&mesh);
    assert!(w.is_clean());
    // Pull the middle point's UV far past a corner: the polygons round it
    // fold over.
    let middle_point = mesh
        .vertices()
        .map(|(v, _)| v)
        .find(|v| mesh.faces().filter(|(_, l)| l.contains(v)).count() == 4)
        .unwrap();
    let mut m = mesh.clone();
    mesh::uvs::transform_uvs(
        &mut m,
        &[],
        &[middle_point],
        Affine2::from_translation(Vec2::new(0.9, 0.9)),
    );
    let w = one(&m);
    assert!(!w.overlapping.is_empty());
    assert_eq!(w.overlapping_pieces, 1, "one piece, counted once");
    assert_eq!(w.mirrored_pieces, 0);
}

#[test]
fn only_pieces_on_the_same_texture_are_checked_together() {
    let mesh = PrimitiveKind::Cube.standard().mesh();
    let other = moved(
        &mesh,
        &mesh.faces().map(|(f, _)| f).collect::<Vec<_>>(),
        Affine2::from_translation(Vec2::splat(0.013)),
    );
    let same = warnings(&[&mesh, &other], |_, _| 0, &[]);
    assert!(same.iter().all(|w| w.overlapping_pieces == 6), "{same:?}");
    let apart = warnings(&[&mesh, &other], |m, _| m, &[]);
    assert!(apart.iter().all(Warnings::is_clean));
}

#[test]
fn stacked_identical_pieces_and_trim_pieces_overlap_on_purpose() {
    let mesh = PrimitiveKind::Cube.standard().mesh();
    let stacked = warnings(&[&mesh, &mesh], |_, _| 0, &[]);
    assert!(stacked.iter().all(Warnings::is_clean), "{stacked:?}");
    // Two pieces on one row, slid over each other along it (a trim
    // sheet repeats, so that's fine).
    let islands = mesh::uvs::islands(&mesh);
    let mut row = mesh.clone();
    for island in &islands[..2] {
        crate::trim::place(&mut row, island, 0.5, 0.75);
    }
    let shift = middle(&row, &islands[0]).x - middle(&row, &islands[1]).x + 0.05;
    let row = moved(
        &row,
        &islands[1],
        Affine2::from_translation(Vec2::new(shift, 0.0)),
    );
    let w = warnings(&[&row], |_, _| 0, &[(0.5, 0.75)]).remove(0);
    assert!(w.overlapping.is_empty(), "{w:?}");
    let w = warnings(&[&row], |_, _| 0, &[]).remove(0);
    assert!(!w.overlapping.is_empty(), "without the row they overlap");
}

#[test]
fn a_shape_without_uvs_has_no_warnings() {
    let mut mesh = Mesh::new();
    let a = mesh.add_vertex(glam::Vec3::ZERO).unwrap();
    let b = mesh.add_vertex(glam::Vec3::X).unwrap();
    let c = mesh.add_vertex(glam::Vec3::Y).unwrap();
    mesh.add_face(&[a, b, c]).unwrap();
    assert!(one(&mesh).is_clean());
    assert!(mirrored_pieces(&mesh).is_empty());
    assert!(warnings(&[], |_, _| 0, &[]).is_empty());
}
