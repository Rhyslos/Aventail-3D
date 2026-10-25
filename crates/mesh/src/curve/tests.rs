//! Tests for curves, sweeps and lathes: normal shapes, every degenerate
//! curve we can think of, closed and outward-facing solids, UVs and hard
//! edges, and a random run that must never panic.

use super::*;
use crate::attributes::{AttributeElement, UV};
use crate::cleanup::faces_facing_wrong;
use crate::polygon::EdgeKey;
use crate::shading::shaded_triangles;
use std::collections::BTreeMap;

/// How many faces use each edge.
fn edge_use(mesh: &Mesh) -> BTreeMap<EdgeKey, usize> {
    let mut uses = BTreeMap::new();
    for (_, corners) in mesh.faces() {
        for i in 0..corners.len() {
            let key = EdgeKey::new(corners[i], corners[(i + 1) % corners.len()]);
            *uses.entry(key).or_insert(0) += 1;
        }
    }
    uses
}

/// Every edge is shared by exactly two faces.
fn is_closed(mesh: &Mesh) -> bool {
    !mesh.faces().next().is_none() && edge_use(mesh).values().all(|n| *n == 2)
}

/// The volume inside (positive when faces look outward).
fn volume(mesh: &Mesh) -> f32 {
    shaded_triangles(mesh)
        .iter()
        .map(|t| t.positions[0].dot(t.positions[1].cross(t.positions[2])) / 6.0)
        .sum()
}

use crate::Mesh;

/// A closed solid facing outward everywhere, with UVs inside the square.
fn assert_solid(mesh: &Mesh, what: &str) {
    assert!(is_closed(mesh), "{what}: not closed");
    assert!(
        faces_facing_wrong(mesh).is_empty(),
        "{what}: faces disagree"
    );
    assert!(
        volume(mesh) > 0.0,
        "{what}: faces inward ({})",
        volume(mesh)
    );
    assert_uvs(mesh, what);
}

/// Every corner has a UV inside the texture square.
fn assert_uvs(mesh: &Mesh, what: &str) {
    for (face, corners) in mesh.faces() {
        for corner in 0..corners.len() {
            let Some(crate::Value::Vec2(uv)) =
                mesh.attribute(UV, AttributeElement::Corner(face, corner))
            else {
                panic!("{what}: a corner of {face:?} has no UV");
            };
            assert!(
                uv.cmpge(glam::Vec2::splat(-1e-4)).all()
                    && uv.cmple(glam::Vec2::splat(1.0001)).all(),
                "{what}: UV {uv} outside the square"
            );
        }
    }
}

fn smooth(p: Vec3, out: Vec3) -> (Vec3, Vec3, Vec3, bool) {
    (p, -out, out, true)
}

fn line(a: Vec3, b: Vec3) -> Curve {
    Curve::corners(&[a, b], false).unwrap()
}

fn pipe() -> SweepShape {
    SweepShape::default()
}

// --- Curves -------------------------------------------------------------

#[test]
fn a_span_runs_from_point_to_point_through_its_handles() {
    let curve = Curve::new(
        [
            (Vec3::ZERO, Vec3::ZERO, Vec3::new(0.0, 50.0, 0.0), false),
            (
                Vec3::new(100.0, 0.0, 0.0),
                Vec3::new(0.0, 50.0, 0.0),
                Vec3::ZERO,
                false,
            ),
        ],
        false,
    )
    .unwrap();
    let span = curve.spans()[0];
    assert_eq!(span.at(0.0), Vec3::ZERO);
    assert!(span.at(1.0).distance(Vec3::new(100.0, 0.0, 0.0)) < 1e-4);
    // Pulled towards +Y by both handles.
    assert!(span.at(0.5).y > 30.0);
    assert!(!span.is_straight());
    assert!(line(Vec3::ZERO, Vec3::X * 10.0).spans()[0].is_straight());
}

#[test]
fn closed_curves_have_a_span_back_to_the_start() {
    let square = [Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::Y];
    assert_eq!(Curve::corners(&square, false).unwrap().spans().len(), 3);
    let closed = Curve::corners(&square, true).unwrap();
    assert_eq!(closed.spans().len(), 4);
    let drawn = closed.polyline(8);
    assert_eq!(drawn.first(), drawn.last(), "a loop ends where it began");
    assert_eq!(drawn.len(), 5, "straight spans are drawn in one piece");
}

#[test]
fn bad_curves_are_refused() {
    let nan = Vec3::new(f32::NAN, 0.0, 0.0);
    assert_eq!(
        Curve::corners(&[Vec3::ZERO, nan], false),
        Err(CurveError::NotANumber)
    );
    let far = Vec3::splat(MAX_COORDINATE * 2.0);
    assert_eq!(
        Curve::corners(&[Vec3::ZERO, far], false),
        Err(CurveError::NotANumber)
    );
    let many = vec![Vec3::ZERO; MAX_CURVE_POINTS + 1];
    assert_eq!(Curve::corners(&many, false), Err(CurveError::TooManyPoints));
    let point = |id| CurvePoint {
        id: CurvePointId::from_raw(id),
        position: Vec3::ZERO,
        handle_in: Vec3::ZERO,
        handle_out: Vec3::ZERO,
        kind: PointKind::Sharp,
    };
    assert_eq!(
        Curve::from_parts(vec![point(1), point(1)], false, 5),
        Err(CurveError::BadIds)
    );
    assert_eq!(
        Curve::from_parts(vec![point(7)], false, 5),
        Err(CurveError::BadIds)
    );
    assert_eq!(
        Curve::from_parts(vec![point(0)], false, MAX_POINT_ID + 1),
        Err(CurveError::BadIds)
    );
    assert!(!CurveError::TooFewPoints.to_string().is_empty());
}

