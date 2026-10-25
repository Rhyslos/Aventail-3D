use super::*;
use crate::primitives::CylinderParams;
use crate::{Primitive, PrimitiveKind, VertexId};
use glam::Vec2;
use std::f32::consts::PI;

fn near(a: Vec3, b: Vec3) -> bool {
    a.distance(b) < 1e-2
}

/// A pole 10 cm across and `height` tall, standing on its pivot, cut
/// into 20 slices.
fn pole(height: f32) -> Mesh {
    Primitive::Cylinder(CylinderParams {
        radius: 5.0,
        height,
        sides: 8,
        height_segments: 20,
        caps: true,
        cap_segments: 1,
        cap_centre: false,
    })
    .mesh()
}

fn options(kind: AxisDeformKind) -> AxisDeformOptions {
    AxisDeformOptions {
        kind,
        ..AxisDeformOptions::default()
    }
}

fn bounds(mesh: &Mesh) -> crate::Bounds {
    mesh.bounds().unwrap()
}

#[test]
fn the_standard_settings() {
    let o = AxisDeformOptions::default();
    assert_eq!(o.kind, AxisDeformKind::Bend);
    assert_eq!(o.axis, DeformAxis::Z);
    assert_eq!(o.angle, 45.0);
    assert_eq!(o.amount, -0.5);
    assert_eq!(o.limits, [0.0, 1.0]);
    assert_eq!(o.centre, Vec3::ZERO);
    assert_eq!(o.clamped(), o);
    for kind in AxisDeformKind::ALL {
        assert!(!kind.name().is_empty());
    }
    for axis in DeformAxis::ALL {
        assert!(!axis.name().is_empty());
    }
}

#[test]
fn settings_are_brought_within_limits() {
    let wild = AxisDeformOptions {
        kind: AxisDeformKind::Twist,
        axis: DeformAxis::X,
        angle: f32::NAN,
        direction: -90.0,
        amount: 1e9,
        curve: f32::NEG_INFINITY,
        limits: [-3.0, f32::NAN],
        centre: Vec3::new(f32::NAN, 1e12, -1e12),
    }
    .clamped();
    assert_eq!(wild.angle, AxisDeformOptions::STANDARD_ANGLE);
    assert_eq!(wild.direction, 270.0);
    assert_eq!(wild.amount, AxisDeformOptions::MAX_AMOUNT);
    assert_eq!(wild.curve, 0.0);
    assert_eq!(wild.limits, [0.0, 1.0]);
    let most = AxisDeformOptions::MAX_CENTRE;
    assert_eq!(wild.centre, Vec3::new(0.0, most, -most));
    let turned = AxisDeformOptions {
        direction: 725.0,
        angle: -1e6,
        ..AxisDeformOptions::default()
    }
    .clamped();
    assert_eq!(turned.direction, 5.0);
    assert_eq!(turned.angle, -AxisDeformOptions::MAX_ANGLE);
    let tiny = AxisDeformOptions {
        direction: -1e-9,
        ..AxisDeformOptions::default()
    }
    .clamped();
    assert!((0.0..360.0).contains(&tiny.direction));
}

#[test]
fn nothing_moves_with_nothing_to_do() {
    let mesh = pole(100.0);
    for kind in AxisDeformKind::ALL {
        let o = AxisDeformOptions {
            kind,
            angle: 0.0,
            amount: 0.0,
            curve: 0.0,
            ..AxisDeformOptions::default()
        };
        assert_eq!(axis_deform(&mesh, &o, Vec3::ZERO).unwrap(), mesh);
        // Limits with no length change nothing either.
        let flat = AxisDeformOptions {
            limits: [0.4, 0.4],
            ..options(kind)
        };
        assert_eq!(axis_deform(&mesh, &flat, Vec3::ZERO).unwrap(), mesh);
    }
}

