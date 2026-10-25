use super::*;
use crate::curve::ReadyCurve;
use crate::primitives::{CubeParams, CylinderParams};
use crate::{MeshReport, Primitive, VertexId};

/// A bar along X: 100 long, 10 by 10 across, centred on the origin
/// along X and Y and standing on it (Z from 0 to 10, as primitives
/// stand), cut into `segments` along its length.
fn bar(segments: u32) -> Mesh {
    Primitive::Cube(CubeParams {
        size: Vec3::new(100.0, 10.0, 10.0),
        segments: [segments, 1, 1],
    })
    .mesh()
}

fn line(from: Vec3, to: Vec3) -> Curve {
    Curve::corners(&[from, to], false).unwrap()
}

fn circle(radius: f32) -> Curve {
    ReadyCurve::Circle { radius }.curve()
}

fn options() -> BendOptions {
    BendOptions::default()
}

fn near(a: Vec3, b: Vec3, by: f32) -> bool {
    a.distance(b) < by
}

/// Each point's place before and after.
fn pairs(before: &Mesh, after: &Mesh) -> Vec<(Vec3, Vec3)> {
    before
        .vertices()
        .map(|(id, p)| (p, after.position(id).unwrap()))
        .collect()
}

#[test]
fn a_straight_curve_along_the_axis_only_moves_the_shape_to_its_start() {
    let shape = bar(4);
    let bounds = shape.bounds().unwrap();
    let curve = line(Vec3::ZERO, Vec3::new(300.0, 0.0, 0.0));
    let out = bend_along(&shape, &curve, Vec3::ZERO, &options()).unwrap();
    for (p, q) in pairs(&shape, &out) {
        let expected = p - Vec3::X * bounds.min.x;
        assert!(near(q, expected, 1e-3), "{p} went to {q}");
    }
    // Ids, polygons and everything else stay.
    assert!(out.same_structure(&shape));
}

#[test]
fn each_axis_can_run_along_the_curve() {
    let shape = bar(1);
    for (axis, way) in [
        (BendAxis::X, Vec3::X),
        (BendAxis::Y, Vec3::Y),
        (BendAxis::Z, Vec3::Z),
    ] {
        let curve = line(Vec3::ZERO, way * 500.0);
        let out = bend_along(
            &shape,
            &curve,
            Vec3::ZERO,
            &BendOptions { axis, ..options() },
        )
        .unwrap();
        let min = shape.bounds().unwrap().min;
        for (p, q) in pairs(&shape, &out) {
            let expected = p - way * min.dot(way);
            assert!(near(q, expected, 1e-3), "{axis:?}: {p} went to {q}");
        }
    }
}

#[test]
fn the_longest_axis_is_found() {
    let tall = Primitive::Cube(CubeParams {
        size: Vec3::new(10.0, 20.0, 300.0),
        segments: [1, 1, 1],
    })
    .mesh();
    assert_eq!(BendAxis::Longest.resolved(&tall.bounds().unwrap()), 2);
    assert_eq!(BendAxis::Longest.resolved(&bar(1).bounds().unwrap()), 0);
    let wide = Bounds::new(Vec3::ZERO, Vec3::new(1.0, 5.0, 2.0)).unwrap();
    assert_eq!(BendAxis::Longest.resolved(&wide), 1);
    // A tie goes to X.
    let cube = Bounds::new(Vec3::ZERO, Vec3::ONE).unwrap();
    assert_eq!(BendAxis::Longest.resolved(&cube), 0);
}

#[test]
fn a_bar_bent_round_a_circle_follows_it() {
    let radius = 100.0;
    let shape = bar(32);
    let out = bend_along(&shape, &circle(radius), Vec3::ZERO, &options()).unwrap();
    // The bar's middle line (y = 0) lies on the circle; its sides (y = ±5)
    // a little inside and outside, its top and bottom (z = ±5) above and
    // below it.
    for (p, q) in pairs(&shape, &out) {
        let flat = Vec2::new(q.x, q.y).length();
        assert!(
            (flat - (radius - p.y)).abs() < 0.2 || (flat - (radius + p.y)).abs() < 0.2,
            "{p} went to {q} ({flat})"
        );
        assert!((q.z - p.z).abs() < 1e-3, "the circle lies flat: {p} {q}");
    }
    // Still a clean closed solid of the same polygons.
    assert!(out.same_structure(&shape));
    assert!(!MeshReport::new(&out).has_problems());
}