#[test]
fn parts_are_named_by_small_numbers_both_ways() {
    for id in [0, 1, 7, MAX_POINT_ID - 1] {
        for kind in [PartKind::Point, PartKind::In, PartKind::Out] {
            let part = CurvePart {
                point: CurvePointId::from_raw(id),
                kind,
            };
            assert_eq!(CurvePart::from_code(part.code()), part);
        }
    }
}

#[test]
fn only_handles_that_pull_are_parts() {
    let curve = Curve::new(
        [
            (Vec3::ZERO, Vec3::ZERO, Vec3::X, false),
            smooth(Vec3::new(10.0, 0.0, 0.0), Vec3::Y),
        ],
        false,
    )
    .unwrap();
    // First: the point and its out handle; second: point and both.
    assert_eq!(curve.parts().len(), 5);
    let out = CurvePart {
        point: CurvePointId::from_raw(0),
        kind: PartKind::Out,
    };
    assert_eq!(curve.part_position(out), Some(Vec3::X));
}

#[test]
fn parts_become_loose_points_named_by_their_codes() {
    let curve = Curve::new(
        [
            (Vec3::ZERO, Vec3::ZERO, Vec3::X, false),
            smooth(Vec3::new(10.0, 0.0, 0.0), Vec3::Y),
        ],
        false,
    )
    .unwrap();
    let mesh = curve.parts_mesh();
    assert_eq!(mesh.vertex_count(), 5);
    assert_eq!(mesh.face_count(), 0);
    let out = CurvePart {
        point: CurvePointId::from_raw(1),
        kind: PartKind::Out,
    };
    assert_eq!(
        mesh.position(crate::VertexId::from_raw(out.code())),
        Some(Vec3::new(10.0, 1.0, 0.0))
    );
    assert_eq!(Curve::default().parts_mesh().vertex_count(), 0);
}

#[test]
fn moving_a_point_brings_its_handles_and_turning_turns_them() {
    let curve = Curve::new(
        [smooth(Vec3::ZERO, Vec3::X), smooth(Vec3::Y * 10.0, Vec3::X)],
        false,
    )
    .unwrap();
    let id = CurvePointId::from_raw(0);
    let moved = curve
        .edited(&CurveEdit::Transform {
            parts: vec![CurvePart::point(id)],
            matrix: Mat4::from_translation(Vec3::Z * 5.0),
        })
        .unwrap();
    assert_eq!(moved.point(id).unwrap().position, Vec3::Z * 5.0);
    assert_eq!(moved.point(id).unwrap().handle_out, Vec3::X);
    let turned = curve
        .edited(&CurveEdit::Transform {
            parts: vec![CurvePart::point(id)],
            matrix: Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2),
        })
        .unwrap();
    assert!(turned.point(id).unwrap().handle_out.distance(Vec3::Y) < 1e-5);
    // The other point stays.
    assert_eq!(turned.points()[1], curve.points()[1]);
}

#[test]
fn moving_one_handle_of_a_smooth_point_turns_its_partner() {
    let curve = Curve::new(
        [smooth(Vec3::ZERO, Vec3::X * 2.0), smooth(Vec3::Y, Vec3::X)],
        false,
    )
    .unwrap();
    let handle = CurvePart {
        point: CurvePointId::from_raw(0),
        kind: PartKind::Out,
    };
    let moved = curve
        .edited(&CurveEdit::Transform {
            parts: vec![handle],
            matrix: Mat4::from_translation(Vec3::new(-2.0, 2.0, 0.0)),
        })
        .unwrap();
    let p = moved.points()[0];
    assert_eq!(p.handle_out, Vec3::new(0.0, 2.0, 0.0));
    assert!(
        p.handle_in.distance(Vec3::new(0.0, -2.0, 0.0)) < 1e-5,
        "{}",
        p.handle_in
    );
    // A sharp point's handles move alone.
    let sharp = curve
        .edited(&CurveEdit::SetKind {
            points: vec![CurvePointId::from_raw(1)],
            kind: if false {
                PointKind::Smooth
            } else {
                PointKind::Sharp
            },
        })
        .unwrap();
    assert!(!sharp.points()[1].has_handles());
}

#[test]
fn a_transform_that_is_not_numbers_is_refused() {
    let curve = line(Vec3::ZERO, Vec3::X);
    let bad = Mat4::from_translation(Vec3::splat(f32::NAN));
    assert_eq!(curve.transformed(bad), Err(CurveError::NotANumber));
    let far = Mat4::from_scale(Vec3::splat(1e12));
    assert_eq!(
        Curve::corners(&[Vec3::ZERO, Vec3::X * 10.0], false)
            .unwrap()
            .transformed(far),
        Err(CurveError::NotANumber)
    );
}