#[test]
fn a_bend_curls_a_pole_into_an_arc_from_its_foot() {
    // 90° over a pole 100 cm tall: a quarter circle of radius 200 / π.
    let mesh = pole(100.0);
    let o = AxisDeformOptions {
        angle: 90.0,
        ..AxisDeformOptions::default()
    };
    let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
    assert!(out.same_structure(&mesh));
    let radius = 200.0 / PI;
    for (id, p) in mesh.vertices() {
        let q = out.position(id).unwrap();
        // The foot stays; every point keeps its distance from the bend's
        // middle, less its place across (X) the pole.
        if p.z == 0.0 {
            assert!(near(q, p), "{p} {q}");
        }
        let from_middle = Vec2::new(q.x - radius, q.z).length();
        assert!((from_middle - (radius - p.x)).abs() < 0.05, "{p} {q}");
        assert!((q.y - p.y).abs() < 1e-4);
    }
    // The top lies on its side at X = radius, its face across Z = radius.
    let b = bounds(&out);
    assert!((b.max.z - (radius + 5.0)).abs() < 0.5, "{b:?}");
    assert!((b.max.x - radius).abs() < 0.5, "{b:?}");
}

#[test]
fn a_bend_s_direction_turns_it_round_the_axis() {
    let mesh = pole(100.0);
    let towards = |direction: f32| {
        let o = AxisDeformOptions {
            angle: 90.0,
            direction,
            ..AxisDeformOptions::default()
        };
        let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
        let b = bounds(&out);
        Vec2::new(b.max.x + b.min.x, b.max.y + b.min.y)
    };
    let x = towards(0.0);
    assert!(x.x > 50.0 && x.y.abs() < 1.0, "{x}");
    let y = towards(90.0);
    assert!(y.y > 50.0 && y.x.abs() < 1.0, "{y}");
    let back = towards(180.0);
    assert!(back.x < -50.0, "{back}");
    // Below 0, the other way.
    let o = AxisDeformOptions {
        angle: -90.0,
        ..AxisDeformOptions::default()
    };
    let b = bounds(&axis_deform(&mesh, &o, Vec3::ZERO).unwrap());
    assert!(b.min.x < -50.0);
}

#[test]
fn each_axis_bends_towards_the_next() {
    for (axis, along, towards) in [
        (DeformAxis::X, Vec3::X, Vec3::Y),
        (DeformAxis::Y, Vec3::Y, Vec3::Z),
        (DeformAxis::Z, Vec3::Z, Vec3::X),
    ] {
        // A thin row of points along the axis, 0 to 100.
        let points: Vec<Option<Vec3>> = (0..=10).map(|i| Some(along * i as f32 * 10.0)).collect();
        let mesh = Mesh::from_slots(points, Vec::new(), []).unwrap();
        let o = AxisDeformOptions {
            axis,
            angle: 90.0,
            ..AxisDeformOptions::default()
        };
        let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
        let end = out.position(VertexId::from_raw(10)).unwrap();
        let radius = 200.0 / PI;
        assert!(
            near(end, along * radius + towards * radius),
            "{axis:?} {end}"
        );
    }
}

#[test]
fn the_centre_stays_and_both_sides_bend_away() {
    // A pole from -50 to 50 with its pivot in its middle bends both ways,
    // like a C; its middle stays.
    let mut mesh = pole(100.0);
    let points: Vec<(VertexId, Vec3)> = mesh.vertices().collect();
    for (id, p) in points {
        mesh.set_position(id, p - Vec3::Z * 50.0);
    }
    let o = AxisDeformOptions {
        angle: 90.0,
        ..AxisDeformOptions::default()
    };
    let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
    for (id, p) in mesh.vertices() {
        let q = out.position(id).unwrap();
        if p.z.abs() < 1e-3 {
            assert!(near(p, q));
        } else if p.x.abs() < 1e-3 {
            assert!(q.x > 0.0, "both ends bend towards X: {p} {q}");
        }
    }
    // Moving the centre to the top bends from there instead: the top
    // stays.
    let o = AxisDeformOptions {
        centre: Vec3::Z * 50.0,
        ..o
    };
    let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
    for (id, p) in mesh.vertices().filter(|(_, p)| p.z > 49.9) {
        assert!(near(out.position(id).unwrap(), p));
    }
    // So does moving the pivot there.
    let pivoted = axis_deform(
        &mesh,
        &AxisDeformOptions {
            centre: Vec3::ZERO,
            ..o
        },
        Vec3::Z * 50.0,
    );
    assert_eq!(pivoted.unwrap(), out);
}

