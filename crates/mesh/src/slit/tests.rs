//! Tests for Slit.

use super::*;
use crate::attributes::{AttributeElement, Domain, Value};
use crate::{MeshReport, PlaneParams, Primitive};
use glam::Vec2;

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

/// A square plane, 100 across, lying flat at z = 0, seen from above.
fn plane() -> Mesh {
    Primitive::Plane(PlaneParams::default()).mesh()
}

/// Looking at the cube's front (y = -50) from in front, at half height.
const FRONT: Eye = Eye::At(Vec3::new(0.0, -500.0, 50.0));
const ABOVE: Eye = Eye::At(Vec3::new(0.0, 0.0, 500.0));

fn line(points: &[[f32; 3]], eye: Eye, through: bool) -> SlitLine {
    SlitLine {
        points: points.iter().map(|p| Vec3::from_array(*p)).collect(),
        eye,
        through,
    }
}

/// Across the cube's front, left to right, at half height.
fn across_front(through: bool) -> SlitLine {
    line(&[[-80.0, -50.0, 50.0], [80.0, -50.0, 50.0]], FRONT, through)
}

fn clean(mesh: &Mesh, before: &Mesh) {
    let (r, b) = (MeshReport::new(mesh), MeshReport::new(before));
    assert!(!r.has_problems(), "{r:?}");
    assert_eq!(r.is_closed(), b.is_closed(), "{r:?}");
    assert_eq!(r.pieces, b.pieces, "{r:?}");
}

#[test]
fn a_line_across_cuts_only_the_polygon_seen() {
    let before = cube();
    let mut mesh = before.clone();
    let out = slit(&mut mesh, &across_front(false));
    assert_eq!(out.matched, 1);
    // The front polygon in two; its two side neighbours gain a corner.
    assert_eq!(mesh.face_count(), 7);
    assert_eq!(mesh.vertex_count(), 10);
    clean(&mesh, &before);
    let new: Vec<Vec3> = mesh.vertices().skip(8).map(|(_, p)| p).collect();
    assert!(
        new.iter()
            .all(|p| (p.z - 50.0).abs() < 1e-3 && (p.y + 50.0).abs() < 1e-3)
    );
    assert_eq!(mesh.faces().filter(|(_, c)| c.len() == 5).count(), 2);
    assert_eq!(slit_size(&before, &across_front(false)), 1);
}

#[test]
fn cutting_through_goes_all_the_way_round() {
    let before = cube();
    let mut mesh = before.clone();
    let out = slit(&mut mesh, &across_front(true));
    // Front, both sides and back.
    assert_eq!(out.matched, 4);
    assert_eq!(mesh.face_count(), 10);
    assert_eq!(mesh.vertex_count(), 12);
    clean(&mesh, &before);
}

#[test]
fn a_flat_view_cuts_the_same() {
    let before = cube();
    let mut mesh = before.clone();
    let mut l = across_front(true);
    l.eye = Eye::Along(Vec3::Y);
    // In a flat view the sides lie edge on: front and back are cut, and the
    // sides take the new points.
    let out = slit(&mut mesh, &l);
    assert!(out.matched >= 2, "{out:?}");
    clean(&mesh, &before);
    let mut seen = before.clone();
    l.through = false;
    assert_eq!(slit(&mut seen, &l).matched, 1);
    clean(&seen, &before);
}

#[test]
fn a_line_can_turn_inside_a_polygon() {
    let before = cube();
    let mut mesh = before.clone();
    let l = line(
        &[
            [-80.0, -50.0, 30.0],
            [0.0, -50.0, 70.0],
            [80.0, -50.0, 30.0],
        ],
        FRONT,
        false,
    );
    assert_eq!(slit(&mut mesh, &l).matched, 1);
    assert_eq!(mesh.face_count(), 7);
    // Two on the front's sides, one inside it.
    assert_eq!(mesh.vertex_count(), 11);
    clean(&mesh, &before);
    // Made in order along the line: in, the turn, out.
    let inner = mesh.position(VertexId::from_raw(9)).unwrap();
    assert!(
        inner.distance(Vec3::new(0.0, -50.0, 70.0)) < 1e-2,
        "{inner}"
    );
}