#[test]
fn deleting_points_and_handles() {
    let curve = Curve::new(
        [
            smooth(Vec3::ZERO, Vec3::X),
            smooth(Vec3::X * 10.0, Vec3::X),
            smooth(Vec3::X * 20.0, Vec3::X),
        ],
        true,
    )
    .unwrap();
    let id = |n| CurvePointId::from_raw(n);
    let fewer = curve
        .edited(&CurveEdit::Delete {
            parts: vec![CurvePart::point(id(1))],
        })
        .unwrap();
    assert_eq!(fewer.points().len(), 2);
    assert!(!fewer.closed(), "two points can't make a loop");
    // Ids stay: the third point keeps its own.
    assert_eq!(fewer.points()[1].id, id(2));
    assert_eq!(
        fewer.edited(&CurveEdit::Delete {
            parts: vec![CurvePart::point(id(2))]
        }),
        Err(CurveError::TooFewPoints)
    );
    // A handle on its own goes back to nothing, and the point turns sharp.
    let handle = CurvePart {
        point: id(0),
        kind: PartKind::In,
    };
    let cut = curve
        .edited(&CurveEdit::Delete {
            parts: vec![handle],
        })
        .unwrap();
    assert_eq!(cut.points()[0].handle_in, Vec3::ZERO);
    assert_eq!(cut.points()[0].kind, PointKind::Broken);
    assert_eq!(cut.points().len(), 3);
    // Unknown ids change nothing.
    let same = curve
        .edited(&CurveEdit::Delete {
            parts: vec![CurvePart::point(id(99))],
        })
        .unwrap();
    assert_eq!(same, curve);
}

#[test]
fn making_a_corner_smooth_gives_it_handles_along_its_neighbours() {
    let curve = Curve::corners(
        &[
            Vec3::ZERO,
            Vec3::new(30.0, 30.0, 0.0),
            Vec3::new(60.0, 0.0, 0.0),
        ],
        false,
    )
    .unwrap();
    let middle = CurvePointId::from_raw(1);
    let round = curve
        .edited(&CurveEdit::SetKind {
            points: vec![middle],
            kind: if true {
                PointKind::Smooth
            } else {
                PointKind::Sharp
            },
        })
        .unwrap();
    let p = round.point(middle).unwrap();
    assert_eq!(p.kind, PointKind::Smooth);
    // Along the way from the first neighbour to the second (+X), a third
    // of the way to each.
    assert!(p.handle_out.normalize().distance(Vec3::X) < 1e-5);
    assert!((p.handle_out.length() - 30.0 * 2f32.sqrt() / 3.0).abs() < 1e-3);
    assert!(p.handle_in.distance(-p.handle_out) < 1e-4);
    // An end point gets handles too, along its one stretch.
    let end = round
        .edited(&CurveEdit::SetKind {
            points: vec![CurvePointId::from_raw(0)],
            kind: if true {
                PointKind::Smooth
            } else {
                PointKind::Sharp
            },
        })
        .unwrap();
    assert!(end.points()[0].has_handles());
    // And back: sharp again, no handles.
    let back = end
        .edited(&CurveEdit::SetKind {
            points: vec![middle],
            kind: if false {
                PointKind::Smooth
            } else {
                PointKind::Sharp
            },
        })
        .unwrap();
    assert!(!back.point(middle).unwrap().has_handles());
}

#[test]
fn closing_and_opening() {
    let curve = Curve::corners(&[Vec3::ZERO, Vec3::X, Vec3::Y], false).unwrap();
    let closed = curve.edited(&CurveEdit::SetClosed(true)).unwrap();
    assert!(closed.closed());
    assert!(
        !closed
            .edited(&CurveEdit::SetClosed(false))
            .unwrap()
            .closed()
    );
}

#[test]
fn adding_points_counts_ids_up_and_stops_at_the_limit() {
    let mut curve = Curve::default();
    for i in 0..5 {
        curve = curve
            .with_point(Vec3::X * i as f32, Vec3::ZERO, Vec3::ZERO, false)
            .unwrap();
    }
    assert_eq!(curve.next_id(), 5);
    assert_eq!(curve.points()[4].id, CurvePointId::from_raw(4));
    let full = Curve::corners(&vec![Vec3::ZERO; MAX_CURVE_POINTS], false).unwrap();
    assert_eq!(
        full.with_point(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, false),
        Err(CurveError::TooManyPoints)
    );
}

#[test]
fn bounds_hold_points_and_handles() {
    let curve = Curve::new(
        [smooth(Vec3::ZERO, Vec3::Z * 5.0), smooth(Vec3::X, Vec3::X)],
        false,
    )
    .unwrap();
    let b = curve.bounds().unwrap();
    assert_eq!(b.max.z, 5.0);
    assert_eq!(b.min.z, -5.0);
    assert!(Curve::default().bounds().is_none());
    assert!(Curve::default().is_empty());
    assert!(
        Curve::corners(&[Vec3::ONE, Vec3::ONE], false)
            .unwrap()
            .is_empty()
    );
}

// --- Ready-made curves --------------------------------------------------

