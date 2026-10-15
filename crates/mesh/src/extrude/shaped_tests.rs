//! Tests for shaped extrudes (decision 0070): straight out, along
//! normals, along an axis, end size, steps and a flat end.

use super::*;
use crate::{CylinderParams, MeshReport, Primitive};

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

fn cylinder() -> Mesh {
    Primitive::Cylinder(CylinderParams {
        height_segments: 2,
        ..Default::default()
    })
    .mesh()
}

/// The polygon facing most along `direction`.
fn facing(mesh: &Mesh, direction: Vec3) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .max_by(|a, b| {
            mesh.face_normal(*a)
                .dot(direction)
                .total_cmp(&mesh.face_normal(*b).dot(direction))
        })
        .unwrap()
}

/// The side polygons of the cylinder facing within `cos` of `direction`.
fn side_patch(mesh: &Mesh, direction: Vec3, cos: f32) -> Vec<FaceId> {
    mesh.faces()
        .map(|(f, _)| f)
        .filter(|f| {
            let n = mesh.face_normal(*f);
            n.z.abs() < 0.1 && n.dot(direction) > cos
        })
        .collect()
}

fn corners(mesh: &Mesh, face: FaceId) -> Vec<Vec3> {
    mesh.face(face)
        .unwrap()
        .iter()
        .map(|v| mesh.position(*v).unwrap())
        .collect()
}

fn centre(mesh: &Mesh, face: FaceId) -> Vec3 {
    let c = corners(mesh, face);
    c.iter().copied().sum::<Vec3>() / c.len() as f32
}

fn area(mesh: &Mesh, face: FaceId) -> f32 {
    facing_by_size(mesh, face).length() / 2.0
}

fn shaped(direction: ExtrudeDirection) -> ExtrudeShape {
    ExtrudeShape {
        direction,
        ..ExtrudeShape::default()
    }
}

/// How each selected polygon's corners moved: end corner minus start.
fn moves(before: &Mesh, after: &Mesh, faces: &[FaceId]) -> Vec<Vec3> {
    faces
        .iter()
        .flat_map(|f| {
            corners(before, *f)
                .into_iter()
                .zip(corners(after, *f))
                .map(|(a, b)| b - a)
        })
        .collect()
}

fn healthy(mesh: &Mesh) {
    for (v, p) in mesh.vertices() {
        assert!(p.is_finite(), "point {v:?} is {p}");
    }
    let report = MeshReport::new(mesh);
    assert_eq!(report.non_manifold_edges, 0, "{report:?}");
}

#[test]
fn straight_out_moves_a_curved_patch_all_one_way() {
    let start = cylinder();
    let patch = side_patch(&start, Vec3::X, 0.8);
    assert!(patch.len() >= 4, "{}", patch.len());
    let mut straight = start.clone();
    extrude_shaped(
        &mut straight,
        &patch,
        20.0,
        Grouping::Together,
        ExtrudeShape::default(),
    );
    let moved = moves(&start, &straight, &patch);
    for m in &moved {
        assert!(m.distance(moved[0]) < 1e-3, "{m} vs {}", moved[0]);
    }
    // Straight out along the patch's average facing: +X here.
    assert!(moved[0].normalize().dot(Vec3::X) > 0.999, "{}", moved[0]);
    assert!((moved[0].length() - 20.0).abs() < 1e-3);
    healthy(&straight);
    // Along normals, the points fan out as before.
    let mut fanned = start.clone();
    extrude_shaped(
        &mut fanned,
        &patch,
        20.0,
        Grouping::Together,
        ExtrudeShape::EARLIER,
    );
    let spread = moves(&start, &fanned, &patch);
    assert!(spread.iter().any(|m| m.distance(spread[0]) > 1.0));
    // ... exactly as the first extrude did.
    let mut first = start.clone();
    extrude_polygons(&mut first, &patch, 20.0, Grouping::Together);
    assert_eq!(first, fanned);
}