#[test]
fn corners_snapped_to_cut_straight_between_them() {
    let before = cube();
    let mut mesh = before.clone();
    // Corner to opposite corner of the front.
    let l = line(&[[-50.0, -50.0, 0.0], [50.0, -50.0, 100.0]], FRONT, false);
    assert_eq!(slit(&mut mesh, &l).matched, 1);
    assert_eq!(mesh.face_count(), 7);
    assert_eq!(mesh.vertex_count(), 8);
    assert_eq!(mesh.faces().filter(|(_, c)| c.len() == 3).count(), 2);
    clean(&mesh, &before);
}

#[test]
fn a_line_crossing_twice_cuts_twice() {
    let before = plane();
    let mut mesh = before.clone();
    let l = line(
        &[
            [-80.0, -20.0, 0.0],
            [80.0, -20.0, 0.0],
            [80.0, 20.0, 0.0],
            [-80.0, 20.0, 0.0],
        ],
        ABOVE,
        false,
    );
    assert_eq!(slit(&mut mesh, &l).matched, 2);
    assert_eq!(mesh.face_count(), 3);
    assert_eq!(mesh.vertex_count(), 8);
    clean(&mesh, &before);
}

#[test]
fn what_is_hidden_is_cut_only_through() {
    // The cube, and a square further back hidden behind it, facing the eye.
    let mut before = cube();
    let behind: Vec<VertexId> = [
        [-40.0, 200.0, 10.0],
        [40.0, 200.0, 10.0],
        [40.0, 200.0, 90.0],
        [-40.0, 200.0, 90.0],
    ]
    .iter()
    .map(|p| before.add_vertex(Vec3::from_array(*p)).unwrap())
    .collect();
    let square = before.add_face(&behind).unwrap();
    assert!(before.face_normal(square).y < -0.9, "faces the eye");
    let mut seen = before.clone();
    slit(&mut seen, &across_front(false));
    assert_eq!(seen.face(square).unwrap().len(), 4, "hidden: not cut");
    assert_eq!(seen.face_count(), before.face_count() + 1);
    let mut through = before.clone();
    slit(&mut through, &across_front(true));
    assert_eq!(through.face_count(), before.face_count() + 5);
}

#[test]
fn values_come_along() {
    let mut before = plane();
    before
        .add_attribute("uv", Domain::Corner, Value::Vec2(Vec2::ZERO))
        .unwrap();
    before
        .add_attribute("w", Domain::Point, Value::Float(0.0))
        .unwrap();
    before
        .add_attribute("seam", Domain::Edge, Value::Bool(false))
        .unwrap();
    let faces: Vec<(FaceId, Vec<VertexId>)> =
        before.faces().map(|(f, c)| (f, c.to_vec())).collect();
    for (f, corners) in faces {
        for (i, c) in corners.iter().enumerate() {
            let p = before.position(*c).unwrap();
            before
                .set_attribute(
                    "uv",
                    AttributeElement::Corner(f, i),
                    Value::Vec2(p.truncate()),
                )
                .unwrap();
            before
                .set_attribute("w", AttributeElement::Point(*c), Value::Float(p.x + p.y))
                .unwrap();
        }
    }
    for edge in before.edges().to_vec() {
        before
            .set_attribute("seam", AttributeElement::Edge(edge), Value::Bool(true))
            .unwrap();
    }
    let mut mesh = before.clone();
    let l = line(
        &[[-80.0, -30.0, 0.0], [10.0, 20.0, 0.0], [80.0, -10.0, 0.0]],
        ABOVE,
        false,
    );
    assert_eq!(slit(&mut mesh, &l).matched, 1);
    for (f, corners) in mesh.faces() {
        for (i, c) in corners.iter().enumerate() {
            let p = mesh.position(*c).unwrap();
            let Some(Value::Vec2(uv)) = mesh.attribute("uv", AttributeElement::Corner(f, i)) else {
                panic!("no uv");
            };
            assert!(uv.distance(p.truncate()) < 1e-3, "{f:?} {i}: {uv} at {p}");
            let Some(Value::Float(w)) = mesh.attribute("w", AttributeElement::Point(*c)) else {
                panic!("no w");
            };
            assert!((w - p.x - p.y).abs() < 1e-3, "{w} at {p}");
        }
    }
    // The border's pieces stay seams; the new edges across aren't.
    let seams = mesh
        .edges()
        .iter()
        .filter(|e| mesh.attribute("seam", AttributeElement::Edge(**e)) == Some(Value::Bool(true)))
        .count();
    assert_eq!(seams, 6);
}