use glam::Vec2;

#[test]
fn stretch_to_fit_makes_the_shape_as_long_as_the_curve() {
    let shape = bar(1);
    let curve = line(Vec3::ZERO, Vec3::new(400.0, 0.0, 0.0));
    let out = bend_along(
        &shape,
        &curve,
        Vec3::ZERO,
        &BendOptions {
            stretch: true,
            ..options()
        },
    )
    .unwrap();
    let bounds = out.bounds().unwrap();
    assert!(near(bounds.min, Vec3::new(0.0, -5.0, 0.0), 1e-3));
    assert!(near(bounds.max, Vec3::new(400.0, 5.0, 10.0), 1e-3));
    // A flat shape along the axis can't be stretched: it stays its size.
    let flat = Primitive::Plane(crate::primitives::PlaneParams {
        width: 50.0,
        depth: 50.0,
        segments: [1, 1],
    })
    .mesh();
    let out = bend_along(
        &flat,
        &line(Vec3::ZERO, Vec3::new(0.0, 0.0, 400.0)),
        Vec3::ZERO,
        &BendOptions {
            axis: BendAxis::Z,
            stretch: true,
            ..options()
        },
    )
    .unwrap();
    assert!(
        out.vertices()
            .all(|(_, p)| p.is_finite() && p.z.abs() < 1e-3)
    );
}

#[test]
fn offset_slides_the_shape_along() {
    let shape = bar(1);
    let curve = line(Vec3::ZERO, Vec3::new(400.0, 0.0, 0.0));
    let out = bend_along(
        &shape,
        &curve,
        Vec3::ZERO,
        &BendOptions {
            offset: 120.0,
            ..options()
        },
    )
    .unwrap();
    let bounds = out.bounds().unwrap();
    assert!((bounds.min.x - 120.0).abs() < 1e-3);
    assert!((bounds.max.x - 220.0).abs() < 1e-3);
}

#[test]
fn past_an_open_curves_ends_the_shape_carries_on_straight() {
    // An L: along X then up Y. The bar starts 50 before the curve and ends
    // past it.
    let curve = Curve::corners(
        &[
            Vec3::ZERO,
            Vec3::new(20.0, 0.0, 0.0),
            Vec3::new(20.0, 20.0, 0.0),
        ],
        false,
    )
    .unwrap();
    let shape = bar(10);
    let out = bend_along(
        &shape,
        &curve,
        Vec3::ZERO,
        &BendOptions {
            offset: -50.0,
            ..options()
        },
    )
    .unwrap();
    // The far end goes on up Y, the near end back along -X.
    let bounds = out.bounds().unwrap();
    assert!(bounds.max.y > 25.0, "{bounds:?}");
    assert!(bounds.min.x < -45.0, "{bounds:?}");
    assert!(out.vertices().all(|(_, p)| p.is_finite()));
}

#[test]
fn round_a_closed_curve_a_stretched_bar_meets_itself() {
    let shape = bar(64);
    let out = bend_along(
        &shape,
        &circle(80.0),
        Vec3::ZERO,
        &BendOptions {
            stretch: true,
            ..options()
        },
    )
    .unwrap();
    // The bar's two end faces land on the same spot.
    let bounds = shape.bounds().unwrap();
    let ends: Vec<Vec3> = shape
        .vertices()
        .filter(|(_, p)| (p.x - bounds.min.x).abs() < 1e-3)
        .map(|(id, p)| {
            let q = out.position(id).unwrap();
            // The point at the other end with the same y and z.
            let (other, _) = shape
                .vertices()
                .find(|(_, o)| (o.x - bounds.max.x).abs() < 1e-3 && near(*o, p.with_x(o.x), 1e-3))
                .unwrap();
            q - out.position(other).unwrap()
        })
        .collect();
    assert!(!ends.is_empty());
    for gap in ends {
        assert!(gap.length() < 0.5, "{gap}");
    }
    // Going round again past the end wraps.
    let long = bend_along(
        &shape,
        &circle(10.0),
        Vec3::ZERO,
        &BendOptions { ..options() },
    )
    .unwrap();
    for (_, q) in long.vertices() {
        assert!(Vec2::new(q.x, q.y).length() < 16.0, "{q}");
    }
}