#[test]
fn a_corner_piece_goes_out_between_its_facings() {
    let start = cube();
    let piece = [facing(&start, Vec3::Z), facing(&start, Vec3::X)];
    let mut mesh = start.clone();
    extrude_shaped(
        &mut mesh,
        &piece,
        10.0,
        Grouping::Together,
        ExtrudeShape::default(),
    );
    let moved = moves(&start, &mesh, &piece);
    let way = (Vec3::X + Vec3::Z).normalize();
    for m in &moved {
        assert!(m.distance(way * 10.0) < 1e-3, "{m}");
    }
    healthy(&mesh);
}

#[test]
fn along_an_axis_goes_out_the_way_the_piece_faces() {
    let start = cube();
    for (face_way, axis, expected) in [
        (Vec3::Z, ExtrudeDirection::AlongZ, Vec3::Z),
        (-Vec3::Z, ExtrudeDirection::AlongZ, -Vec3::Z),
        (Vec3::X, ExtrudeDirection::AlongX, Vec3::X),
        // Across the axis: the positive way.
        (Vec3::X, ExtrudeDirection::AlongY, Vec3::Y),
    ] {
        let face = facing(&start, face_way);
        let mut mesh = start.clone();
        extrude_shaped(&mut mesh, &[face], 5.0, Grouping::Together, shaped(axis));
        for m in moves(&start, &mesh, &[face]) {
            assert!(m.distance(expected * 5.0) < 1e-3, "{axis:?}: {m}");
        }
        healthy(&mesh);
    }
    // A sloped patch pulled out level along X.
    let start = cylinder();
    let patch = side_patch(&start, (Vec3::X + Vec3::Y).normalize(), 0.9);
    let mut mesh = start.clone();
    extrude_shaped(
        &mut mesh,
        &patch,
        8.0,
        Grouping::Together,
        shaped(ExtrudeDirection::AlongX),
    );
    for m in moves(&start, &mesh, &patch) {
        assert!(m.distance(Vec3::X * 8.0) < 1e-3, "{m}");
    }
}

#[test]
fn the_end_size_tapers_around_the_ends_middle() {
    let start = cube();
    let top = facing(&start, Vec3::Z);
    let mut mesh = start.clone();
    let shape = ExtrudeShape {
        end_size: 0.5,
        ..ExtrudeShape::default()
    };
    extrude_shaped(&mut mesh, &[top], 30.0, Grouping::Together, shape);
    assert!((area(&mesh, top) - area(&start, top) * 0.25).abs() < 1e-2);
    assert!(centre(&mesh, top).distance(centre(&start, top) + Vec3::Z * 30.0) < 1e-3);
    healthy(&mesh);
    // Down to a point: still whole numbers, the walls meeting at the tip.
    let mut tip = start.clone();
    let point = ExtrudeShape {
        end_size: 0.0,
        ..ExtrudeShape::default()
    };
    extrude_shaped(&mut tip, &[top], 30.0, Grouping::Together, point);
    for p in corners(&tip, top) {
        assert!(p.distance(centre(&start, top) + Vec3::Z * 30.0) < 1e-3);
    }
}

#[test]
fn steps_make_rows_evenly_along_the_walls() {
    let start = cube();
    let top = facing(&start, Vec3::Z);
    let mut mesh = start.clone();
    let shape = ExtrudeShape {
        steps: 3,
        end_size: 0.4,
        ..ExtrudeShape::default()
    };
    extrude_shaped(&mut mesh, &[top], 30.0, Grouping::Together, shape);
    // Four walls in each of three rows.
    assert_eq!(mesh.face_count(), start.face_count() + 12);
    assert_eq!(mesh.vertex_count(), start.vertex_count() + 12);
    // Rows at a third and two thirds of the way, tapering in a line.
    let base = start.bounds().unwrap().max.z;
    let mut heights: Vec<i32> = mesh
        .vertices()
        .map(|(_, p)| (p.z - base).round() as i32)
        .filter(|h| *h > 0)
        .collect();
    heights.sort_unstable();
    heights.dedup();
    assert_eq!(heights, vec![10, 20, 30]);
    healthy(&mesh);
}

