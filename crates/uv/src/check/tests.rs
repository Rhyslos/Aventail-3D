//! Tests for the UV part of the bake check (decision 0105).

use super::*;
use glam::Affine2;
use mesh::{CubeParams, PlaneParams, Primitive, PrimitiveKind};

fn cube() -> Mesh {
    Primitive::Cube(CubeParams::default()).mesh()
}

fn all(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

/// The shape's hard edges, as the export shades them.
fn hard(mesh: &Mesh) -> Vec<EdgeKey> {
    let topology = mesh.topology();
    mesh.hard_edges()
        .filter(|e| topology.has_edge(*e))
        .collect()
}

/// Each UV island of `mesh` as a sheet piece.
fn pieces_of(mesh: &Mesh) -> Vec<SheetPiece<'_>> {
    mesh::uvs::islands(mesh)
        .into_iter()
        .map(|faces| SheetPiece { mesh, faces })
        .collect()
}

/// `mesh` packed with `padding` pixels on a texture of `size`.
fn packed(mesh: &Mesh, size: u32, padding: u32) -> Mesh {
    let islands = crate::pack::islands_of(mesh, &all(mesh));
    let pieces: Vec<crate::pack::Piece> = islands
        .iter()
        .map(|i| crate::pack::piece_of(mesh, i))
        .collect();
    let options = crate::pack::PackOptions {
        padding: padding as f32 / size as f32,
        rotate: true,
        stack: false,
        cells: (size / (padding / 2).max(1)).clamp(128, 512) as usize,
    };
    let moves = crate::pack::pack(&pieces, &options).unwrap();
    let mut out = mesh.clone();
    crate::pack::place(
        &mut out,
        &islands.into_iter().zip(moves).collect::<Vec<_>>(),
    );
    out
}

#[test]
fn every_ready_made_shape_is_clean() {
    for kind in PrimitiveKind::ALL {
        let mesh = kind.standard().mesh();
        let report = UvReport::new(&mesh, &hard(&mesh)).unwrap();
        assert!(report.is_clean(), "{kind:?}: {report:?}");
        // Packed with padding, nothing is too close or over another (a
        // small texture keeps the test quick).
        let packed = packed(&mesh, 512, 4);
        let sheet = check_sheet(&pieces_of(&packed), 512, 4);
        assert!(sheet.is_clean(), "{kind:?}: {sheet:?}");
    }
}

#[test]
fn a_shape_without_uvs_has_no_report() {
    let mut mesh = cube();
    mesh.remove_attribute(mesh::attributes::UV);
    assert!(UvReport::new(&mesh, &[]).is_none());
}

#[test]
fn hard_edges_inside_a_piece_are_found() {
    // A plane in one piece, with an inner edge marked hard.
    let mut plane = Primitive::Plane(PlaneParams {
        segments: [2, 1],
        ..Default::default()
    })
    .mesh();
    let topology = plane.topology();
    let inner: Vec<EdgeKey> = topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 2)
        .collect();
    assert_eq!(inner.len(), 1);
    plane.set_hard(inner[0].ends().0, inner[0].ends().1, true);
    let report = UvReport::new(&plane, &inner).unwrap();
    assert_eq!(report.hard_joined, inner);
    // Open edges are never counted.
    let border = plane.topology().border_edges();
    assert!(
        UvReport::new(&plane, &border)
            .unwrap()
            .hard_joined
            .is_empty()
    );
}

#[test]
fn folded_squashed_outside_and_uneven_pieces_are_found() {
    let mut mesh = cube();
    let faces = all(&mesh);
    // One side mirrored on its own: a whole piece mirrored is fine.
    let one = vec![faces[0]];
    let centre = mesh::uvs::uv_centre(&mesh, &one, &[]).unwrap();
    let mirror = Affine2::from_translation(centre)
        * Affine2::from_scale(Vec2::new(-1.0, 1.0))
        * Affine2::from_translation(-centre);
    mesh::uvs::transform_uvs(&mut mesh, &one, &[], mirror);
    let report = UvReport::new(&mesh, &hard(&mesh)).unwrap();
    assert!(report.folded.is_empty(), "{report:?}");
    // Another side squashed flat: folded.
    let two = vec![faces[1]];
    mesh::uvs::transform_uvs(
        &mut mesh,
        &two,
        &[],
        Affine2::from_scale(Vec2::new(1.0, 0.0)),
    );
    // A third moved out of the square, and a fourth made four times as
    // big (twice the density both ways is four times the area).
    let three = vec![faces[2]];
    mesh::uvs::transform_uvs(&mut mesh, &three, &[], Affine2::from_translation(Vec2::X));
    let four = vec![faces[3]];
    let c4 = mesh::uvs::uv_centre(&mesh, &four, &[]).unwrap();
    let grow = Affine2::from_translation(c4)
        * Affine2::from_scale(Vec2::splat(2.5))
        * Affine2::from_translation(-c4);
    mesh::uvs::transform_uvs(&mut mesh, &four, &[], grow);
    let report = UvReport::new(&mesh, &hard(&mesh)).unwrap();
    assert_eq!(report.folded, two);
    assert!(report.outside.contains(&faces[2]), "{report:?}");
    assert!(report.uneven.contains(&faces[3]), "{report:?}");
    assert!(!report.uneven.contains(&faces[4]), "{report:?}");
    assert!(report.usual_density.is_some());
}

