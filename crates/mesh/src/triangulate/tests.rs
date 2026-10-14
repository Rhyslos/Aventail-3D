//! Tests for cutting polygons into triangles.

use super::*;

fn cut(points: &[Vec3]) -> Vec<[usize; 3]> {
    let mut out = Vec::new();
    triangulate(points.len(), |i| points[i], |t| out.push(t));
    out
}

/// Twice the area all triangles cover, along `normal`.
fn area(points: &[Vec3], triangles: &[[usize; 3]], normal: Vec3) -> f32 {
    triangles
        .iter()
        .map(|[a, b, c]| {
            (points[*b] - points[*a])
                .cross(points[*c] - points[*a])
                .dot(normal)
        })
        .sum()
}

/// Every triangle faces the polygon's way, each corner is used, and there
/// are n - 2 of them.
fn sound(points: &[Vec3], triangles: &[[usize; 3]]) {
    let n = points.len();
    assert_eq!(triangles.len(), n - 2, "{triangles:?}");
    let normal = newell(points).normalize();
    for t in triangles {
        let [a, b, c] = t.map(|i| points[i]);
        assert!(
            (b - a).cross(c - a).dot(normal) > -1e-3,
            "{t:?} faces the other way"
        );
    }
    for i in 0..n {
        assert!(
            triangles.iter().any(|t| t.contains(&i)),
            "corner {i} unused"
        );
    }
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

#[test]
fn small_polygons() {
    assert!(cut(&[]).is_empty());
    assert!(cut(&[Vec3::ZERO, Vec3::X]).is_empty());
    assert_eq!(cut(&[Vec3::ZERO, Vec3::X, Vec3::Y]), vec![[0, 1, 2]]);
    // A square: corner 0 to 2, as before.
    let square = [v(0., 0., 0.), v(1., 0., 0.), v(1., 1., 0.), v(0., 1., 0.)];
    assert_eq!(cut(&square), vec![[0, 1, 2], [0, 2, 3]]);
}

#[test]
fn a_dented_quad_is_cut_inside() {
    // An arrowhead whose dent is at corner 3: 0 to 2 would run outside.
    let arrow = [v(0., 0., 0.), v(2., 1., 0.), v(0., 2., 0.), v(0.5, 1., 0.)];
    let t = cut(&arrow);
    assert_eq!(t, vec![[0, 1, 3], [1, 2, 3]]);
    sound(&arrow, &t);
    let total = area(&arrow, &t, Vec3::Z);
    // Its area, 1.5 (twice: 3), with no overlap.
    assert!((total - 3.0).abs() < 1e-4, "{total}");
    // The dent at corner 0 instead: 0 to 2 is right.
    let other = [v(0.5, 1., 0.), v(0., 0., 0.), v(2., 1., 0.), v(0., 2., 0.)];
    sound(&other, &cut(&other));
    assert!((area(&other, &cut(&other), Vec3::Z) - 3.0).abs() < 1e-4);
}

#[test]
fn flat_round_polygons_are_fans() {
    let ring: Vec<Vec3> = (0..64)
        .map(|i| {
            let a = i as f32 / 64.0 * std::f32::consts::TAU;
            v(a.cos() * 50.0, a.sin() * 50.0, 10.0)
        })
        .collect();
    let t = cut(&ring);
    assert!(t.iter().all(|t| t[0] == 0), "a fan");
    sound(&ring, &t);
}

#[test]
fn a_dented_polygon_stays_inside() {
    // An L shape, flat.
    let l = [
        v(0., 0., 0.),
        v(2., 0., 0.),
        v(2., 1., 0.),
        v(1., 1., 0.),
        v(1., 2., 0.),
        v(0., 2., 0.),
    ];
    let t = cut(&l);
    sound(&l, &t);
    // Area 3 (twice: 6) exactly: nothing outside, nothing twice.
    assert!((area(&l, &t, Vec3::Z) - 6.0).abs() < 1e-4);
    let flat: Vec<Vec2> = l.iter().map(|p| Vec2::new(p.x, p.y)).collect();
    for [a, b, c] in &t {
        for (x, y) in [(a, b), (b, c), (c, a)] {
            let side = (*x + 1) % 6 == *y || (*y + 1) % 6 == *x;
            assert!(side || inside(&flat, *x, *y), "{x}–{y} runs outside");
        }
    }
}

#[test]
fn a_bent_polygon_is_cut_along_its_fold() {
    // Two sides of a cube merged into one polygon round the corner, as
    // Dissolve leaves it: the fold is the line from corner 1 to 4.
    let bent = [
        v(-50., -50., 0.),
        v(50., -50., 0.),
        v(50., 50., 0.),
        v(50., 50., 100.),
        v(50., -50., 100.),
        v(-50., -50., 100.),
    ];
    let t = cut(&bent);
    sound(&bent, &t);
    let uses_fold = t.iter().any(|t| t.contains(&1) && t.contains(&4));
    assert!(uses_fold, "{t:?}");
    // Every triangle lies on one of the two sides.
    for tri in &t {
        let [a, b, c] = tri.map(|i| bent[i]);
        let front = [a, b, c].iter().all(|p| (p.y + 50.0).abs() < 1e-3);
        let right = [a, b, c].iter().all(|p| (p.x - 50.0).abs() < 1e-3);
        assert!(front || right, "{tri:?} cuts across the corner");
    }
}

#[test]
fn big_dented_polygons_are_cut_ear_by_ear() {
    // A star with many points: too many corners for the shortest cut.
    let n = 2 * (BEST_CUT_UP_TO + 4);
    let star: Vec<Vec3> = (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let r = if i % 2 == 0 { 100.0 } else { 60.0 };
            v(a.cos() * r, a.sin() * r, 0.0)
        })
        .collect();
    let t = cut(&star);
    sound(&star, &t);
    let flat_area: f32 = (0..n)
        .map(|i| {
            let (a, b) = (star[i], star[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    assert!((area(&star, &t, Vec3::Z) - flat_area).abs() < 1e-1);
}

#[test]
fn degenerate_polygons_fall_back_to_a_fan() {
    // All corners on one line: no area, no facing.
    let line: Vec<Vec3> = (0..6).map(|i| v(i as f32, 0., 0.)).collect();
    let t = cut(&line);
    assert_eq!(t.len(), 4);
    // Repeated corners (points in one place).
    let same = [Vec3::ZERO; 5];
    assert_eq!(cut(&same).len(), 3);
    // A polygon crossing itself (a bow tie with a middle point).
    let tie = [
        v(0., 0., 0.),
        v(2., 2., 0.),
        v(2., 0., 0.),
        v(0., 2., 0.),
        v(-1., 1., 0.),
    ];
    assert_eq!(cut(&tie).len(), 3);
}

#[test]
fn every_shape_and_edit_cuts_cleanly() {
    // Random polygons, flat or bent: always n - 2 triangles with corners
    // in range, never a panic.
    let mut seed = 7u32;
    let mut rand = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 1000) as f32 / 10.0
    };
    for n in 3..40 {
        let points: Vec<Vec3> = (0..n).map(|_| v(rand(), rand(), rand() * 0.1)).collect();
        let t = cut(&points);
        assert_eq!(t.len(), n - 2);
        assert!(t.iter().flatten().all(|i| *i < n));
    }
}
