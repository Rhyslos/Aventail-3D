//! Tests for paths: measures, splitting, mirroring, changes and odd
//! paths.

use super::*;

fn square() -> Path {
    Path::polygon([
        Vec2::new(0.25, 0.25),
        Vec2::new(0.75, 0.25),
        Vec2::new(0.75, 0.75),
        Vec2::new(0.25, 0.75),
    ])
}

#[test]
fn a_square_has_its_area_inside_and_sides() {
    let s = square();
    assert!((s.area() - 0.25).abs() < 1e-6);
    assert!(s.contains(Vec2::splat(0.5)));
    assert!(!s.contains(Vec2::splat(0.1)));
    assert_eq!(s.sides(), 4);
    assert_eq!(s.bounds(), Some((Vec2::splat(0.25), Vec2::splat(0.75))));
    assert_eq!(s.middle(), Vec2::splat(0.5));
    let (side, t, d) = s.nearest(Vec2::new(0.5, 0.2)).unwrap();
    assert_eq!(side, 0);
    assert!((t - 0.5).abs() < 1e-3 && (d - 0.05).abs() < 1e-4);
}

#[test]
fn an_open_path_has_no_inside() {
    let open = Path::new(square().knots().to_vec(), false);
    assert_eq!(open.area(), 0.0);
    assert!(!open.contains(Vec2::splat(0.5)));
    assert_eq!(open.sides(), 3);
    assert!(open.clone().closed().contains(Vec2::splat(0.5)));
}

#[test]
fn splitting_a_curve_keeps_its_shape() {
    let curved = Path::new(
        vec![
            Knot::smooth(Vec2::new(0.2, 0.5), Vec2::new(0.0, -0.3)),
            Knot::smooth(Vec2::new(0.8, 0.5), Vec2::new(0.0, 0.3)),
        ],
        true,
    );
    let before = curved.point_on_side(0, 0.75).unwrap();
    let split = curved.with_knot_on_side(0, 0.5);
    assert_eq!(split.knots().len(), 3);
    // Three quarters along the old side is half along the new second.
    let after = split.point_on_side(1, 0.5).unwrap();
    assert!(before.distance(after) < 1e-5, "{before} {after}");
    assert!((curved.area() - split.area()).abs() < 1e-5);
    // A straight side gains a corner.
    let s = square().with_knot_on_side(1, 0.5);
    assert_eq!(s.knots()[2], Knot::corner(Vec2::new(0.75, 0.5)));
    // No such side: unchanged.
    assert_eq!(square().with_knot_on_side(9, 0.5), square());
}

#[test]
fn mirroring_and_changes_move_points_and_handles() {
    let p = Path::new(
        vec![
            Knot::smooth(Vec2::new(0.4, 0.2), Vec2::new(0.1, 0.0)),
            Knot::corner(Vec2::new(0.3, 0.6)),
            Knot::corner(Vec2::new(0.45, 0.8)),
        ],
        true,
    );
    let m = p.mirrored(0.5);
    assert_eq!(m.knots()[2].at, Vec2::new(0.6, 0.2));
    assert_eq!(m.knots()[2].inn, Vec2::new(-0.1, 0.0));
    assert!((m.area() - p.area()).abs() < 1e-6);
    let back = m.mirrored(0.5);
    for (a, b) in back.knots().iter().zip(p.knots()) {
        assert!(a.at.distance(b.at) < 1e-6 && a.inn == b.inn && a.out == b.out);
    }
    let moved = p.changed(&Change::moved(Vec2::new(0.1, 0.0)));
    assert_eq!(moved.knots()[1].at, Vec2::new(0.4, 0.6));
    assert_eq!(moved.knots()[0].out, Vec2::new(0.1, 0.0));
    let turned = square().changed(&Change::turned(
        Vec2::splat(0.5),
        std::f32::consts::FRAC_PI_2,
    ));
    assert!((turned.area() - 0.25).abs() < 1e-5);
    let big = square().changed(&Change::scaled(Vec2::splat(0.5), Vec2::new(2.0, 1.0)));
    assert!((big.area() - 0.5).abs() < 1e-5);
    assert!(Change::scaled(Vec2::ZERO, Vec2::new(-1.0, 1.0)).flips());
}

#[test]
fn odd_paths_never_break_the_measures() {
    // No points, one point, two on one spot, a side of no length.
    let empty = Path::default();
    assert_eq!(empty.bounds(), None);
    assert_eq!(empty.nearest(Vec2::ZERO), None);
    assert_eq!(empty.area(), 0.0);
    let one = Path::polygon([Vec2::splat(0.5)]);
    assert!(!one.contains(Vec2::splat(0.5)));
    assert!((one.nearest(Vec2::new(0.5, 0.6)).unwrap().2 - 0.1).abs() < 1e-6);
    let doubled = Path::polygon([Vec2::splat(0.5), Vec2::splat(0.5), Vec2::new(0.9, 0.5)]);
    assert_eq!(doubled.area(), 0.0);
    assert!(doubled.nearest(Vec2::ZERO).is_some());
    // A figure of eight: both loops count as inside.
    let eight = Path::polygon([
        Vec2::new(0.1, 0.1),
        Vec2::new(0.9, 0.9),
        Vec2::new(0.9, 0.1),
        Vec2::new(0.1, 0.9),
    ]);
    assert!(eight.contains(Vec2::new(0.8, 0.5)));
    assert!(eight.contains(Vec2::new(0.2, 0.5)));
    assert!(!eight.contains(Vec2::new(0.5, 0.2)));
    // Too far out, not a number, too many points.
    assert!(Path::polygon([Vec2::splat(20.0)]).problem().is_some());
    assert!(Path::polygon([Vec2::splat(f32::NAN)]).problem().is_some());
    assert!(
        Path::polygon(vec![Vec2::ZERO; MAX_KNOTS + 1])
            .problem()
            .is_some()
    );
    assert!(square().problem().is_none());
    assert_eq!(square().without_knot(0).knots().len(), 3);
    assert_eq!(square().without_knot(9), square());
}