#[test]
fn ready_made_curves_have_their_shape() {
    for kind in ReadyCurveKind::ALL {
        let ready = kind.standard();
        assert_eq!(ready.kind(), kind);
        assert!(!kind.name().is_empty() && !kind.description().is_empty());
        let curve = ready.curve();
        assert!(!curve.is_empty(), "{kind:?}");
        assert_eq!(
            ready.clamped(),
            ready,
            "standard settings are within limits"
        );
    }
    let circle = ReadyCurve::Circle { radius: 50.0 }.curve();
    assert!(circle.closed());
    // Every place along it is about 50 cm from the middle.
    for p in circle.polyline(16) {
        assert!((p.length() - 50.0).abs() < 0.05, "{p}");
    }
    let spiral = ReadyCurve::Spiral {
        radius: 20.0,
        height: 90.0,
        turns: 3.0,
    }
    .curve();
    let line = spiral.polyline(8);
    assert!(line[0].z.abs() < 1e-4);
    assert!((line[line.len() - 1].z - 90.0).abs() < 1e-3);
    let arc = ReadyCurve::Arc {
        radius: 10.0,
        angle: 180.0,
    }
    .curve();
    let end = arc.points().last().unwrap().position;
    assert!(end.distance(Vec3::new(-10.0, 0.0, 0.0)) < 1e-4);
    let length = ReadyCurve::Line { length: 40.0 }.curve();
    assert_eq!(length.points()[1].position, Vec3::new(20.0, 0.0, 0.0));
}

#[test]
fn ready_made_settings_are_clamped() {
    let silly = ReadyCurve::Spiral {
        radius: -5.0,
        height: f32::NAN,
        turns: 1e9,
    }
    .clamped();
    assert_eq!(
        silly,
        ReadyCurve::Spiral {
            radius: 0.01,
            height: 0.0,
            turns: ready::MAX_TURNS,
        }
    );
    assert_eq!(
        ReadyCurve::Arc {
            radius: 1.0,
            angle: 0.0
        }
        .clamped(),
        ReadyCurve::Arc {
            radius: 1.0,
            angle: 1.0
        }
    );
    assert!(!ReadyCurve::Line { length: 0.0 }.curve().is_empty());
}

// --- Sweeps ---------------------------------------------------------------

#[test]
fn a_straight_pipe_is_a_capped_cylinder() {
    let path = line(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0));
    let mesh = sweep::sweep(&path, &Profile::circle(20.0, 8), &pipe());
    assert_solid(&mesh, "straight pipe");
    // One ring of quads (a straight span is one piece) and two caps.
    assert_eq!(mesh.face_count(), 8 + 2);
    let b = mesh.bounds().unwrap();
    assert!((b.size().x - 100.0).abs() < 1e-3);
    assert!((b.size().z - 20.0).abs() < 1e-3);
    // Roughly a cylinder's volume (an octagon is a little smaller).
    let expected = std::f32::consts::PI * 100.0 * 100.0;
    assert!((volume(&mesh) / expected - 0.9).abs() < 0.05);
    // The cap rims are hard; the sides of a round pipe soft.
    assert_eq!(mesh.hard_edge_count(), 16);
}

#[test]
fn even_detail_cuts_every_span_alike() {
    let path = line(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0));
    let shape = SweepShape {
        detail: Detail {
            segments: 5,
            adaptive: false,
        },
        ..pipe()
    };
    let mesh = sweep::sweep(&path, &Profile::circle(20.0, 6), &shape);
    assert_eq!(mesh.face_count(), 5 * 6 + 2);
    assert_solid(&mesh, "even");
}

#[test]
fn a_square_profile_has_hard_corners_along_the_length() {
    let path = line(Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0));
    let mesh = sweep::sweep(&path, &Profile::rectangle(10.0, 4.0), &pipe());
    assert_solid(&mesh, "square bar");
    // Four along the length and four round each cap.
    assert_eq!(mesh.hard_edge_count(), 12);
    let b = mesh.bounds().unwrap();
    // Width across (X for a path along Y), height up.
    assert!((b.size().x - 10.0).abs() < 1e-3, "{:?}", b);
    assert!((b.size().z - 4.0).abs() < 1e-3);
}

#[test]
fn a_bent_pipe_follows_its_curve_without_twisting() {
    // A quarter circle on the ground.
    let path = ReadyCurve::Arc {
        radius: 100.0,
        angle: 90.0,
    }
    .curve();
    let mesh = sweep::sweep(&path, &Profile::rectangle(10.0, 10.0), &pipe());
    assert_solid(&mesh, "bent bar");
    // A flat path keeps the profile upright: everything between z = -5 and 5.
    let b = mesh.bounds().unwrap();
    assert!((b.min.z + 5.0).abs() < 1e-3 && (b.max.z - 5.0).abs() < 1e-3);
    // Adaptive detail: the bend got its pieces.
    assert!(mesh.face_count() > 6 * 4);
}

#[test]
fn a_closed_curve_makes_a_ring_with_no_caps_and_no_seam_gap() {
    let path = ReadyCurve::Circle { radius: 100.0 }.curve();
    let mesh = sweep::sweep(&path, &Profile::circle(20.0, 8), &pipe());
    assert_solid(&mesh, "ring");
    // A torus: no caps, every face a quad.
    assert!(mesh.faces().all(|(_, c)| c.len() == 4));
    let torus = 2.0 * std::f32::consts::PI * std::f32::consts::PI * 100.0 * 100.0;
    assert!((volume(&mesh) / torus - 0.9).abs() < 0.06);
}