#[test]
fn a_flat_end_lies_square_to_the_way_it_goes() {
    let start = cylinder();
    let patch = side_patch(&start, Vec3::X, 0.7);
    let mut mesh = start.clone();
    let shape = ExtrudeShape {
        flat_end: true,
        ..ExtrudeShape::default()
    };
    extrude_shaped(&mut mesh, &patch, 10.0, Grouping::Together, shape);
    let xs: Vec<f32> = patch
        .iter()
        .flat_map(|f| corners(&mesh, *f))
        .map(|p| p.x)
        .collect();
    let (lo, hi) = xs
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), x| (a.min(*x), b.max(*x)));
    assert!(hi - lo < 1e-3, "flat across X: {lo}..{hi}");
    healthy(&mesh);
}

#[test]
fn polygons_each_on_their_own_keep_apart_with_steps() {
    let start = cylinder();
    let patch = side_patch(&start, Vec3::X, 0.8);
    let mut mesh = start.clone();
    let shape = ExtrudeShape {
        steps: 2,
        end_size: 0.8,
        ..ExtrudeShape::default()
    };
    extrude_shaped(&mut mesh, &patch, 10.0, Grouping::Separate, shape);
    for f in &patch {
        let way = start.face_normal(*f);
        let moved = centre(&mesh, *f) - centre(&start, *f);
        assert!(moved.distance(way * 10.0) < 1e-3, "{moved}");
    }
    healthy(&mesh);
}

#[test]
fn the_same_shaped_edit_gives_the_same_ids() {
    let start = cylinder();
    let patch = side_patch(&start, Vec3::Y, 0.5);
    let shape = ExtrudeShape {
        direction: ExtrudeDirection::AlongZ,
        end_size: 1.3,
        steps: 4,
        flat_end: true,
    };
    let mut a = start.clone();
    let mut b = start.clone();
    extrude_shaped(&mut a, &patch, 12.0, Grouping::Together, shape);
    extrude_shaped(&mut b, &patch, 12.0, Grouping::Together, shape);
    assert_eq!(a, b);
    healthy(&a);
}

#[test]
fn broken_numbers_and_missing_polygons_are_handled() {
    let start = cube();
    let top = facing(&start, Vec3::Z);
    let mut mesh = start.clone();
    let outcome = extrude_shaped(
        &mut mesh,
        &[top],
        f32::NAN,
        Grouping::Together,
        ExtrudeShape::default(),
    );
    assert_eq!(mesh, start);
    assert_eq!(outcome.matched, 1);
    let outcome = extrude_shaped(
        &mut mesh,
        &[FaceId::from_raw(9_999)],
        5.0,
        Grouping::Together,
        ExtrudeShape::default(),
    );
    assert_eq!((outcome.matched, outcome.missing), (0, 1));
    // Settings out of range are brought in.
    let wild = ExtrudeShape {
        end_size: f32::NAN,
        steps: 0,
        ..ExtrudeShape::default()
    }
    .clamped();
    assert_eq!((wild.end_size, wild.steps), (1.0, 1));
    let huge = ExtrudeShape {
        end_size: 1e9,
        steps: 10_000,
        ..ExtrudeShape::default()
    }
    .clamped();
    assert_eq!(
        (huge.end_size, huge.steps),
        (MAX_END_SIZE, MAX_EXTRUDE_STEPS)
    );
    // A whole closed shape, straight out: moves, never breaks.
    let mut whole = start.clone();
    let all: Vec<FaceId> = start.faces().map(|(f, _)| f).collect();
    extrude_shaped(
        &mut whole,
        &all,
        5.0,
        Grouping::Together,
        ExtrudeShape::default(),
    );
    healthy(&whole);
    // An n-gon cap pushed in with steps.
    let mut cyl = cylinder();
    let cap = facing(&cyl, Vec3::Z);
    let shape = ExtrudeShape {
        steps: 2,
        ..ExtrudeShape::default()
    };
    extrude_shaped(&mut cyl, &[cap], -10.0, Grouping::Together, shape);
    healthy(&cyl);
}