#[test]
fn past_the_limits_the_shape_carries_on_rigidly() {
    // Bend only the middle fifth: the lower part stays, the upper part
    // goes on straight, turned 90°, with no tear.
    let mesh = pole(100.0);
    let o = AxisDeformOptions {
        angle: 90.0,
        limits: [0.6, 0.4],
        ..AxisDeformOptions::default()
    };
    let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
    for (id, p) in mesh.vertices() {
        let q = out.position(id).unwrap();
        if p.z <= 40.0 {
            assert!(near(p, q), "below the limits nothing moves: {p} {q}");
        }
        if p.z >= 60.0 {
            // Lying along X now, as far beyond the bend as it was above it.
            let radius = 40.0 / PI;
            let want = Vec3::new(radius + (p.z - 60.0), p.y, 40.0 + radius - p.x);
            assert!(near(q, want), "{p} {q} {want}");
        }
    }
    assert!(out.vertices().all(|(_, p)| p.is_finite()));
}

#[test]
fn a_twist_turns_slices_round_the_axis() {
    let mesh = pole(100.0);
    let o = AxisDeformOptions {
        angle: 180.0,
        ..options(AxisDeformKind::Twist)
    };
    let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
    assert!(out.same_structure(&mesh));
    for (id, p) in mesh.vertices() {
        let q = out.position(id).unwrap();
        let turn = PI * p.z / 100.0;
        let (sin, cos) = turn.sin_cos();
        let want = Vec3::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos, p.z);
        assert!(near(q, want), "{p} {q}");
    }
    // Limits: only the stretch between turns; above, the rest is turned
    // as its top is.
    let o = AxisDeformOptions {
        limits: [0.0, 0.5],
        ..o
    };
    let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
    for (id, p) in mesh.vertices().filter(|(_, p)| p.z >= 50.0) {
        let q = out.position(id).unwrap();
        assert!(near(q, Vec3::new(-p.x, -p.y, p.z)), "{p} {q}");
    }
}

#[test]
fn a_taper_narrows_towards_the_top_and_curve_bulges_it() {
    let mesh = pole(100.0);
    let o = options(AxisDeformKind::Taper);
    let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
    for (id, p) in mesh.vertices() {
        let q = out.position(id).unwrap();
        let scale = 1.0 - 0.5 * p.z / 100.0;
        assert!(near(q, Vec3::new(p.x * scale, p.y * scale, p.z)), "{p} {q}");
    }
    // Curve 1 with no amount: half as wide again in the middle, the ends
    // as they were.
    let o = AxisDeformOptions {
        amount: 0.0,
        curve: 1.0,
        ..o
    };
    let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
    for (id, p) in mesh.vertices() {
        let q = out.position(id).unwrap();
        let t = p.z / 100.0;
        let scale = 1.0 + 4.0 * t * (1.0 - t);
        assert!(near(q, Vec3::new(p.x * scale, p.y * scale, p.z)), "{p} {q}");
    }
    // Never thinner than nothing: past -1 the top is a point.
    let o = AxisDeformOptions {
        amount: -3.0,
        curve: 0.0,
        ..o
    };
    let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
    for (id, p) in mesh.vertices().filter(|(_, p)| p.z >= 40.0) {
        let q = out.position(id).unwrap();
        assert!(q.x.abs() < 1e-4 && q.y.abs() < 1e-4, "{p} {q}");
    }
}