#[test]
fn overlapping_stacked_and_close_pieces_are_told_apart() {
    let a = cube();
    // Two copies of the same layout: every piece lies on its twin.
    let b = a.clone();
    let mut pieces = pieces_of(&a);
    pieces.extend(pieces_of(&b));
    let sheet = check_sheet(&pieces, 2048, 8);
    assert_eq!(sheet.stacked.len(), pieces.len(), "{sheet:?}");
    assert!(sheet.overlapping.is_empty());
    // One copy moved half a side across: overlapping, not stacked.
    let mut c = a.clone();
    let every = all(&c);
    mesh::uvs::transform_uvs(
        &mut c,
        &every,
        &[],
        Affine2::from_translation(Vec2::new(0.07, 0.0)),
    );
    let mut pieces = pieces_of(&a);
    pieces.extend(pieces_of(&c));
    let sheet = check_sheet(&pieces, 2048, 8);
    assert!(!sheet.overlapping.is_empty(), "{sheet:?}");
    // Packed, then one piece nudged up to another, 1 px away, on a small
    // texture with 4 px padding.
    let p = packed(&a, 512, 4);
    let pieces = pieces_of(&p);
    assert!(check_sheet(&pieces, 512, 4).is_clean());
    let (min_a, max_a) = bounds(&p, &pieces[0].faces);
    let (min_b, _) = bounds(&p, &pieces[1].faces);
    let mut q = p.clone();
    // The second piece's lower left corner put 1 px right of the first's
    // right side, level with it.
    let shift = Vec2::new(max_a.x + 1.0 / 512.0, min_a.y) - min_b;
    mesh::uvs::transform_uvs(
        &mut q,
        &pieces[1].faces,
        &[],
        Affine2::from_translation(shift),
    );
    let sheet = check_sheet(&pieces_of(&q), 512, 4);
    assert!(sheet.overlapping.is_empty(), "{sheet:?}");
    assert!(
        sheet.too_close.contains(&0) && sheet.too_close.contains(&1),
        "{sheet:?}"
    );
    // No padding asked: nothing is too close.
    assert!(check_sheet(&pieces_of(&q), 512, 0).too_close.is_empty());
}

fn bounds(mesh: &Mesh, faces: &[FaceId]) -> (Vec2, Vec2) {
    mesh::uvs::uv_bounds(mesh, faces, &[]).unwrap()
}

#[test]
fn empty_and_odd_input_is_safe() {
    assert!(check_sheet(&[], 2048, 8).is_clean());
    let mesh = cube();
    let none = SheetPiece {
        mesh: &mesh,
        faces: Vec::new(),
    };
    assert!(check_sheet(&[none], 2048, 8).is_clean());
    // A tiny texture and huge padding.
    let pieces = pieces_of(&mesh);
    let _ = check_sheet(&pieces, 1, 256);
    let _ = check_sheet(&pieces, 16384, 256);
}

/// `cargo test --release -p uv check_benchmark -- --ignored --nocapture`.
#[test]
#[ignore = "benchmark"]
fn check_benchmark() {
    for n in [32u32, 128, 256] {
        let plane = Primitive::Plane(PlaneParams {
            segments: [n, n],
            ..Default::default()
        })
        .mesh();
        // Every polygon a piece of its own, packed.
        let mut cut = plane.clone();
        let topology = cut.topology();
        let inner: Vec<EdgeKey> = topology
            .edges()
            .filter(|e| topology.edge_uses(*e).len() == 2)
            .collect();
        mesh::uvs::set_seams(&mut cut, &inner, true);
        let every = all(&cut);
        crate::unwrap(&mut cut, &every, true);
        let p = packed(&cut, 2048, 8);
        let start = std::time::Instant::now();
        let report = UvReport::new(&p, &[]).unwrap();
        let sheet = check_sheet(&pieces_of(&p), 2048, 8);
        println!(
            "{} pieces: checked in {:?} ({} uneven, {} too close)",
            n * n,
            start.elapsed(),
            report.uneven.len(),
            sheet.too_close.len()
        );
    }
}