#[test]
fn the_pivot_is_where_the_curve_runs_through_the_shape() {
    let shape = bar(1);
    let curve = line(Vec3::ZERO, Vec3::new(300.0, 0.0, 0.0));
    // With the pivot at the bar's foot, the bar rides on the curve.
    let out = bend_along(&shape, &curve, Vec3::ZERO, &options()).unwrap();
    let bounds = out.bounds().unwrap();
    assert!((bounds.min.z - 0.0).abs() < 1e-3 && (bounds.max.z - 10.0).abs() < 1e-3);
    // With it in the middle, the curve runs through the bar.
    let out = bend_along(&shape, &curve, Vec3::new(0.0, 0.0, 5.0), &options()).unwrap();
    let bounds = out.bounds().unwrap();
    assert!((bounds.min.z + 5.0).abs() < 1e-3 && (bounds.max.z - 5.0).abs() < 1e-3);
}

#[test]
fn twist_turns_the_shape_round_the_curve() {
    let shape = bar(16);
    let curve = line(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0));
    let out = bend_along(
        &shape,
        &curve,
        Vec3::ZERO,
        &BendOptions {
            twist: 180.0,
            ..options()
        },
    )
    .unwrap();
    let bounds = shape.bounds().unwrap();
    for (p, q) in pairs(&shape, &out) {
        if (p.x - bounds.max.x).abs() < 1e-3 {
            // Half a turn: across and up both flipped.
            assert!(near(q, Vec3::new(100.0, -p.y, -p.z), 0.05), "{p} {q}");
        }
        if (p.x - bounds.min.x).abs() < 1e-3 {
            assert!(near(q, Vec3::new(0.0, p.y, p.z), 1e-3), "{p} {q}");
        }
    }
}

#[test]
fn a_curve_with_no_length_is_refused_and_an_empty_shape_kept() {
    let dot = Curve::corners(&[Vec3::ONE, Vec3::ONE], false).unwrap();
    assert_eq!(
        bend_along(&bar(1), &dot, Vec3::ZERO, &options()),
        Err(BendProblem::NoLength)
    );
    assert!(!BendProblem::NoLength.to_string().is_empty());
    let empty = Mesh::new();
    let out = bend_along(&empty, &circle(10.0), Vec3::ZERO, &options()).unwrap();
    assert_eq!(out, empty);
}

#[test]
fn the_curve_never_flips_the_shape_up_a_spiral() {
    let spiral = ReadyCurve::Spiral {
        radius: 100.0,
        height: 300.0,
        turns: 3.0,
    }
    .curve();
    let shape = Primitive::Cube(CubeParams {
        size: Vec3::new(1500.0, 10.0, 10.0),
        segments: [200, 1, 1],
    })
    .mesh();
    let out = bend_along(&shape, &spiral, Vec3::ZERO, &options()).unwrap();
    // Neighbouring points along the bar stay about as far apart as they
    // were: no flip tears it.
    for (_, corners) in out.faces() {
        for pair in corners.windows(2) {
            let (a, b) = (
                out.position(pair[0]).unwrap(),
                out.position(pair[1]).unwrap(),
            );
            assert!(a.distance(b) < 20.0, "{a} {b}");
        }
    }
}