#[test]
fn huge_and_tiny_angles_stay_real() {
    let mesh = pole(100.0);
    for angle in [1e-9, 1e-5, 0.01, 359.9, 360.0, 3_600.0, -3_600.0, 1e30] {
        for kind in [AxisDeformKind::Bend, AxisDeformKind::Twist] {
            let o = AxisDeformOptions {
                kind,
                angle,
                ..AxisDeformOptions::default()
            };
            let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
            assert!(out.vertices().all(|(_, p)| p.is_finite()), "{angle}");
        }
    }
    // A tiny bend barely moves anything.
    let o = AxisDeformOptions {
        angle: 1e-5,
        ..AxisDeformOptions::default()
    };
    let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
    for (id, p) in mesh.vertices() {
        assert!(near(out.position(id).unwrap(), p));
    }
}

#[test]
fn shapes_of_every_sort_keep_their_polygons() {
    let mut shapes: Vec<Mesh> = [
        PrimitiveKind::Cube,
        PrimitiveKind::Cylinder,
        PrimitiveKind::Plane,
        PrimitiveKind::Sphere,
    ]
    .into_iter()
    .map(|k| k.standard().mesh())
    .collect();
    // An n-gon with a zero-length edge, and a fin of three polygons on one
    // edge (not manifold); an open border on all of them.
    let mut odd = Mesh::new();
    let v: Vec<VertexId> = [
        Vec3::ZERO,
        Vec3::X * 50.0,
        Vec3::X * 50.0,
        Vec3::new(50.0, 50.0, 0.0),
        Vec3::new(0.0, 50.0, 0.0),
        Vec3::Z * 50.0,
        Vec3::new(0.0, -50.0, 0.0),
    ]
    .into_iter()
    .map(|p| odd.add_vertex(p).unwrap())
    .collect();
    odd.add_face(&[v[0], v[1], v[2], v[3], v[4]]).unwrap();
    odd.add_face(&[v[0], v[4], v[5]]).unwrap();
    odd.add_face(&[v[4], v[0], v[6]]).unwrap();
    shapes.push(odd);
    shapes.push(Mesh::default());
    // A single loose point: no length along any axis.
    shapes.push(Mesh::from_slots(vec![Some(Vec3::ONE)], Vec::new(), []).unwrap());
    for shape in shapes {
        for kind in AxisDeformKind::ALL {
            for axis in DeformAxis::ALL {
                let o = AxisDeformOptions {
                    kind,
                    axis,
                    angle: 120.0,
                    ..AxisDeformOptions::default()
                };
                let out = axis_deform(&shape, &o, Vec3::ZERO).unwrap();
                assert!(out.same_structure(&shape));
                assert!(out.vertices().all(|(_, p)| p.is_finite()));
                assert_eq!(
                    out.attributes().collect::<Vec<_>>(),
                    shape.attributes().collect::<Vec<_>>()
                );
            }
        }
    }
}

#[test]
fn the_frame_turns_and_moves_only() {
    for axis in DeformAxis::ALL {
        for direction in [0.0, 33.0, 270.0] {
            let o = AxisDeformOptions {
                axis,
                direction,
                centre: Vec3::new(1.0, 2.0, 3.0),
                ..AxisDeformOptions::default()
            };
            let f = frame(&o, Vec3::new(10.0, 0.0, 0.0));
            assert!((f.determinant() - 1.0).abs() < 1e-5);
            assert!(near(
                f.transform_point3(Vec3::ZERO),
                Vec3::new(11.0, 2.0, 3.0)
            ));
        }
    }
    // A pivot that isn't a number counts as the origin.
    let f = frame(&AxisDeformOptions::default(), Vec3::NAN);
    assert!(f.is_finite());
}

#[test]
fn limits_cover_the_shape_along_the_axis() {
    let mesh = pole(100.0);
    let o = AxisDeformOptions {
        limits: [0.25, 0.75],
        ..AxisDeformOptions::default()
    };
    let (whole, range) = limits(&mesh, &o, Vec3::Z * 10.0).unwrap();
    assert!((whole.from() + 10.0).abs() < 1e-4 && (whole.to() - 90.0).abs() < 1e-4);
    assert!((range.from() - 15.0).abs() < 1e-4 && (range.to() - 65.0).abs() < 1e-4);
    assert_eq!(limits(&Mesh::default(), &o, Vec3::ZERO), None);
}

