//! Tests for re-applying UV intents (decision 0248).

use super::*;
use glam::Vec3;
use mesh::uv_intents::set_intent;

/// One upright square (in the XZ plane), its UVs the unit square turned
/// a quarter, so the world's up runs across the texture.
fn upright_square() -> Mesh {
    let mut mesh = Mesh::new();
    let v: Vec<_> = [
        Vec3::ZERO,
        Vec3::X,
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::Z,
    ]
    .into_iter()
    .map(|p| mesh.add_vertex(p).unwrap())
    .collect();
    mesh.add_face(&v).unwrap();
    // Up (+Z) runs along -U.
    let uvs = vec![
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 1.0),
        Vec2::ZERO,
    ];
    assert!(mesh::uvs::set_uvs(&mut mesh, uvs));
    mesh
}

fn faces(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

/// Scales every UV by `by` about the origin (a change an edit could make).
fn squash(mesh: &mut Mesh, by: f32) {
    let all = faces(mesh);
    mesh::uvs::transform_uvs(mesh, &all, &[], Affine2::from_scale(Vec2::splat(by)));
}

#[test]
fn an_upright_piece_is_turned_so_up_runs_up_the_texture() {
    let mut mesh = upright_square();
    let all = faces(&mesh);
    set_intent(&mut mesh, &all, UvIntent::Upright);
    let before = mesh.clone();
    squash(&mut mesh, 0.5);
    assert_eq!(reapply(&before, &mut mesh), 1);
    let up = up_direction(&mesh, &all).unwrap();
    assert!(up.distance(Vec2::Y) < 1e-4, "{up:?}");
}

#[test]
fn a_trim_row_piece_is_fitted_back_into_its_row() {
    let mut mesh = upright_square();
    let all = faces(&mesh);
    set_intent(&mut mesh, &all, UvIntent::TrimRow { bottom: 0.25, top: 0.5 });
    let before = mesh.clone();
    squash(&mut mesh, 0.8);
    assert_eq!(reapply(&before, &mut mesh), 1);
    let (min, max) = bounds(&mesh, &all).unwrap();
    assert!((min.y - 0.25).abs() < 1e-5 && (max.y - 0.5).abs() < 1e-5, "{min:?} {max:?}");
}

#[test]
fn a_density_piece_gets_its_density_back() {
    let mut mesh = upright_square();
    let all = faces(&mesh);
    set_intent(&mut mesh, &all, UvIntent::Density(0.5));
    let before = mesh.clone();
    squash(&mut mesh, 0.3);
    reapply(&before, &mut mesh);
    let d = crate::density::density(&mesh, &all).unwrap();
    assert!((d - 0.5).abs() < 1e-3, "{d}");
}

#[test]
fn pieces_without_intents_or_untouched_ones_are_left_alone() {
    let mut mesh = upright_square();
    let before = mesh.clone();
    squash(&mut mesh, 0.5);
    let squashed = mesh.clone();
    assert_eq!(reapply(&before, &mut mesh), 0);
    assert_eq!(mesh, squashed);
    // With an intent, but nothing changed.
    let all = faces(&mesh);
    set_intent(&mut mesh, &all, UvIntent::Upright);
    let same = mesh.clone();
    assert_eq!(reapply(&same, &mut mesh), 0);
}

#[test]
fn locked_pieces_keep_what_they_are() {
    let mut mesh = upright_square();
    let all = faces(&mesh);
    set_intent(&mut mesh, &all, UvIntent::Upright);
    mesh::uvs::set_locked(&mut mesh, &all, true);
    let before = mesh.clone();
    squash(&mut mesh, 0.5);
    assert_eq!(reapply(&before, &mut mesh), 0);
}

#[test]
fn a_flat_lying_piece_has_no_up() {
    let mut mesh = upright_square();
    // Lay it flat: every point at height 0.
    let points: Vec<_> = mesh.vertices().map(|(v, _)| v).collect();
    for v in points {
        let p = mesh.position(v).unwrap();
        assert!(mesh.set_position(v, Vec3::new(p.x, p.z, 0.0)));
    }
    let all = faces(&mesh);
    assert!(up_direction(&mesh, &all).is_none());
    assert!(!apply(&mut mesh, &all, UvIntent::Upright));
}
