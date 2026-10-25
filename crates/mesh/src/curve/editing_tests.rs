//! Tests for spline editing (decision 0139): adding points without
//! changing the shape, deleting with the shape kept, handle kinds, turning
//! round, closing, joining, splitting, rounding corners and even spacing,
//! with the awkward cases and a random run that must never panic.

use super::*;

fn id(n: u32) -> CurvePointId {
    CurvePointId::from_raw(n)
}

/// A gentle S of three smooth points.
fn s_curve() -> Curve {
    Curve::new(
        [
            (Vec3::ZERO, Vec3::ZERO, Vec3::new(10.0, 20.0, 0.0), true),
            (
                Vec3::new(50.0, 0.0, 0.0),
                Vec3::new(-15.0, -10.0, 0.0),
                Vec3::new(15.0, 10.0, 0.0),
                true,
            ),
            (
                Vec3::new(100.0, 0.0, 0.0),
                Vec3::new(-10.0, -20.0, 0.0),
                Vec3::ZERO,
                true,
            ),
        ],
        false,
    )
    .unwrap()
}

fn square(closed: bool) -> Curve {
    Curve::corners(
        &[
            Vec3::ZERO,
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(100.0, 100.0, 0.0),
            Vec3::new(0.0, 100.0, 0.0),
        ],
        closed,
    )
    .unwrap()
}

/// The furthest any finely sampled place of `a` is from `b`'s line.
fn distance_between(a: &Curve, b: &Curve) -> f32 {
    let line = b.polyline(400);
    a.polyline(400)
        .iter()
        .map(|p| {
            line.windows(2)
                .map(|w| {
                    let along = w[1] - w[0];
                    let t = if along.length_squared() > 1e-12 {
                        ((*p - w[0]).dot(along) / along.length_squared()).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    p.distance(w[0] + along * t)
                })
                .fold(f32::MAX, f32::min)
        })
        .fold(0.0, f32::max)
}

fn same_shape(a: &Curve, b: &Curve, within: f32) {
    let d = distance_between(a, b).max(distance_between(b, a));
    assert!(d < within, "shapes differ by {d}");
}

fn ids_unique(curve: &Curve) {
    let mut ids: Vec<u32> = curve.points().iter().map(|p| p.id.get()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), curve.points().len());
    assert!(ids.iter().all(|i| *i < curve.next_id()));
}

// ------------------------------------------------------------ adding points

#[test]
fn adding_a_point_keeps_the_shape_exactly() {
    let curve = s_curve();
    for t in [0.1, 0.37, 0.5, 0.9] {
        let more = curve
            .edited(&CurveEdit::Insert { after: id(0), t })
            .unwrap();
        assert_eq!(more.points().len(), 4);
        let new = more.points()[1];
        assert_eq!(new.id, id(3), "the next id");
        assert_eq!(new.kind, PointKind::Smooth);
        // Each half follows its part of the old stretch, bit-close.
        let old = curve.spans()[0];
        let (first, second) = (more.spans()[0], more.spans()[1]);
        for k in 0..=20 {
            let u = k as f32 / 20.0;
            assert!(old.at(u * t).distance(first.at(u)) < 1e-4, "t {t} u {u}");
            assert!(
                old.at(t + u * (1.0 - t)).distance(second.at(u)) < 1e-4,
                "t {t} u {u}"
            );
        }
        // The new point's handles are in one line.
        assert!(new.handle_in.normalize().dot(new.handle_out.normalize()) < -0.9999);
        ids_unique(&more);
    }
}

#[test]
fn adding_a_point_on_a_straight_stretch_makes_a_corner_on_the_line() {
    let curve = square(false);
    let more = curve
        .edited(&CurveEdit::Insert {
            after: id(1),
            t: 0.25,
        })
        .unwrap();
    let new = more.points()[2];
    assert_eq!(new.kind, PointKind::Sharp);
    assert!(!new.has_handles());
    // Where the stretch is at t (by its own measure, not its length).
    assert!(new.position.distance(curve.spans()[1].at(0.25)) < 1e-4);
    assert!((new.position.x - 100.0).abs() < 1e-4);
    assert!(more.points().iter().all(|p| !p.has_handles()));
}