#[test]
fn the_helper_follows_the_bend() {
    let mesh = pole(100.0);
    let o = AxisDeformOptions {
        angle: 90.0,
        ..AxisDeformOptions::default()
    };
    let h = helper(&mesh, &o, Vec3::ZERO).unwrap();
    assert_eq!(h.axis.len(), 33);
    assert!(near(h.axis[0], Vec3::ZERO));
    let radius = 200.0 / PI;
    assert!(near(
        *h.axis.last().unwrap(),
        Vec3::new(radius, 0.0, radius)
    ));
    assert_eq!(h.limits.len(), 4);
    assert_eq!(h.centre, Vec3::ZERO);
    // A taper's crosses shrink with it.
    let t = helper(&mesh, &options(AxisDeformKind::Taper), Vec3::ZERO).unwrap();
    let width = |[a, b]: [Vec3; 2]| a.distance(b);
    assert!((width(t.limits[2]) - width(t.limits[0]) * 0.5).abs() < 1e-3);
    assert_eq!(helper(&Mesh::default(), &o, Vec3::ZERO), None);
}

#[test]
fn ten_thousand_settings_never_give_a_place_that_isn_t_a_number() {
    let mesh = pole(60.0);
    let mut seed = 2024u32;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    for round in 0..10_000 {
        let wild = |x: f32| match round % 7 {
            0 => f32::NAN,
            1 => x * 1e12,
            _ => x,
        };
        let o = AxisDeformOptions {
            kind: AxisDeformKind::ALL[round % 3],
            axis: DeformAxis::ALL[(round / 3) % 3],
            angle: wild(next() * 8_000.0 - 4_000.0),
            direction: wild(next() * 1_000.0 - 500.0),
            amount: wild(next() * 30.0 - 15.0),
            curve: wild(next() * 30.0 - 15.0),
            limits: [wild(next() * 1.4 - 0.2), wild(next() * 1.4 - 0.2)],
            centre: Vec3::new(next(), next(), next()) * 400.0 - 200.0,
        };
        let range = AxisRange::new(-30.0, 30.0).unwrap();
        if let Some(d) = AxisDeformer::new(&o, range) {
            let p = Vec3::new(next(), next(), next()) * 200.0 - 100.0;
            assert!(d.moved(p).is_finite(), "{o:?} {p}");
        }
        if round % 100 == 0 {
            let out = axis_deform(&mesh, &o, Vec3::ZERO).unwrap();
            assert!(out.vertices().all(|(_, p)| p.is_finite()));
        }
    }
}

#[test]
fn the_same_settings_always_give_the_same_shape() {
    let mesh = PrimitiveKind::Sphere.standard().mesh();
    let o = AxisDeformOptions {
        kind: AxisDeformKind::Twist,
        angle: 77.0,
        limits: [0.1, 0.9],
        ..AxisDeformOptions::default()
    };
    let a = axis_deform(&mesh, &o, Vec3::ONE).unwrap();
    let b = axis_deform(&mesh, &o, Vec3::ONE).unwrap();
    assert_eq!(a, b);
    assert!(work(&mesh) == mesh.vertex_count() / 20);
}

#[test]
#[ignore = "timing: run with --release -- --ignored --nocapture"]
fn timing() {
    use crate::primitives::SphereParams;
    let sphere = Primitive::Sphere(SphereParams {
        radius: 90.0,
        sides: 1024,
        rings: 512,
    })
    .mesh();
    for kind in AxisDeformKind::ALL {
        let start = std::time::Instant::now();
        let out = axis_deform(&sphere, &options(kind), Vec3::ZERO).unwrap();
        println!(
            "{kind:?}: {} points in {:.1} ms",
            out.vertex_count(),
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
}
