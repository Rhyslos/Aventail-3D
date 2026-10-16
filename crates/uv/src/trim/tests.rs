//! Tests for laying strips onto trim rows (decision 0106).

use super::*;
use mesh::{CubeParams, CylinderParams, PlaneParams, Primitive, PrimitiveKind};

fn all(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

fn uv_box(mesh: &Mesh, faces: &[FaceId]) -> (Vec2, Vec2) {
    mesh::uvs::uv_bounds(mesh, faces, &[]).unwrap()
}

#[test]
fn a_strip_lies_straight_along_a_row_filling_its_height() {
    // A long thin plane, two by eight polygons, its UVs turned on end.
    let mut strip = Primitive::Plane(PlaneParams {
        width: 400.0,
        depth: 50.0,
        segments: [8, 2],
    })
    .mesh();
    let faces = all(&strip);
    mesh::uvs::transform_uvs(&mut strip, &faces, &[], glam::Affine2::from_angle(1.2));
    let done = place(&mut strip, &faces, 0.25, 0.375);
    assert_eq!(done.matched, faces.len());
    let (min, max) = uv_box(&strip, &faces);
    assert!(
        (min.y - 0.25).abs() < 1e-5 && (max.y - 0.375).abs() < 1e-5,
        "{min} {max}"
    );
    assert!(min.x.abs() < 1e-5, "it starts at the row's left end");
    // The long side along the row: eight times as long as tall.
    assert!(((max.x - min.x) / (max.y - min.y) - 8.0).abs() < 1e-3);
    assert!(on_row(&strip, &faces, 0.25, 0.375));
    assert!(!on_row(&strip, &faces, 0.0, 0.125));
    // A straight grid: every polygon a rectangle of the same size.
    let uvs = Uvs::of(&strip).unwrap();
    for f in &faces {
        let n = strip.face(*f).unwrap().len();
        let c: Vec<Vec2> = (0..n).map(|k| uvs.corner(&strip, *f, k).unwrap()).collect();
        for k in 0..n {
            let side = c[(k + 1) % n] - c[k];
            assert!(side.x.abs() < 1e-5 || side.y.abs() < 1e-5, "{side}");
        }
    }
    // Not mirrored.
    let stretch = crate::stretch::stretch_of(&strip).unwrap();
    assert!(stretch.iter().all(|(_, s)| !s.flipped));
}

#[test]
fn several_pieces_follow_one_another_along_the_row() {
    let mut cube = Primitive::Cube(CubeParams::default()).mesh();
    let faces = all(&cube);
    place(&mut cube, &faces[..3], 0.5, 0.75);
    let mut ends: Vec<(f32, f32)> = faces[..3]
        .iter()
        .map(|f| {
            let (min, max) = uv_box(&cube, &[*f]);
            assert!((min.y - 0.5).abs() < 1e-5 && (max.y - 0.75).abs() < 1e-5);
            (min.x, max.x)
        })
        .collect();
    ends.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert!(ends[0].0.abs() < 1e-5);
    for pair in ends.windows(2) {
        assert!((pair[0].1 - pair[1].0).abs() < 1e-5, "{ends:?}");
    }
    // The other sides are left alone.
    let fresh = Primitive::Cube(CubeParams::default()).mesh();
    for f in &faces[3..] {
        assert_eq!(uv_box(&cube, &[*f]), uv_box(&fresh, &[*f]));
    }
}

#[test]
fn a_cylinders_side_comes_out_as_one_straight_strip() {
    let mut cylinder = Primitive::Cylinder(CylinderParams::default()).mesh();
    let side: Vec<FaceId> = all(&cylinder)
        .into_iter()
        .filter(|f| cylinder.face_normal(*f).z.abs() < 0.5)
        .collect();
    place(&mut cylinder, &side[..1], 0.0, 0.125);
    let (min, max) = uv_box(&cylinder, &side);
    assert!(min.y.abs() < 1e-5 && (max.y - 0.125).abs() < 1e-5);
    // The whole side, along the row: as long as its 16 sides around
    // (radius 50) against its height of 100.
    let around = 16.0 * 2.0 * 50.0 * (std::f32::consts::PI / 16.0).sin();
    let aspect = (max.x - min.x) / (max.y - min.y);
    assert!((aspect - around / 100.0).abs() < 1e-3, "{aspect}");
}

#[test]
fn every_primitive_and_bad_rows_are_safe() {
    for kind in PrimitiveKind::ALL {
        let mut mesh = kind.standard().mesh();
        let faces = all(&mesh);
        place(&mut mesh, &faces, 0.1, 0.2);
        let uvs = Uvs::of(&mesh).unwrap();
        let count = mesh.face_layout().1.len();
        assert!(
            uvs.into_values(count).iter().all(|v| v.is_finite()),
            "{kind:?}"
        );
    }
    let fresh = Primitive::Cube(CubeParams::default()).mesh();
    for (bottom, top) in [
        (0.5, 0.5),
        (0.6, 0.5),
        (f32::NAN, 0.5),
        (0.0, f32::INFINITY),
    ] {
        let mut mesh = fresh.clone();
        let faces = all(&mesh);
        let done = place(&mut mesh, &faces, bottom, top);
        assert_eq!(done.matched, 0);
        assert_eq!(mesh, fresh);
    }
    let mut mesh = fresh.clone();
    assert_eq!(
        place(&mut mesh, &[FaceId::from_raw(99)], 0.0, 0.5).missing,
        1
    );
}