#[test]
fn a_spiral_sweep_does_not_flip() {
    let path = ReadyCurve::Spiral {
        radius: 50.0,
        height: 150.0,
        turns: 4.0,
    }
    .curve();
    let mesh = sweep::sweep(&path, &Profile::rectangle(8.0, 2.0), &pipe());
    assert_solid(&mesh, "spiral");
    // Neighbouring rings stay alike: no face is stretched long and thin by
    // a sudden spin.
    for (face, corners) in mesh.faces() {
        if corners.len() != 4 {
            continue;
        }
        let p: Vec<Vec3> = corners.iter().map(|v| mesh.position(*v).unwrap()).collect();
        let longest = (0..4)
            .map(|i| p[i].distance(p[(i + 1) % 4]))
            .fold(0.0, f32::max);
        assert!(longest < 40.0, "face {face:?} is stretched: {longest}");
    }
}

#[test]
fn sharp_corners_are_mitred_and_hard() {
    let path = Curve::corners(
        &[
            Vec3::ZERO,
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(100.0, 100.0, 0.0),
        ],
        false,
    )
    .unwrap();
    let mesh = sweep::sweep(&path, &Profile::rectangle(10.0, 10.0), &pipe());
    assert_solid(&mesh, "L-shaped bar");
    // The corner ring is stretched by 1/cos 45° across the corner.
    let corner = mesh
        .vertices()
        .map(|(_, p)| p)
        .filter(|p| p.x > 90.0 && p.y < 10.0 && p.y > -10.0)
        .map(|p| p.distance(Vec3::new(100.0, 0.0, p.z)))
        .fold(0.0, f32::max);
    assert!((corner - 5.0 * 2f32.sqrt()).abs() < 0.01, "{corner}");
    // The bar is 10 wide on both legs: 100 + 5 along X, 100 + 5 along Y.
    let b = mesh.bounds().unwrap();
    assert!((b.max.x - 105.0).abs() < 1e-3 && (b.max.y - 100.0).abs() < 1e-3);
    // Four along each leg, four round each cap, and the corner ring.
    assert!(mesh.hard_edge_count() >= 4 + 4 + 4);
}

#[test]
fn a_very_tight_bend_stays_a_closed_solid() {
    // Almost straight back on itself.
    let path = Curve::corners(
        &[
            Vec3::ZERO,
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(0.0, 5.0, 0.0),
        ],
        false,
    )
    .unwrap();
    let mesh = sweep::sweep(&path, &Profile::circle(4.0, 6), &pipe());
    assert!(is_closed(&mesh));
    assert!(faces_facing_wrong(&mesh).is_empty());
    // The mitre is held to four times the width.
    let b = mesh.bounds().unwrap();
    assert!(b.size().max_element() < 100.0 + 4.0 * 4.0 + 1.0);
}

#[test]
fn coincident_points_and_zero_handles_are_skipped() {
    let path = Curve::new(
        [
            (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, false),
            (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, true),
            (Vec3::new(50.0, 0.0, 0.0), Vec3::ZERO, Vec3::ZERO, false),
            (Vec3::new(50.0, 0.0, 0.0), Vec3::ZERO, Vec3::ZERO, false),
        ],
        false,
    )
    .unwrap();
    let mesh = sweep::sweep(&path, &Profile::circle(10.0, 6), &pipe());
    assert_solid(&mesh, "doubled points");
    assert_eq!(mesh.face_count(), 6 + 2);
    // All on one spot: nothing at all.
    let dot = Curve::corners(&[Vec3::ONE; 3], true).unwrap();
    assert_eq!(
        sweep::sweep(&dot, &Profile::circle(10.0, 6), &pipe()).face_count(),
        0
    );
    assert_eq!(
        sweep::sweep(&Curve::default(), &Profile::circle(10.0, 6), &pipe()).face_count(),
        0
    );
}

#[test]
fn caps_off_leaves_the_ends_open() {
    let path = line(Vec3::ZERO, Vec3::X * 50.0);
    let shape = SweepShape {
        caps: false,
        ..pipe()
    };
    let mesh = sweep::sweep(&path, &Profile::circle(10.0, 6), &shape);
    assert_eq!(mesh.face_count(), 6);
    assert!(!is_closed(&mesh));
}

#[test]
fn twist_turns_the_profile_and_rounds_to_whole_turns_on_a_loop() {
    let path = line(Vec3::ZERO, Vec3::X * 100.0);
    let shape = SweepShape {
        twist: 90.0,
        ..pipe()
    };
    let mesh = sweep::sweep(&path, &Profile::rectangle(20.0, 2.0), &shape);
    assert_solid(&mesh, "twisted bar");
    // The far end is turned a quarter: 20 tall there.
    let end: Vec<Vec3> = mesh
        .vertices()
        .map(|(_, p)| p)
        .filter(|p| p.x > 99.9)
        .collect();
    let tall = end.iter().map(|p| p.z).fold(f32::MIN, f32::max);
    assert!((tall - 10.0).abs() < 1e-3, "{tall}");
    // The twist asked for more pieces along the straight line.
    assert!(mesh.face_count() > 4 + 2);
    let ring = ReadyCurve::Circle { radius: 100.0 }.curve();
    let shape = SweepShape {
        twist: 100.0,
        ..pipe()
    };
    assert_solid(
        &sweep::sweep(&ring, &Profile::rectangle(20.0, 2.0), &shape),
        "twisted ring",
    );
    let wild = SweepShape {
        twist: f32::NAN,
        ..pipe()
    };
    assert_solid(
        &sweep::sweep(&path, &Profile::circle(5.0, 5), &wild),
        "NaN twist",
    );
}

