//! Tests for convex hulls (decision 0151).

use super::*;
use crate::Primitive;
use crate::primitives::{CubeParams, SphereParams};

fn cube_points() -> Vec<Vec3> {
    let mut out = Vec::new();
    for i in 0..8 {
        out.push(Vec3::new(
            if i & 1 == 0 { -1.0 } else { 1.0 },
            if i & 2 == 0 { -1.0 } else { 1.0 },
            if i & 4 == 0 { -1.0 } else { 1.0 },
        ));
    }
    out
}

/// Every triangle faces out (away from the middle), and every edge is
/// used twice, once each way: a closed solid.
fn assert_closed_and_facing_out(hull: &Hull) {
    let middle = hull.points.iter().copied().sum::<Vec3>() / hull.points.len() as f32;
    let mut edges = HashMap::new();
    for t in &hull.triangles {
        let [a, b, c] = t.map(|i| hull.points[i as usize]);
        let n = (b - a).cross(c - a);
        assert!(n.dot(a - middle) > 0.0, "a triangle faces in");
        for k in 0..3 {
            *edges.entry((t[k], t[(k + 1) % 3])).or_insert(0) += 1;
        }
    }
    for (&(p, q), &n) in &edges {
        assert_eq!(n, 1, "an edge is used twice the same way");
        assert_eq!(edges.get(&(q, p)), Some(&1), "an open edge");
    }
}

/// A little pseudo-random cloud (the same every time).
fn cloud(count: usize, seed: u32) -> Vec<Vec3> {
    let mut state = seed.wrapping_mul(2_654_435_761).max(1);
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    (0..count)
        .map(|_| Vec3::new(next() * 50.0, next() * 20.0, next() * 35.0))
        .collect()
}

#[test]
fn a_cube_wraps_to_its_eight_corners() {
    let mut points = cube_points();
    // Points inside and on its sides change nothing.
    points.push(Vec3::ZERO);
    points.push(Vec3::new(1.0, 0.2, 0.3));
    points.push(Vec3::new(0.0, 0.0, 1.0));
    let hull = convex_hull(&points).unwrap();
    assert_eq!(hull.points.len(), 8);
    assert_eq!(hull.triangles.len(), 12);
    assert_closed_and_facing_out(&hull);
    assert!((hull.volume() - 8.0).abs() < 1e-9);
    // Drawn as its twelve edges, not the diagonals across its sides.
    assert_eq!(hull.outline().len(), 12);
}

#[test]
fn the_hull_holds_every_point() {
    for seed in 1..20 {
        let points = cloud(500, seed);
        let hull = convex_hull(&points).unwrap();
        assert_closed_and_facing_out(&hull);
        for p in &points {
            assert!(hull.contains(*p, 1e-3), "a point is left outside");
        }
        // Its corners are some of the points.
        for c in &hull.points {
            assert!(points.contains(c));
        }
    }
}

#[test]
fn a_sphere_mesh_wraps_to_all_its_points() {
    let sphere = Primitive::Sphere(SphereParams::default()).mesh();
    let points: Vec<Vec3> = sphere.vertices().map(|(_, p)| p).collect();
    let hull = convex_hull(&points).unwrap();
    assert_eq!(hull.points.len(), points.len());
    assert_closed_and_facing_out(&hull);
}

#[test]
fn a_limited_hull_stops_at_its_corners_and_stays_inside() {
    let sphere = Primitive::Sphere(SphereParams::default()).mesh();
    let points: Vec<Vec3> = sphere.vertices().map(|(_, p)| p).collect();
    let full = convex_hull(&points).unwrap();
    let limited = convex_hull_limited(&points, 32).unwrap();
    assert!(limited.points.len() <= 32);
    assert!(limited.points.len() >= 20, "{}", limited.points.len());
    assert_closed_and_facing_out(&limited);
    for p in &limited.points {
        assert!(full.contains(*p, 1e-3));
    }
    // Far enough points first: most of the sphere is held.
    assert!(limited.volume() > 0.6 * full.volume());
}

#[test]
fn the_same_points_give_the_same_hull() {
    let points = cloud(2_000, 7);
    assert_eq!(convex_hull(&points), convex_hull(&points));
    assert_eq!(
        convex_hull_limited(&points, 16),
        convex_hull_limited(&points, 16)
    );
}

#[test]
fn flat_and_empty_points_have_no_hull() {
    assert_eq!(convex_hull(&[]), Err(HullProblem::Empty));
    assert_eq!(
        convex_hull(&[Vec3::NAN, Vec3::INFINITY]),
        Err(HullProblem::Empty)
    );
    assert_eq!(convex_hull(&[Vec3::ONE; 5]), Err(HullProblem::Flat));
    let line: Vec<Vec3> = (0..10).map(|i| Vec3::X * i as f32).collect();
    assert_eq!(convex_hull(&line), Err(HullProblem::Flat));
    let plane: Vec<Vec3> = (0..30)
        .map(|i| Vec3::new((i % 6) as f32, (i / 6) as f32 * 3.0, 2.0))
        .collect();
    assert_eq!(convex_hull(&plane), Err(HullProblem::Flat));
    assert!(!HullProblem::Flat.to_string().is_empty());
    assert!(!HullProblem::Empty.to_string().is_empty());
}