#[test]
fn adding_a_point_on_a_closed_curves_last_stretch_and_odd_cases() {
    let closed = square(true);
    let more = closed
        .edited(&CurveEdit::Insert {
            after: id(3),
            t: 0.5,
        })
        .unwrap();
    assert_eq!(more.points().len(), 5);
    assert!(
        more.points()[4]
            .position
            .distance(Vec3::new(0.0, 50.0, 0.0))
            < 1e-4
    );
    // The last point of an open curve has no stretch leaving it.
    let open = square(false);
    assert_eq!(
        open.edited(&CurveEdit::Insert {
            after: id(3),
            t: 0.5
        })
        .unwrap(),
        open
    );
    // Unknown ids change nothing; a t that isn't a number is refused.
    assert_eq!(
        open.edited(&CurveEdit::Insert {
            after: id(99),
            t: 0.5
        })
        .unwrap(),
        open
    );
    assert_eq!(
        open.edited(&CurveEdit::Insert {
            after: id(0),
            t: f32::NAN
        }),
        Err(CurveError::NotANumber)
    );
    // At the very ends, on a stretch of no length: a point on the same
    // spot, no harm done.
    for t in [0.0, 1.0, -3.0, 7.0] {
        let more = open.edited(&CurveEdit::Insert { after: id(0), t }).unwrap();
        assert_eq!(more.points().len(), 5);
        same_shape(&more, &open, 1e-3);
    }
    let dot = Curve::corners(&[Vec3::ONE, Vec3::ONE], false).unwrap();
    let more = dot
        .edited(&CurveEdit::Insert {
            after: id(0),
            t: 0.5,
        })
        .unwrap();
    assert!(more.points().iter().all(|p| p.position == Vec3::ONE));
}

#[test]
fn adding_a_point_beside_an_auto_point_keeps_the_shape() {
    let curve = square(false)
        .edited(&CurveEdit::SetKind {
            points: vec![id(1), id(2)],
            kind: PointKind::Auto,
        })
        .unwrap();
    let more = curve
        .edited(&CurveEdit::Insert {
            after: id(1),
            t: 0.4,
        })
        .unwrap();
    same_shape(&more, &curve, 1e-3);
    // The neighbours keep their handles: they became smooth.
    assert_eq!(more.point(id(1)).unwrap().kind, PointKind::Smooth);
    assert_eq!(more.point(id(2)).unwrap().kind, PointKind::Smooth);
}

#[test]
fn ids_are_never_reused_after_deleting_and_adding() {
    let curve = s_curve();
    let fewer = curve
        .edited(&CurveEdit::Delete {
            parts: vec![CurvePart::point(id(2))],
        })
        .unwrap();
    let more = fewer
        .edited(&CurveEdit::Insert {
            after: id(0),
            t: 0.5,
        })
        .unwrap();
    assert_eq!(more.points()[1].id, id(3));
    ids_unique(&more);
}

// ----------------------------------------------------------------- deleting

#[test]
fn deleting_an_added_point_gives_the_shape_back() {
    let curve = s_curve();
    for t in [0.2, 0.5, 0.8] {
        let more = curve
            .edited(&CurveEdit::Insert { after: id(1), t })
            .unwrap();
        let back = more
            .edited(&CurveEdit::Delete {
                parts: vec![CurvePart::point(id(3))],
            })
            .unwrap();
        for (a, b) in back.points().iter().zip(curve.points()) {
            assert!(a.handle_in.distance(b.handle_in) < 1e-3, "t {t}");
            assert!(a.handle_out.distance(b.handle_out) < 1e-3, "t {t}");
        }
        same_shape(&back, &curve, 1e-2);
    }
}