#[test]
fn nonsense_lines_change_nothing() {
    let before = cube();
    for l in [
        line(&[[0.0, -50.0, 50.0]], FRONT, false),
        line(&[[f32::NAN, 0.0, 0.0], [1.0, 0.0, 0.0]], FRONT, false),
        line(
            &[[-80.0, -50.0, 50.0], [80.0, -50.0, 50.0]],
            Eye::Along(Vec3::ZERO),
            false,
        ),
        line(
            &[[-80.0, -50.0, 50.0], [80.0, -50.0, 50.0]],
            Eye::At(Vec3::INFINITY),
            true,
        ),
        SlitLine {
            points: vec![Vec3::ZERO; MAX_LINE_POINTS + 1],
            eye: FRONT,
            through: true,
        },
        // Starting inside the front: it only goes out, never across.
        line(&[[0.0, -50.0, 50.0], [80.0, -50.0, 50.0]], FRONT, false),
        // Off to the side: misses.
        line(&[[200.0, -50.0, 50.0], [300.0, -50.0, 50.0]], FRONT, true),
    ] {
        let mut mesh = before.clone();
        let out = slit(&mut mesh, &l);
        assert_eq!(out.skipped, 1, "{l:?}");
        assert_eq!(mesh, before, "{l:?}");
    }
}

#[test]
fn ids_are_kept_and_the_cut_repeats_exactly() {
    let before = cube();
    let mut a = before.clone();
    let mut b = before.clone();
    slit(&mut a, &across_front(true));
    slit(&mut b, &across_front(true));
    assert_eq!(a, b);
    for (f, _) in before.faces() {
        assert!(a.face(f).is_some());
    }
}

#[test]
fn every_shape_cuts_cleanly() {
    let eyes = [
        Eye::At(Vec3::new(300.0, -400.0, 250.0)),
        Eye::At(Vec3::new(-50.0, 30.0, 600.0)),
        Eye::Along(Vec3::new(0.3, 1.0, -0.2)),
    ];
    for primitive in Primitive::all() {
        let before = primitive.mesh();
        let middle = before.bounds().unwrap().centre();
        let mut cuts = 0;
        for (k, eye) in eyes.iter().enumerate() {
            for through in [false, true] {
                let wobble = Vec3::new(13.0, -7.0, 21.0) * k as f32;
                let l = SlitLine {
                    points: vec![
                        middle + Vec3::new(-120.0, 10.0, -40.0) + wobble,
                        middle + Vec3::new(5.0, -3.0, 7.0),
                        middle + Vec3::new(110.0, -20.0, 45.0) - wobble,
                    ],
                    eye: *eye,
                    through,
                };
                let mut mesh = before.clone();
                cuts += slit(&mut mesh, &l).matched;
                let (r, b) = (MeshReport::new(&mesh), MeshReport::new(&before));
                let what = format!("{} eye {k} through {through}: {r:?}", primitive.name());
                assert_eq!(r.flipped_edges, 0, "{what}");
                assert_eq!(r.non_manifold_edges, 0, "{what}");
                assert_eq!(r.loose_points, 0, "{what}");
                assert_eq!(r.is_closed(), b.is_closed(), "{what}");
                for (v, p) in mesh.vertices() {
                    assert!(p.is_finite(), "{what}: {v:?}");
                }
            }
        }
        assert!(cuts >= 3, "{}: only {cuts} cuts", primitive.name());
    }
}

#[test]
fn a_line_can_turn_from_one_polygon_onto_the_next() {
    // Seen from the front right, as the app first opens: across the front,
    // turning inside it, round the corner and across the right side.
    let before = cube();
    let mut mesh = before.clone();
    let l = line(
        &[
            [-80.0, -50.0, 90.0],
            [30.0, -50.0, 40.0],
            [50.0, 0.0, 60.0],
            [50.0, 120.0, 140.0],
        ],
        Eye::At(Vec3::new(560.0, -980.0, 330.0)),
        false,
    );
    assert_eq!(slit(&mut mesh, &l).matched, 2);
    assert_eq!(mesh.face_count(), 8);
    // In, the turn and the corner on the front; the turn and out on the
    // side.
    assert_eq!(mesh.vertex_count(), 13);
    clean(&mesh, &before);
    // Ending inside the side: it is cut only up to the corner's polygon.
    let mut short = before.clone();
    let mut l = l.clone();
    l.points.pop();
    assert_eq!(slit(&mut short, &l).matched, 1);
    clean(&short, &before);
}