#[test]
fn degenerate_shapes_keep_their_polygons() {
    let mut mesh = Mesh::new();
    let p: Vec<VertexId> = [
        Vec3::ZERO,
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 0.0), // a zero-length edge
        Vec3::new(20.0, 5.0, 0.0),
        Vec3::new(5.0, 10.0, 0.0),
        Vec3::new(5.0, -10.0, 3.0),
        Vec3::new(5.0, 0.0, 9.0),
    ]
    .iter()
    .map(|p| mesh.add_vertex(*p).unwrap())
    .collect();
    // A five-sided polygon, and three triangles on one edge (non-manifold,
    // with open borders).
    mesh.add_face(&[p[0], p[1], p[2], p[3], p[4]]).unwrap();
    mesh.add_face(&[p[0], p[1], p[5]]).unwrap();
    mesh.add_face(&[p[1], p[0], p[6]]).unwrap();
    let out = bend_along(&mesh, &circle(30.0), Vec3::ZERO, &options()).unwrap();
    assert!(out.same_structure(&mesh));
    assert!(out.vertices().all(|(_, p)| p.is_finite()));
    // Cylinders' poles and n-gon caps too.
    let cylinder = Primitive::Cylinder(CylinderParams::default()).mesh();
    let out = bend_along(&cylinder, &circle(30.0), Vec3::ZERO, &options()).unwrap();
    assert!(out.same_structure(&cylinder));
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let wild = BendOptions {
        axis: BendAxis::Y,
        stretch: true,
        offset: f32::NAN,
        twist: 1e12,
    }
    .clamped();
    assert_eq!(wild.offset, 0.0);
    assert_eq!(wild.twist, BendOptions::MAX_TWIST);
    let far = BendOptions {
        offset: -1e30,
        twist: f32::NEG_INFINITY,
        ..options()
    }
    .clamped();
    assert_eq!(far.offset, -BendOptions::MAX_OFFSET);
    assert_eq!(far.twist, 0.0);
}

#[test]
fn the_same_settings_give_the_same_shape() {
    let shape = bar(8);
    let o = BendOptions {
        twist: 90.0,
        offset: 7.0,
        ..options()
    };
    let a = bend_along(&shape, &circle(50.0), Vec3::ONE, &o).unwrap();
    let b = bend_along(&shape, &circle(50.0), Vec3::ONE, &o).unwrap();
    assert_eq!(a, b);
}

#[test]
fn random_settings_never_give_a_point_that_isnt_a_number() {
    let shape = bar(8);
    let curves = [
        circle(50.0),
        line(Vec3::ZERO, Vec3::Z * 100.0),
        ReadyCurve::Arc {
            radius: 1e-3,
            angle: 359.0,
        }
        .curve(),
        Curve::corners(
            &[
                Vec3::ZERO,
                Vec3::X * 10.0,
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, 1e5),
            ],
            true,
        )
        .unwrap(),
    ];
    let values = [0.0, -1.0, 1.0, 1e9, -1e9, f32::NAN, f32::INFINITY, 1e-9];
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..200 {
        let o = BendOptions {
            axis: BendAxis::ALL[(next() % 4) as usize],
            stretch: next() % 2 == 0,
            offset: values[(next() % values.len() as u64) as usize],
            twist: values[(next() % values.len() as u64) as usize],
        };
        let curve = &curves[(next() % curves.len() as u64) as usize];
        let pivot = Vec3::splat(values[(next() % values.len() as u64) as usize]);
        if let Ok(out) = bend_along(&shape, curve, pivot, &o) {
            assert!(out.vertices().all(|(_, p)| p.is_finite()), "{o:?}");
        }
    }
}

#[test]
#[ignore = "timing: run with --release -- --ignored --nocapture"]
fn timings() {
    let shape = Primitive::Sphere(crate::primitives::SphereParams {
        radius: 50.0,
        sides: 512,
        rings: 256,
    })
    .mesh();
    let curve = ReadyCurve::Spiral {
        radius: 100.0,
        height: 300.0,
        turns: 3.0,
    }
    .curve();
    let start = std::time::Instant::now();
    let out = bend_along(&shape, &curve, Vec3::ZERO, &options()).unwrap();
    eprintln!(
        "{} points bent along a spiral in {:.1} ms",
        out.vertex_count(),
        start.elapsed().as_secs_f64() * 1000.0
    );
}