#[test]
fn deleting_keeps_the_neighbours_handles_pointing_the_same_way() {
    let curve = Curve::new(
        (0..6).map(|i| {
            let a = i as f32;
            (
                Vec3::new(a * 20.0, (a * 1.3).sin() * 30.0, 0.0),
                Vec3::new(-6.0, -2.0, 0.0),
                Vec3::new(6.0, 2.0, 0.0),
                true,
            )
        }),
        false,
    )
    .unwrap();
    let fewer = curve
        .edited(&CurveEdit::Delete {
            parts: vec![CurvePart::point(id(2)), CurvePart::point(id(3))],
        })
        .unwrap();
    assert_eq!(fewer.points().len(), 4);
    let (before, after) = (fewer.point(id(1)).unwrap(), fewer.point(id(4)).unwrap());
    let old = |n| curve.point(id(n)).unwrap();
    assert!(
        before
            .handle_out
            .normalize()
            .dot(old(1).handle_out.normalize())
            > 0.9999
    );
    assert!(
        after
            .handle_in
            .normalize()
            .dot(old(4).handle_in.normalize())
            > 0.9999
    );
    // Their other handles stay exactly.
    assert_eq!(before.handle_in, old(1).handle_in);
    assert_eq!(after.handle_out, old(4).handle_out);
    // The ends of an open curve just go.
    let shorter = curve
        .edited(&CurveEdit::Delete {
            parts: vec![CurvePart::point(id(0))],
        })
        .unwrap();
    assert_eq!(shorter.point(id(1)), curve.point(id(1)));
    // A closed curve's gap across its first point.
    let closed = square(true)
        .edited(&CurveEdit::Delete {
            parts: vec![CurvePart::point(id(0))],
        })
        .unwrap();
    assert!(closed.closed());
    assert_eq!(closed.points().len(), 3);
}

// ------------------------------------------------------------ handle kinds

#[test]
fn handle_kinds_behave_as_named() {
    let curve = s_curve();
    let middle = id(1);
    let pull = |c: &Curve, kind| {
        c.edited(&CurveEdit::Transform {
            parts: vec![CurvePart {
                point: middle,
                kind,
            }],
            matrix: Mat4::from_translation(Vec3::new(0.0, 10.0, 0.0)),
        })
        .unwrap()
    };
    // Smooth: the other handle turns to match, keeping its length.
    let smooth = pull(&curve, PartKind::Out);
    let p = smooth.point(middle).unwrap();
    assert!(p.handle_in.normalize().dot(p.handle_out.normalize()) < -0.9999);
    assert!((p.handle_in.length() - curve.point(middle).unwrap().handle_in.length()).abs() < 1e-4);
    // Broken: the other handle stays.
    let broken = curve
        .edited(&CurveEdit::SetKind {
            points: vec![middle],
            kind: PointKind::Broken,
        })
        .unwrap();
    let moved = pull(&broken, PartKind::Out);
    assert_eq!(
        moved.point(middle).unwrap().handle_in,
        curve.point(middle).unwrap().handle_in
    );
    // Auto: handles follow the neighbours, a third of the way to each.
    let auto = square(false)
        .edited(&CurveEdit::SetKind {
            points: vec![id(1)],
            kind: PointKind::Auto,
        })
        .unwrap();
    let p = auto.point(id(1)).unwrap();
    let way = (Vec3::new(100.0, 100.0, 0.0) - Vec3::ZERO).normalize();
    assert!(p.handle_out.distance(way * 100.0 / 3.0) < 1e-3);
    assert!(p.handle_in.distance(-way * 100.0 / 3.0) < 1e-3);
    // ... and follow when a neighbour moves.
    let moved = auto
        .edited(&CurveEdit::Transform {
            parts: vec![CurvePart::point(id(2))],
            matrix: Mat4::from_translation(Vec3::new(0.0, 50.0, 0.0)),
        })
        .unwrap();
    assert_ne!(moved.point(id(1)).unwrap().handle_out, p.handle_out);
    // Pulling an auto point's handle by hand makes it smooth.
    let pulled = auto
        .edited(&CurveEdit::Transform {
            parts: vec![CurvePart {
                point: id(1),
                kind: PartKind::In,
            }],
            matrix: Mat4::from_translation(Vec3::Z * 5.0),
        })
        .unwrap();
    assert_eq!(pulled.point(id(1)).unwrap().kind, PointKind::Smooth);
    // Sharp: no handles. Broken from sharp: handles to pull.
    let sharp = curve
        .edited(&CurveEdit::SetKind {
            points: vec![middle],
            kind: PointKind::Sharp,
        })
        .unwrap();
    assert!(!sharp.point(middle).unwrap().has_handles());
    let broken = sharp
        .edited(&CurveEdit::SetKind {
            points: vec![middle],
            kind: PointKind::Broken,
        })
        .unwrap();
    assert!(broken.point(middle).unwrap().has_handles());
    // An auto end point of an open curve points at its one neighbour.
    let end = square(false)
        .edited(&CurveEdit::SetKind {
            points: vec![id(0)],
            kind: PointKind::Auto,
        })
        .unwrap();
    let p = end.points()[0];
    assert_eq!(p.handle_in, Vec3::ZERO);
    assert!(p.handle_out.distance(Vec3::new(100.0 / 3.0, 0.0, 0.0)) < 1e-4);
}