#[test]
fn profile_sizes_are_clamped() {
    let tiny = Profile::circle(-3.0, 1);
    assert_eq!(tiny.points().len(), 3);
    assert!((tiny.points()[0].length() - sweep::MIN_SIZE / 2.0).abs() < 1e-6);
    let huge = Profile::rectangle(f32::INFINITY, 1e12);
    assert!(huge.points().iter().all(|p| p.is_finite()));
    let path = line(Vec3::ZERO, Vec3::X * 50.0);
    assert_solid(&sweep::sweep(&path, &tiny, &pipe()), "tiny");
    assert_eq!(Profile::circle(10.0, 1000).points().len(), 64);
}

#[test]
fn a_profile_from_a_curve_lies_flat_on_its_plane() {
    // Drawn in the front view: an L standing in the X–Z plane.
    let l = Curve::corners(
        &[
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 8.0),
            Vec3::new(0.0, 0.0, 8.0),
        ],
        true,
    )
    .unwrap();
    let profile = Profile::from_curve(&l, Detail::default()).unwrap();
    assert!(profile.closed());
    assert_eq!(profile.points().len(), 6);
    // x is the world's X, y its Z.
    assert!(profile.points().contains(&glam::Vec2::new(10.0, 2.0)));
    let path = line(Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0));
    let mesh = sweep::sweep(&path, &profile, &pipe());
    assert_solid(&mesh, "L trim");
    // Every corner of the L is hard along the length.
    assert!(mesh.hard_edge_count() >= 6);
    // Going the other way round makes the same solid, facing out.
    let backwards = Curve::corners(
        &l.points()
            .iter()
            .rev()
            .map(|p| p.position)
            .collect::<Vec<_>>(),
        true,
    )
    .unwrap();
    let flipped = Profile::from_curve(&backwards, Detail::default()).unwrap();
    assert_solid(&sweep::sweep(&path, &flipped, &pipe()), "L backwards");
}

#[test]
fn an_open_profile_makes_a_strip() {
    let profile_curve = Curve::new(
        [
            (
                Vec3::new(-10.0, 0.0, 0.0),
                Vec3::ZERO,
                Vec3::new(5.0, 0.0, 5.0),
                false,
            ),
            (
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(-5.0, 0.0, 5.0),
                Vec3::ZERO,
                false,
            ),
        ],
        false,
    )
    .unwrap();
    let profile = Profile::from_curve(&profile_curve, Detail::default()).unwrap();
    assert!(!profile.closed());
    let path = line(Vec3::ZERO, Vec3::new(0.0, 50.0, 0.0));
    let mesh = sweep::sweep(&path, &profile, &pipe());
    // No caps on an open profile; an open strip, one piece per profile
    // stretch.
    assert!(mesh.face_count() > 1);
    assert!(!is_closed(&mesh));
    assert!(faces_facing_wrong(&mesh).is_empty());
    assert_uvs(&mesh, "strip");
    // A straight line profile works too.
    let flat = Profile::from_curve(&line(Vec3::ZERO, Vec3::X * 10.0), Detail::default()).unwrap();
    assert_eq!(sweep::sweep(&path, &flat, &pipe()).face_count(), 1);
    // A profile curve of one spot is no profile.
    assert!(
        Profile::from_curve(
            &Curve::corners(&[Vec3::ONE; 2], false).unwrap(),
            Detail::default()
        )
        .is_none()
    );
}

#[test]
fn sweeps_have_uvs_at_real_size_with_seams() {
    let path = line(Vec3::ZERO, Vec3::X * 100.0);
    let mesh = sweep::sweep(&path, &Profile::circle(20.0, 8), &pipe());
    // The strip is cut open along one line: seams there and round the caps.
    let seams = crate::uvs::seams(&mesh);
    assert!(seams.len() > 16, "{}", seams.len());
}

#[test]
fn a_heavy_sweep_is_cut_back_to_the_limit() {
    let path = ReadyCurve::Spiral {
        radius: 50.0,
        height: 100.0,
        turns: ready::MAX_TURNS,
    }
    .curve();
    let shape = SweepShape {
        detail: Detail {
            segments: Detail::MAX_SEGMENTS,
            adaptive: false,
        },
        ..pipe()
    };
    let mesh = sweep::sweep(&path, &Profile::circle(5.0, 64), &shape);
    assert!(mesh.face_count() <= sweep::MAX_POLYGONS + 2);
    assert!(mesh.face_count() > 1000);
}

// --- Lathes ---------------------------------------------------------------

fn vase() -> Curve {
    // From the axis on the ground, out, up and in a little: an open
    // profile in the X–Z plane.
    Curve::new(
        [
            (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, false),
            (Vec3::new(30.0, 0.0, 0.0), Vec3::ZERO, Vec3::ZERO, false),
            smooth(Vec3::new(40.0, 0.0, 40.0), Vec3::new(0.0, 0.0, 15.0)),
            (Vec3::new(20.0, 0.0, 80.0), Vec3::ZERO, Vec3::ZERO, false),
        ],
        false,
    )
    .unwrap()
}