#[test]
fn points_not_at_real_places_are_skipped() {
    let mut points = cube_points();
    points.push(Vec3::new(f32::NAN, 0.0, 0.0));
    points.push(Vec3::new(f32::INFINITY, 0.0, 0.0));
    let hull = convex_hull(&points).unwrap();
    assert_eq!(hull.points.len(), 8);
}

#[test]
fn many_points_on_a_grid_keep_flat_sides_flat() {
    // A cube's points, many on each side: only the corners remain.
    let cube = Primitive::Cube(CubeParams {
        segments: [6, 6, 6],
        ..CubeParams::default()
    })
    .mesh();
    let points: Vec<Vec3> = cube.vertices().map(|(_, p)| p).collect();
    let hull = convex_hull(&points).unwrap();
    assert_eq!(hull.points.len(), 8);
    assert_eq!(hull.outline().len(), 12);
    assert_closed_and_facing_out(&hull);
}

#[test]
fn huge_and_tiny_shapes_wrap() {
    for scale in [1e-4_f32, 1e6] {
        let points: Vec<Vec3> = cloud(200, 3).iter().map(|p| *p * scale).collect();
        let hull = convex_hull(&points).unwrap();
        assert_closed_and_facing_out(&hull);
        for p in &points {
            assert!(hull.contains(*p, scale * 1e-3));
        }
    }
}

#[test]
fn nearly_flat_shapes_still_wrap_if_they_have_any_thickness() {
    let mut points = cube_points();
    for p in &mut points {
        p.z *= 0.01;
    }
    let hull = convex_hull(&points).unwrap();
    assert_eq!(hull.points.len(), 8);
}

#[test]
fn a_turned_cube_keeps_its_eight_corners() {
    let cube = Primitive::Cube(CubeParams {
        segments: [4, 3, 5],
        ..CubeParams::default()
    })
    .mesh();
    let turn = glam::Quat::from_euler(glam::EulerRot::XYZ, 0.3, 0.7, -1.1);
    let points: Vec<Vec3> = cube.vertices().map(|(_, p)| turn * p).collect();
    let hull = convex_hull(&points).unwrap();
    assert_eq!(hull.points.len(), 8);
    assert_eq!(hull.outline().len(), 12);
    for p in &points {
        assert!(hull.contains(*p, 1e-3));
    }
}

#[test]
fn awkward_clouds_never_panic_and_stay_closed() {
    // Points snapped to a coarse grid: many on the same planes and lines,
    // and many the same.
    for seed in 1..40 {
        let points: Vec<Vec3> = cloud(300, seed)
            .iter()
            .map(|p| (*p / 10.0).round() * 10.0)
            .collect();
        if let Ok(hull) = convex_hull(&points) {
            assert_closed_and_facing_out(&hull);
            for p in &points {
                assert!(hull.contains(*p, 1e-3));
            }
        }
        let limited = convex_hull_limited(&points, 8);
        if let Ok(hull) = limited {
            assert!(hull.points.len() <= 8);
        }
    }
}

/// A 10 cm box with a ridge `h` high along its top (a very shallow tent),
/// moved to `at`.
fn tent(h: f32, at: Vec3) -> Vec<Vec3> {
    let mut points = Vec::new();
    for x in [0.0, 10.0] {
        for y in [0.0, 10.0] {
            for z in [0.0, 10.0] {
                points.push(Vec3::new(x, y, z));
            }
        }
    }
    points.push(Vec3::new(5.0, 0.0, 10.0 + h));
    points.push(Vec3::new(5.0, 10.0, 10.0 + h));
    points.into_iter().map(|p| p + at).collect()
}

#[test]
fn a_shallow_ridge_keeps_its_corners_far_from_the_origin_too() {
    // The same-side tolerance follows the hull's own size (and the
    // rounding of its points' places), not how far it is from the origin:
    // a ridge 0.035 cm high on a 10 cm box is a real edge wherever the box
    // is. (Measured from the origin, 100 m away it was taken for flat.)
    for at in [Vec3::ZERO, Vec3::new(100_000.0, 0.0, 0.0)] {
        let hull = convex_hull(&tent(0.035, at)).unwrap();
        assert_eq!(hull.points.len(), 10, "at {at}");
        assert_closed_and_facing_out(&hull);
    }
    // Flat to within the points' rounding: no ridge.
    let hull = convex_hull(&tent(0.0, Vec3::ZERO)).unwrap();
    assert_eq!(hull.points.len(), 8);
}