#[test]
fn auto_points_on_one_spot_or_in_a_line_stay_numbers() {
    let curve = Curve::corners(&[Vec3::ONE; 4], true)
        .unwrap()
        .edited(&CurveEdit::SetKind {
            points: (0..4).map(id).collect(),
            kind: PointKind::Auto,
        })
        .unwrap();
    assert!(curve.points().iter().all(|p| !p.has_handles()));
    let back_and_forth = Curve::corners(&[Vec3::ZERO, Vec3::X * 10.0, Vec3::ZERO], false)
        .unwrap()
        .edited(&CurveEdit::SetKind {
            points: vec![id(1)],
            kind: PointKind::Auto,
        })
        .unwrap();
    assert!(back_and_forth.points()[1].handle_in.is_finite());
}

#[test]
fn older_smooth_switches_become_kinds() {
    assert_eq!(PointKind::from_smooth(true, false), PointKind::Smooth);
    assert_eq!(PointKind::from_smooth(false, true), PointKind::Broken);
    assert_eq!(PointKind::from_smooth(false, false), PointKind::Sharp);
    let names: std::collections::HashSet<_> = PointKind::ALL.iter().map(|k| k.name()).collect();
    assert_eq!(names.len(), 4);
}

// ----------------------------------------------------- reversing, closing

#[test]
fn reversing_twice_gives_the_same_curve() {
    for curve in [s_curve(), square(true), square(false)] {
        let once = curve.edited(&CurveEdit::Reverse).unwrap();
        assert_ne!(once, curve);
        same_shape(&once, &curve, 1e-3);
        assert_eq!(once.edited(&CurveEdit::Reverse).unwrap(), curve);
    }
    // An open curve's last point comes first; a closed one keeps its first.
    let once = s_curve().edited(&CurveEdit::Reverse).unwrap();
    assert_eq!(once.points()[0].id, id(2));
    assert_eq!(once.points()[0].handle_out, Vec3::new(-10.0, -20.0, 0.0));
    let round = square(true).edited(&CurveEdit::Reverse).unwrap();
    assert_eq!(round.points()[0].id, id(0));
    assert_eq!(round.points()[1].id, id(3));
}

#[test]
fn closing_needs_three_points() {
    let two = Curve::corners(&[Vec3::ZERO, Vec3::X], false).unwrap();
    assert_eq!(
        two.edited(&CurveEdit::SetClosed(true)),
        Err(CurveError::LoopTooSmall)
    );
    assert!(two.edited(&CurveEdit::SetClosed(false)).is_ok());
    // Welding the ends of three points would leave two.
    let three = Curve::corners(&[Vec3::ZERO, Vec3::X, Vec3::Y], false).unwrap();
    assert_eq!(
        three.edited(&CurveEdit::CloseEnds { weld: true }),
        Err(CurveError::LoopTooSmall)
    );
    let joined = three.edited(&CurveEdit::CloseEnds { weld: false }).unwrap();
    assert!(joined.closed() && joined.points().len() == 3);
    assert_eq!(
        joined.edited(&CurveEdit::CloseEnds { weld: false }),
        Err(CurveError::NotEnds)
    );
}