#[test]
fn a_lathe_spins_a_profile_into_a_solid() {
    let mesh = lathe::lathe(&vase(), &LatheShape::default());
    // The bottom closes at the axis (a pole); the top is capped.
    assert_solid(&mesh, "vase");
    let b = mesh.bounds().unwrap();
    assert!((b.max.z - 80.0).abs() < 1e-3);
    // Round: as wide one way as the other.
    assert!((b.size().x - b.size().y).abs() < 2.0);
    // The pole is one point.
    let on_axis = mesh
        .vertices()
        .filter(|(_, p)| p.truncate().length() < 1e-3 && p.z.abs() < 1e-3)
        .count();
    assert_eq!(on_axis, 1);
}

#[test]
fn a_lathe_without_caps_is_open_where_the_profile_stops_short() {
    let shape = LatheShape {
        caps: false,
        ..LatheShape::default()
    };
    let mesh = lathe::lathe(&vase(), &shape);
    assert!(!is_closed(&mesh));
    assert!(faces_facing_wrong(&mesh).is_empty());
    assert!(volume(&mesh) > 0.0);
}

#[test]
fn a_part_turn_is_closed_with_flat_sides() {
    for angle in [90.0, 180.0, 270.0] {
        let shape = LatheShape {
            angle,
            ..LatheShape::default()
        };
        assert_solid(&lathe::lathe(&vase(), &shape), &format!("{angle}° vase"));
    }
    // A profile that never reaches the axis: pie slices top and bottom.
    let wall = Curve::corners(
        &[Vec3::new(20.0, 0.0, 0.0), Vec3::new(30.0, 0.0, 50.0)],
        false,
    )
    .unwrap();
    let shape = LatheShape {
        angle: 120.0,
        ..LatheShape::default()
    };
    assert_solid(&lathe::lathe(&wall, &shape), "slice of a wall");
    // Both ends at the same height: closed straight across between them.
    let arch = Curve::corners(
        &[
            Vec3::new(20.0, 0.0, 0.0),
            Vec3::new(30.0, 0.0, 50.0),
            Vec3::new(40.0, 0.0, 0.0),
        ],
        false,
    )
    .unwrap();
    assert_solid(&lathe::lathe(&arch, &shape), "arch slice");
}

#[test]
fn a_closed_profile_makes_a_ring() {
    let square = Curve::corners(
        &[
            Vec3::new(40.0, 0.0, 0.0),
            Vec3::new(50.0, 0.0, 0.0),
            Vec3::new(50.0, 0.0, 10.0),
            Vec3::new(40.0, 0.0, 10.0),
        ],
        true,
    )
    .unwrap();
    let mesh = lathe::lathe(&square, &LatheShape::default());
    assert_solid(&mesh, "washer");
    // Its sharp corners make hard rings.
    assert!(mesh.hard_edge_count() >= 4 * 24);
    let backwards = Curve::corners(
        &square
            .points()
            .iter()
            .rev()
            .map(|p| p.position)
            .collect::<Vec<_>>(),
        true,
    )
    .unwrap();
    assert_solid(
        &lathe::lathe(&backwards, &LatheShape::default()),
        "washer backwards",
    );
    let half = LatheShape {
        angle: 180.0,
        ..LatheShape::default()
    };
    assert_solid(&lathe::lathe(&square, &half), "half washer");
}

#[test]
fn lathes_round_every_axis() {
    for axis in LatheAxis::ALL {
        // A profile from the axis, out and back to it: a ball-ish shape.
        let a = axis.vector();
        let side = a.any_orthonormal_vector();
        let curve = Curve::corners(
            &[
                a * -20.0,
                a * -10.0 + side * 15.0,
                a * 10.0 + side * 15.0,
                a * 20.0,
            ],
            false,
        )
        .unwrap();
        let mesh = lathe::lathe(
            &curve,
            &LatheShape {
                axis,
                ..LatheShape::default()
            },
        );
        assert_solid(&mesh, &format!("{axis:?}"));
    }
}

#[test]
fn lathe_degenerate_cases() {
    // All on the axis: nothing to spin.
    let rod = Curve::corners(&[Vec3::ZERO, Vec3::Z * 10.0], false).unwrap();
    assert_eq!(lathe::lathe(&rod, &LatheShape::default()).face_count(), 0);
    // A line along the ground from the axis: a flat disc, facing up.
    let spoke = Curve::corners(&[Vec3::ZERO, Vec3::X * 10.0], false).unwrap();
    let disc = lathe::lathe(&spoke, &LatheShape::default());
    assert!(disc.face_count() > 0);
    assert!(disc.faces().all(|(f, _)| disc.face_normal(f).z > 0.99));
    // Silly settings are clamped.
    let wild = LatheShape {
        angle: f32::NAN,
        segments: 0,
        detail: Detail {
            segments: 0,
            adaptive: false,
        },
        ..LatheShape::default()
    }
    .clamped();
    assert_eq!(wild.angle, 360.0);
    assert_eq!(wild.segments, 3);
    assert_eq!(wild.detail.segments, 1);
    assert_solid(&lathe::lathe(&vase(), &wild), "three-sided vase");
    assert_eq!(
        lathe::lathe(&Curve::default(), &LatheShape::default()).face_count(),
        0
    );
}

// --- Never panics --------------------------------------------------------

/// A small random number maker, the same every run.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn number(&mut self, scale: f32) -> f32 {
        let pick = self.next() % 20;
        match pick {
            0 => 0.0,
            1 => scale * 1e-6,
            _ => (self.next() % 2001) as f32 / 1000.0 * scale - scale,
        }
    }
    fn point(&mut self, scale: f32) -> Vec3 {
        Vec3::new(self.number(scale), self.number(scale), self.number(scale))
    }
}