#[test]
fn welding_a_curves_ends_melts_them_at_their_middle() {
    let open = Curve::corners(
        &[
            Vec3::new(0.0, -10.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(100.0, 100.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
        ],
        false,
    )
    .unwrap();
    let closed = open.edited(&CurveEdit::CloseEnds { weld: true }).unwrap();
    assert!(closed.closed());
    assert_eq!(closed.points().len(), 3);
    assert_eq!(closed.points()[0].id, id(0));
    assert!(closed.points()[0].position.distance(Vec3::ZERO) < 1e-5);
    ids_unique(&closed);
}

// ------------------------------------------------------------------ joining

#[test]
fn joining_two_curves_end_to_start_with_a_stretch() {
    let a = Curve::corners(&[Vec3::ZERO, Vec3::X * 10.0], false).unwrap();
    let b = Curve::corners(&[Vec3::X * 20.0, Vec3::X * 30.0], false).unwrap();
    let joined = join(&a, CurveEnd::End, &b, CurveEnd::Start, false).unwrap();
    assert_eq!(joined.points().len(), 4);
    assert!(!joined.closed());
    ids_unique(&joined);
    // `a`'s points keep their ids.
    assert_eq!(joined.points()[0].id, id(0));
    assert_eq!(joined.points()[1].id, id(1));
    let xs: Vec<f32> = joined.points().iter().map(|p| p.position.x).collect();
    assert_eq!(xs, vec![0.0, 10.0, 20.0, 30.0]);
}

#[test]
fn joining_turns_curves_round_to_meet_at_the_chosen_ends() {
    let a = Curve::corners(&[Vec3::X * 10.0, Vec3::ZERO], false).unwrap();
    let b = Curve::corners(&[Vec3::X * 30.0, Vec3::X * 20.0], false).unwrap();
    // `a`'s start (x 10) to `b`'s end (x 20).
    let joined = join(&a, CurveEnd::Start, &b, CurveEnd::End, false).unwrap();
    let xs: Vec<f32> = joined.points().iter().map(|p| p.position.x).collect();
    assert_eq!(xs, vec![0.0, 10.0, 20.0, 30.0]);
}

#[test]
fn joining_ends_on_one_spot_or_welding_melts_them() {
    let a = s_curve();
    let b = Curve::corners(
        &[Vec3::new(100.0, 0.0, 0.0), Vec3::new(150.0, 0.0, 0.0)],
        false,
    )
    .unwrap();
    let joined = join(&a, CurveEnd::End, &b, CurveEnd::Start, false).unwrap();
    assert_eq!(joined.points().len(), 4, "the same spot melts");
    assert_eq!(joined.points()[2].handle_in, Vec3::new(-10.0, -20.0, 0.0));
    // Welding meets in the middle.
    let c = Curve::corners(
        &[Vec3::new(110.0, 0.0, 0.0), Vec3::new(150.0, 0.0, 0.0)],
        false,
    )
    .unwrap();
    let welded = join(&a, CurveEnd::End, &c, CurveEnd::Start, true).unwrap();
    assert_eq!(welded.points().len(), 4);
    assert!(
        welded.points()[2]
            .position
            .distance(Vec3::new(105.0, 0.0, 0.0))
            < 1e-4
    );
    // Ends meeting back at the start close the loop.
    let back = Curve::corners(
        &[
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(50.0, 80.0, 0.0),
            Vec3::ZERO,
        ],
        false,
    )
    .unwrap();
    let loop_ = join(&a, CurveEnd::End, &back, CurveEnd::Start, false).unwrap();
    assert!(loop_.closed());
    assert_eq!(loop_.points().len(), 4);
    ids_unique(&loop_);
}

#[test]
fn joining_refuses_closed_curves_and_a_curve_to_itself_is_close_ends() {
    let a = square(true);
    let b = square(false);
    assert_eq!(
        join(&a, CurveEnd::End, &b, CurveEnd::Start, false),
        Err(CurveError::NotEnds)
    );
    // A curve joined to a copy of itself end to end: the shared end
    // melts, and so do its two starts, now on one spot, closing a loop.
    let both = join(&b, CurveEnd::End, &b, CurveEnd::End, false).unwrap();
    assert_eq!(both.points().len(), 6);
    assert!(both.closed());
    ids_unique(&both);
    // Its own two ends are joined with CloseEnds.
    assert!(
        b.edited(&CurveEdit::CloseEnds { weld: false })
            .unwrap()
            .closed()
    );
}

// ---------------------------------------------------------------- splitting

#[test]
fn splitting_an_open_curve_at_a_point_makes_two_that_keep_the_shape() {
    let curve = s_curve();
    let pieces = split(&curve, &[id(1)]).unwrap();
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces[0].points().len(), 2);
    assert_eq!(pieces[1].points().len(), 2);
    assert_eq!(pieces[0].points()[1].id, id(1));
    assert_eq!(pieces[1].points()[0].id, id(1));
    for (piece, span) in pieces.iter().zip(curve.spans()) {
        assert_eq!(piece.spans()[0], span);
        assert!(!piece.closed());
        ids_unique(piece);
    }
    // The ends of an open curve cut nothing.
    assert_eq!(split(&curve, &[id(0), id(2)]).unwrap(), vec![curve.clone()]);
    assert_eq!(split(&curve, &[]).unwrap(), vec![curve]);
}

#[test]
fn splitting_a_closed_curve_opens_it_or_cuts_it_in_pieces() {
    let closed = square(true);
    let opened = split(&closed, &[id(2)]).unwrap();
    assert_eq!(opened.len(), 1);
    let open = &opened[0];
    assert!(!open.closed());
    assert_eq!(open.points().len(), 5);
    assert_eq!(open.points()[0].id, id(2));
    assert_eq!(open.points()[4].id, id(4), "the copy gets a new id");
    assert_eq!(open.points()[0].position, open.points()[4].position);
    same_shape(open, &closed, 1e-3);
    ids_unique(open);
    let halves = split(&closed, &[id(0), id(2)]).unwrap();
    assert_eq!(halves.len(), 2);
    assert!(halves.iter().all(|h| h.points().len() == 3 && !h.closed()));
    // A two-point curve has nothing inside to cut at.
    let two = Curve::corners(&[Vec3::ZERO, Vec3::X], false).unwrap();
    assert_eq!(split(&two, &[id(0)]).unwrap(), vec![two]);
}

// ------------------------------------------------------- rounding corners

#[test]
fn rounding_a_corner_makes_an_arc_of_the_radius() {
    let curve = square(false);
    let round = curve
        .edited(&CurveEdit::RoundCorners {
            points: vec![id(1)],
            radius: 20.0,
        })
        .unwrap();
    assert_eq!(round.points().len(), 5);
    let (a, b) = (round.points()[1], round.points()[2]);
    assert_eq!(a.id, id(1));
    assert_eq!(b.id, id(4));
    // A square corner of radius 20: the arc starts 20 back along each leg.
    assert!(a.position.distance(Vec3::new(80.0, 0.0, 0.0)) < 1e-3);
    assert!(b.position.distance(Vec3::new(100.0, 20.0, 0.0)) < 1e-3);
    // Its middle is 20 from the arc's centre (80, 20).
    let middle = round.spans()[1].at(0.5);
    assert!((middle.distance(Vec3::new(80.0, 20.0, 0.0)) - 20.0).abs() < 0.05);
    // Tangent to the legs: no kink where the arc meets them.
    assert!(a.handle_out.normalize().distance(Vec3::X) < 1e-4);
    assert!(b.handle_in.normalize().distance(-Vec3::Y) < 1e-4);
    ids_unique(&round);
}

#[test]
fn a_radius_too_big_is_cut_back_to_what_fits() {
    let curve = square(true);
    let all: Vec<_> = (0..4).map(id).collect();
    let round = curve
        .edited(&CurveEdit::RoundCorners {
            points: all,
            radius: 1e6,
        })
        .unwrap();
    assert_eq!(round.points().len(), 8);
    // Neighbouring corners share each leg: each arc reaches half of it.
    assert!(
        round.points()[0]
            .position
            .distance(Vec3::new(0.0, 50.0, 0.0))
            < 0.1
    );
    assert!(
        round.points()[1]
            .position
            .distance(Vec3::new(50.0, 0.0, 0.0))
            < 0.1
    );
    // A circle, near enough.
    for p in round.polyline(16) {
        let d = p.distance(Vec3::new(50.0, 50.0, 0.0));
        assert!((d - 50.0).abs() < 0.2, "{d}");
    }
    // Next to a corner that isn't rounded the arc may use the whole leg.
    let one = square(false)
        .edited(&CurveEdit::RoundCorners {
            points: vec![id(1)],
            radius: 1e6,
        })
        .unwrap();
    assert!(one.points()[1].position.x < 1.0);
}

#[test]
fn rounding_leaves_what_isnt_a_sharp_corner() {
    let curve = square(false);
    for (points, radius) in [
        (vec![id(0), id(3)], 10.0),
        (vec![id(1)], 0.0),
        (vec![id(1)], -5.0),
        (vec![id(42)], 10.0),
    ] {
        assert_eq!(
            curve
                .edited(&CurveEdit::RoundCorners { points, radius })
                .unwrap(),
            curve
        );
    }
    let smooth = s_curve();
    assert_eq!(
        smooth
            .edited(&CurveEdit::RoundCorners {
                points: vec![id(1)],
                radius: 10.0
            })
            .unwrap(),
        smooth
    );
    let straight = Curve::corners(&[Vec3::ZERO, Vec3::X, Vec3::X * 2.0], false).unwrap();
    assert_eq!(
        straight
            .edited(&CurveEdit::RoundCorners {
                points: vec![id(1)],
                radius: 10.0
            })
            .unwrap(),
        straight
    );
    // On one spot: nothing to round.
    let dot = Curve::corners(&[Vec3::ZERO, Vec3::ZERO, Vec3::X], false).unwrap();
    assert!(
        dot.edited(&CurveEdit::RoundCorners {
            points: vec![id(1)],
            radius: 10.0
        })
        .is_ok()
    );
    assert_eq!(
        curve.edited(&CurveEdit::RoundCorners {
            points: vec![id(1)],
            radius: f32::NAN
        }),
        Err(CurveError::NotANumber)
    );
}

// ------------------------------------------------------------ even spacing

#[test]
fn even_spacing_spreads_points_evenly_along_the_curve() {
    let curve = s_curve();
    let even = curve.edited(&CurveEdit::Resample { count: 9 }).unwrap();
    assert_eq!(even.points().len(), 9);
    // The ends stay.
    assert!(even.points()[0].position.distance(Vec3::ZERO) < 1e-4);
    assert!(
        even.points()[8]
            .position
            .distance(Vec3::new(100.0, 0.0, 0.0))
            < 1e-3
    );
    // Each stretch the same length, near enough.
    let lengths: Vec<f32> = even
        .spans()
        .iter()
        .map(|s| {
            (0..32)
                .map(|k| s.at(k as f32 / 32.0).distance(s.at((k + 1) as f32 / 32.0)))
                .sum()
        })
        .collect();
    let mean = lengths.iter().sum::<f32>() / lengths.len() as f32;
    for l in &lengths {
        assert!((l - mean).abs() < mean * 0.03, "{lengths:?}");
    }
    same_shape(&even, &curve, 0.5);
    // All new ids.
    assert!(even.points().iter().all(|p| p.id.get() >= curve.next_id()));
    ids_unique(&even);
}

#[test]
fn even_spacing_keeps_lines_straight_and_loops_closed() {
    let line = Curve::corners(&[Vec3::ZERO, Vec3::X * 9.0, Vec3::X * 90.0], false).unwrap();
    let even = line.edited(&CurveEdit::Resample { count: 4 }).unwrap();
    let xs: Vec<f32> = even.points().iter().map(|p| p.position.x).collect();
    for (x, want) in xs.iter().zip([0.0, 30.0, 60.0, 90.0]) {
        assert!((x - want).abs() < 0.01, "{xs:?}");
    }
    assert!(even.points().iter().all(|p| p.kind == PointKind::Sharp));
    let circle = ReadyCurve::Circle { radius: 50.0 }.curve();
    let even = circle.edited(&CurveEdit::Resample { count: 7 }).unwrap();
    assert!(even.closed());
    assert_eq!(even.points().len(), 7);
    for p in even.polyline(8) {
        assert!((p.length() - 50.0).abs() < 0.5, "{}", p.length());
    }
    // Too few is brought up to the least that works.
    assert_eq!(
        circle
            .edited(&CurveEdit::Resample { count: 1 })
            .unwrap()
            .points()
            .len(),
        3
    );
    assert_eq!(
        line.edited(&CurveEdit::Resample { count: 0 })
            .unwrap()
            .points()
            .len(),
        2
    );
    // A curve on one spot stays a number.
    let dot = Curve::corners(&[Vec3::ONE, Vec3::ONE], false).unwrap();
    let even = dot.edited(&CurveEdit::Resample { count: 5 }).unwrap();
    assert!(even.points().iter().all(|p| p.position == Vec3::ONE));
}

#[test]
fn a_curve_that_has_used_up_its_ids_says_so() {
    let points = vec![
        CurvePoint {
            id: id(MAX_POINT_ID - 2),
            position: Vec3::ZERO,
            handle_in: Vec3::ZERO,
            handle_out: Vec3::ZERO,
            kind: PointKind::Sharp,
        },
        CurvePoint {
            id: id(MAX_POINT_ID - 1),
            position: Vec3::X,
            handle_in: Vec3::ZERO,
            handle_out: Vec3::ZERO,
            kind: PointKind::Sharp,
        },
    ];
    let full = Curve::from_parts(points, false, MAX_POINT_ID).unwrap();
    assert_eq!(
        full.edited(&CurveEdit::Insert {
            after: id(MAX_POINT_ID - 2),
            t: 0.5
        }),
        Err(CurveError::OutOfIds)
    );
    assert_eq!(
        full.edited(&CurveEdit::Resample { count: 4 }),
        Err(CurveError::OutOfIds)
    );
}

#[test]
fn carrying_on_from_either_end() {
    let curve = Curve::corners(&[Vec3::ZERO, Vec3::X * 10.0], false).unwrap();
    let after = curve
        .edited(&CurveEdit::Extend {
            at_start: false,
            points: vec![(Vec3::X * 20.0, Vec3::ZERO, Vec3::ZERO, PointKind::Sharp)],
        })
        .unwrap();
    assert_eq!(after.points()[2].id, id(2));
    assert_eq!(after.points()[2].position, Vec3::X * 20.0);
    // From the start: in front, the handles turned round to match.
    let before = curve
        .edited(&CurveEdit::Extend {
            at_start: true,
            points: vec![
                (-Vec3::X * 10.0, Vec3::Y, -Vec3::Y, PointKind::Smooth),
                (-Vec3::X * 20.0, Vec3::ZERO, Vec3::ZERO, PointKind::Sharp),
            ],
        })
        .unwrap();
    let xs: Vec<f32> = before.points().iter().map(|p| p.position.x).collect();
    assert_eq!(xs, vec![-20.0, -10.0, 0.0, 10.0]);
    assert_eq!(before.points()[1].handle_out, Vec3::Y);
    ids_unique(&before);
    assert_eq!(
        square(true).edited(&CurveEdit::Extend {
            at_start: false,
            points: vec![]
        }),
        Err(CurveError::NotEnds)
    );
}

#[test]
fn end_points_are_found() {
    let open = square(false);
    assert_eq!(open.end_of(id(0)), Some(CurveEnd::Start));
    assert_eq!(open.end_of(id(3)), Some(CurveEnd::End));
    assert_eq!(open.end_of(id(1)), None);
    assert_eq!(square(true).end_of(id(0)), None);
}

#[test]
fn random_spline_edits_never_panic() {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..300 {
        let count = (next() % 7) as usize + 2;
        let points: Vec<(Vec3, Vec3, Vec3, bool)> = (0..count)
            .map(|_| {
                let mut v = || (next() % 200) as f32 - 100.0;
                let p = Vec3::new(v(), v(), (v() / 4.0).round());
                let h = Vec3::new(v(), v(), 0.0) / 4.0;
                (p, -h, h, next() % 2 == 0)
            })
            .collect();
        let Ok(curve) = Curve::new(points, next() % 3 == 0) else {
            continue;
        };
        let ids: Vec<CurvePointId> = curve.points().iter().map(|p| p.id).collect();
        let pick = ids[(next() as usize) % ids.len()];
        let kind = PointKind::ALL[(next() % 4) as usize];
        let edits = [
            CurveEdit::Insert {
                after: pick,
                t: (next() % 100) as f32 / 100.0,
            },
            CurveEdit::SetKind {
                points: ids.clone(),
                kind,
            },
            CurveEdit::Reverse,
            CurveEdit::RoundCorners {
                points: ids.clone(),
                radius: (next() % 500) as f32,
            },
            CurveEdit::Resample {
                count: (next() % 20) as usize,
            },
            CurveEdit::CloseEnds {
                weld: next() % 2 == 0,
            },
            CurveEdit::Delete {
                parts: vec![CurvePart::point(pick)],
            },
        ];
        for edit in edits {
            if let Ok(changed) = curve.edited(&edit) {
                assert!(changed.points().iter().all(|p| p.position.is_finite()));
                ids_unique(&changed);
            }
        }
        let other = Curve::corners(&[Vec3::ZERO, Vec3::X * (next() % 50) as f32], false).unwrap();
        for (a, b) in [
            (CurveEnd::Start, CurveEnd::End),
            (CurveEnd::End, CurveEnd::Start),
        ] {
            if let Ok(joined) = join(&curve, a, &other, b, next() % 2 == 0) {
                ids_unique(&joined);
            }
        }
        if let Ok(pieces) = split(&curve, &ids[..ids.len() / 2]) {
            for piece in pieces {
                ids_unique(&piece);
            }
        }
    }
}