#[test]
fn random_curves_never_panic() {
    let mut dice = Dice(0x5eed_cafe);
    for round in 0..300 {
        let count = (dice.next() % 7) as usize;
        let scale = [1e-3, 1.0, 100.0, 50_000.0][(dice.next() % 4) as usize];
        let points: Vec<(Vec3, Vec3, Vec3, bool)> = (0..count)
            .map(|_| {
                let p = if dice.next().is_multiple_of(4) {
                    Vec3::ZERO
                } else {
                    dice.point(scale)
                };
                let h = if dice.next().is_multiple_of(3) {
                    Vec3::ZERO
                } else {
                    dice.point(scale / 2.0)
                };
                (p, -h, h, dice.next().is_multiple_of(2))
            })
            .collect();
        let Ok(curve) = Curve::new(points, dice.next().is_multiple_of(3)) else {
            continue;
        };
        let detail = Detail {
            segments: (dice.next() % 10) as u32,
            adaptive: dice.next().is_multiple_of(2),
        };
        let profile = match dice.next() % 3 {
            0 => Profile::circle(dice.number(scale), (dice.next() % 10) as u32),
            1 => Profile::rectangle(dice.number(scale), dice.number(scale)),
            _ => Profile::from_curve(&curve, detail).unwrap_or_else(|| Profile::circle(1.0, 4)),
        };
        let shape = SweepShape {
            detail,
            twist: dice.number(720.0),
            caps: dice.next().is_multiple_of(2),
        };
        let swept = sweep::sweep(&curve, &profile, &shape);
        assert!(
            swept.vertices().all(|(_, p)| p.is_finite()),
            "round {round}"
        );
        let spun = lathe::lathe(
            &curve,
            &LatheShape {
                axis: LatheAxis::ALL[(dice.next() % 3) as usize],
                angle: dice.number(400.0),
                segments: (dice.next() % 40) as u32,
                detail,
                caps: dice.next().is_multiple_of(2),
            },
        );
        assert!(spun.vertices().all(|(_, p)| p.is_finite()), "round {round}");
        // Random edits never panic either.
        let parts: Vec<CurvePart> = curve.parts().into_iter().map(|(p, _)| p).collect();
        for edit in [
            CurveEdit::Transform {
                parts: parts.clone(),
                matrix: Mat4::from_scale(Vec3::splat(dice.number(3.0))),
            },
            CurveEdit::Delete {
                parts: parts
                    .iter()
                    .copied()
                    .take((dice.next() % 3) as usize)
                    .collect(),
            },
            CurveEdit::SetKind {
                points: curve.points().iter().map(|p| p.id).collect(),
                kind: if dice.next().is_multiple_of(2) {
                    PointKind::Smooth
                } else {
                    PointKind::Sharp
                },
            },
            CurveEdit::SetClosed(dice.next().is_multiple_of(2)),
        ] {
            let _ = curve.edited(&edit);
        }
    }
}

#[test]
#[ignore = "timing: run with --ignored --nocapture in release"]
fn timing_a_long_sweep() {
    // A cable of 200 smooth points, round and fine.
    let points: Vec<(Vec3, Vec3, Vec3, bool)> = (0..200)
        .map(|i| {
            let a = i as f32 * 0.3;
            smooth(
                Vec3::new(i as f32 * 20.0, a.sin() * 100.0, a.cos() * 50.0),
                Vec3::new(8.0, a.cos() * 10.0, -a.sin() * 5.0),
            )
        })
        .collect();
    let curve = Curve::new(points, false).unwrap();
    let shape = SweepShape {
        detail: Detail {
            segments: 16,
            adaptive: true,
        },
        ..pipe()
    };
    let start = std::time::Instant::now();
    let mesh = sweep::sweep(&curve, &Profile::circle(10.0, 24), &shape);
    let took = start.elapsed();
    println!(
        "long sweep: {} polygons in {:.1} ms",
        mesh.face_count(),
        took.as_secs_f64() * 1000.0
    );
    assert!(took.as_secs_f64() < 2.0);
}

#[test]
fn a_closed_curve_of_fewer_than_three_points_is_refused_from_its_parts() {
    let point = |id: u32, x: f32| CurvePoint {
        id: CurvePointId(id),
        position: Vec3::new(x, 0.0, 0.0),
        handle_in: Vec3::ZERO,
        handle_out: Vec3::ZERO,
        kind: PointKind::Sharp,
    };
    for count in 0..3 {
        let points: Vec<CurvePoint> = (0..count).map(|i| point(i, i as f32 * 10.0)).collect();
        assert_eq!(
            Curve::from_parts(points.clone(), true, 10).map(|_| ()),
            Err(CurveError::LoopTooSmall),
            "{count} points"
        );
    }
    let three: Vec<CurvePoint> = (0..3).map(|i| point(i, i as f32 * 10.0)).collect();
    assert!(Curve::from_parts(three, true, 10).unwrap().closed());
    // Open, two points are a line.
    let two: Vec<CurvePoint> = (0..2).map(|i| point(i, i as f32 * 10.0)).collect();
    assert!(Curve::from_parts(two, false, 10).is_ok());
}
