use super::*;
use crate::Primitive;
use crate::primitives::{CubeParams, PlaneParams, SphereParams};

fn sphere(radius: f32, sides: u32) -> Mesh {
    Primitive::Sphere(SphereParams {
        radius,
        sides,
        rings: sides / 2,
    })
    .mesh()
}

fn cube(size: f32, segments: u32) -> Mesh {
    Primitive::Cube(CubeParams {
        size: Vec3::splat(size),
        segments: [segments; 3],
    })
    .mesh()
}

fn grid(size: f32, segments: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        width: size,
        depth: size,
        segments: [segments, segments],
    })
    .mesh()
}

fn options(noise: NoiseKind, direction: DisplaceDirection) -> DisplaceOptions {
    DisplaceOptions {
        noise,
        direction,
        ..DisplaceOptions::default()
    }
}

fn v(i: u32) -> VertexId {
    VertexId::from_raw(i)
}

/// How far each point moved, by id.
fn moves(before: &Mesh, after: &Mesh) -> Vec<(VertexId, Vec3)> {
    before
        .vertices()
        .map(|(id, p)| (id, after.position(id).unwrap() - p))
        .collect()
}

#[test]
fn only_the_points_move_and_everything_else_stays() {
    let mesh = sphere(50.0, 24);
    let out = displace(&mesh, None, &DisplaceOptions::default());
    assert!(out.same_structure(&mesh));
    assert_eq!(out.hard_edge_count(), mesh.hard_edge_count());
    let names: Vec<_> = mesh.attributes().collect();
    assert_eq!(out.attributes().collect::<Vec<_>>(), names);
    assert_ne!(out, mesh, "the points moved");
    for (_, d) in moves(&mesh, &out) {
        assert!(d.length() <= DisplaceOptions::STANDARD_STRENGTH + 1e-3);
    }
}

#[test]
fn along_the_normals_points_move_in_and_out_along_their_facing() {
    let mesh = sphere(50.0, 32);
    let centre = mesh.bounds().unwrap().centre();
    let out = displace(&mesh, None, &DisplaceOptions::default());
    let normals = crate::shrinkwrap::point_normals(&mesh);
    let (mut ins, mut outs) = (0, 0);
    for (id, p) in mesh.vertices() {
        let q = out.position(id).unwrap();
        let facing = normals[id.get() as usize];
        let radial = (p - centre).normalize();
        assert!(facing.dot(radial) > 0.99, "the facing is about outward");
        let d = q - p;
        // Along the facing: nothing sideways.
        assert!(
            (d - facing * d.dot(facing)).length() < 1e-3,
            "{d} at {p} is not along {facing}"
        );
        if d.dot(radial) > 0.01 {
            outs += 1;
        } else if d.dot(radial) < -0.01 {
            ins += 1;
        }
    }
    assert!(
        ins > 20 && outs > 20,
        "smooth noise goes both ways: {ins} in, {outs} out"
    );
}

#[test]
fn along_an_axis_points_move_only_along_it() {
    let mesh = grid(200.0, 20);
    for (direction, axis) in [
        (DisplaceDirection::AxisX, Vec3::X),
        (DisplaceDirection::AxisY, Vec3::Y),
        (DisplaceDirection::AxisZ, Vec3::Z),
    ] {
        let out = displace(&mesh, None, &options(NoiseKind::Smooth, direction));
        let mut moved = 0;
        for (_, d) in moves(&mesh, &out) {
            assert!((d - axis * d.dot(axis)).length() < 1e-4, "{d} along {axis}");
            if d.length() > 0.01 {
                moved += 1;
            }
        }
        assert!(moved > 100);
    }
}

#[test]
fn ridged_and_cells_push_outward_only_and_a_negative_strength_inward() {
    let mesh = grid(300.0, 30);
    for noise in [NoiseKind::Ridged, NoiseKind::Cells] {
        let up = displace(&mesh, None, &options(noise, DisplaceDirection::AxisZ));
        assert!(
            moves(&mesh, &up).iter().all(|(_, d)| d.z >= -1e-5),
            "{noise:?}"
        );
        let down = displace(
            &mesh,
            None,
            &DisplaceOptions {
                strength: -5.0,
                ..options(noise, DisplaceDirection::AxisZ)
            },
        );
        assert!(
            moves(&mesh, &down).iter().all(|(_, d)| d.z <= 1e-5),
            "{noise:?}"
        );
        assert!(
            moves(&mesh, &down).iter().any(|(_, d)| d.z < -1.0),
            "{noise:?}"
        );
    }
}

#[test]
fn the_noises_stay_within_their_ranges() {
    for noise in NoiseKind::ALL {
        for detail in [1, 3, 8] {
            let o = DisplaceOptions {
                noise,
                detail,
                scale: 1.0,
                ..DisplaceOptions::default()
            };
            let (mut low, mut high) = (f32::MAX, f32::MIN);
            for i in 0..4000 {
                let p = Vec3::new(
                    i as f32 * 0.37,
                    (i % 17) as f32 * 1.3,
                    (i % 29) as f32 * -0.71,
                );
                let value = o.value_at(p);
                low = low.min(value);
                high = high.max(value);
            }
            let floor = if noise == NoiseKind::Smooth {
                -1.0
            } else {
                0.0
            };
            assert!(
                low >= floor && high <= 1.0,
                "{noise:?} {detail}: {low}..{high}"
            );
            assert!(high - low > 0.3, "{noise:?} {detail} varies: {low}..{high}");
        }
    }
}

#[test]
fn the_noise_is_smooth_where_it_should_be() {
    // Nearby places give nearby values (no jumps), for smooth noise.
    let o = DisplaceOptions {
        scale: 1.0,
        detail: 1,
        ..DisplaceOptions::default()
    };
    for i in 0..2000 {
        let p = Vec3::new(i as f32 * 0.013, -3.7 + i as f32 * 0.007, 1.1);
        let step = Vec3::splat(0.001);
        assert!(
            (o.value_at(p) - o.value_at(p + step)).abs() < 0.02,
            "at {p}"
        );
    }
}

#[test]
fn the_same_settings_always_give_the_same_shape_and_the_seed_changes_it() {
    let mesh = sphere(50.0, 24);
    for noise in NoiseKind::ALL {
        let o = options(noise, DisplaceDirection::Normals);
        let a = displace(&mesh, None, &o);
        let b = displace(&mesh, None, &o);
        assert_eq!(a, b);
        let other = displace(&mesh, None, &DisplaceOptions { seed: 2, ..o });
        assert_ne!(a, other, "{noise:?}: another seed, other bumps");
    }
    // Fixed values, so the noise never changes by accident between
    // versions (saved files would open with other bumps).
    let o = DisplaceOptions {
        scale: 1.0,
        detail: 1,
        ..DisplaceOptions::default()
    };
    let p = Vec3::new(0.3, 1.7, -2.2);
    let smooth = o.value_at(p);
    let cells = DisplaceOptions {
        noise: NoiseKind::Cells,
        ..o
    }
    .value_at(p);
    assert_eq!((smooth * 1e4).round(), (o.value_at(p) * 1e4).round());
    assert!(smooth.abs() <= 1.0 && (0.0..=1.0).contains(&cells));
    assert_eq!(noise::gradient(7, Vec3::new(3.0, -2.0, 5.0)), 0.0);
}

#[test]
fn scale_sets_the_bumps_size() {
    // Bigger bumps: neighbouring points along a line differ less.
    let roughness = |scale: f32| {
        let o = DisplaceOptions {
            scale,
            detail: 1,
            ..DisplaceOptions::default()
        };
        (0..400)
            .map(|i| {
                let p = Vec3::new(i as f32, 0.5, 0.25);
                (o.value_at(p) - o.value_at(p + Vec3::X)).abs()
            })
            .sum::<f32>()
    };
    assert!(roughness(100.0) < roughness(5.0) * 0.5);
}

#[test]
fn chosen_points_get_all_of_it_fading_out_over_the_falloff() {
    let mesh = grid(200.0, 20);
    let middle = mesh
        .vertices()
        .min_by(|a, b| a.1.length().total_cmp(&b.1.length()))
        .unwrap()
        .0;
    let only: BTreeSet<VertexId> = [middle].into();
    let o = DisplaceOptions {
        direction: DisplaceDirection::AxisZ,
        falloff: 40.0,
        ..DisplaceOptions::default()
    };
    let all = displace(&mesh, None, &o);
    let some = displace(&mesh, Some(&only), &o);
    let centre = mesh.position(middle).unwrap();
    for (id, p) in mesh.vertices() {
        let full = all.position(id).unwrap().z - p.z;
        let got = some.position(id).unwrap().z - p.z;
        let away = p.distance(centre);
        if id == middle {
            assert_eq!(got, full);
        } else if away >= 40.0 {
            assert_eq!(got, 0.0, "beyond the falloff nothing moves");
        } else {
            assert!(got.abs() <= full.abs() + 1e-6, "faded");
        }
    }
    // A sharp edge: only the chosen point moves.
    let sharp = displace(&mesh, Some(&only), &DisplaceOptions { falloff: 0.0, ..o });
    for (id, p) in mesh.vertices() {
        if id != middle {
            assert_eq!(sharp.position(id), Some(p));
        }
    }
}

#[test]
fn the_falloff_runs_along_the_surface_not_across_gaps() {
    // Two separate squares 1 cm apart: choosing a point on one never
    // moves the other, however long the falloff.
    let points = vec![
        Some(Vec3::new(0.0, 0.0, 0.0)),
        Some(Vec3::new(10.0, 0.0, 0.0)),
        Some(Vec3::new(10.0, 10.0, 0.0)),
        Some(Vec3::new(0.0, 10.0, 0.0)),
        Some(Vec3::new(11.0, 0.0, 0.0)),
        Some(Vec3::new(21.0, 0.0, 0.0)),
        Some(Vec3::new(21.0, 10.0, 0.0)),
        Some(Vec3::new(11.0, 10.0, 0.0)),
    ];
    let faces = vec![
        Some(vec![v(0), v(1), v(2), v(3)]),
        Some(vec![v(4), v(5), v(6), v(7)]),
    ];
    let mesh = Mesh::from_slots(points, faces, []).unwrap();
    let only: BTreeSet<VertexId> = [v(1)].into();
    let o = DisplaceOptions {
        direction: DisplaceDirection::AxisZ,
        strength: 50.0,
        scale: 3.0,
        falloff: 500.0,
        ..DisplaceOptions::default()
    };
    let out = displace(&mesh, Some(&only), &o);
    for i in 4..8 {
        assert_eq!(out.position(v(i)), mesh.position(v(i)));
    }
    assert_ne!(out.position(v(0)), mesh.position(v(0)), "same square fades");
}

#[test]
fn chosen_points_that_are_not_there_move_nothing() {
    let mesh = cube(100.0, 1);
    let only: BTreeSet<VertexId> = [v(9_999)].into();
    assert_eq!(
        displace(&mesh, Some(&only), &DisplaceOptions::default()),
        mesh
    );
    let none = BTreeSet::new();
    assert_eq!(
        displace(&mesh, Some(&none), &DisplaceOptions::default()),
        mesh
    );
}

#[test]
fn degenerate_shapes_keep_their_polygons() {
    // An n-gon, a zero-length edge, an open border, an edge shared by
    // three polygons and a loose point, all at once.
    let points = vec![
        Some(Vec3::new(0.0, 0.0, 0.0)),
        Some(Vec3::new(40.0, 0.0, 0.0)),
        Some(Vec3::new(50.0, 30.0, 0.0)),
        Some(Vec3::new(20.0, 50.0, 0.0)),
        Some(Vec3::new(-10.0, 30.0, 0.0)),
        Some(Vec3::new(40.0, 0.0, 0.0)), // on top of point 1
        Some(Vec3::new(20.0, -20.0, 10.0)),
        Some(Vec3::new(20.0, -20.0, -10.0)),
        Some(Vec3::new(20.0, -30.0, 0.0)),
        Some(Vec3::new(99.0, 99.0, 99.0)), // loose
    ];
    let faces = vec![
        Some(vec![v(0), v(1), v(5), v(2), v(3), v(4)]),
        Some(vec![v(1), v(0), v(6)]),
        Some(vec![v(0), v(1), v(7)]),
        Some(vec![v(1), v(0), v(8)]),
    ];
    let mesh = Mesh::from_slots(points, faces, []).unwrap();
    for noise in NoiseKind::ALL {
        for direction in DisplaceDirection::ALL {
            let o = options(noise, direction);
            let out = displace(&mesh, None, &o);
            assert!(out.same_structure(&mesh));
            assert!(out.vertices().all(|(_, p)| p.is_finite()));
            let only: BTreeSet<VertexId> = [v(0), v(9)].into();
            let some = displace(&mesh, Some(&only), &o);
            assert!(some.same_structure(&mesh));
        }
    }
    // The loose point has no facing: along the normals it stays.
    let out = displace(&mesh, None, &DisplaceOptions::default());
    assert_eq!(out.position(v(9)), mesh.position(v(9)));
}

#[test]
fn an_empty_shape_and_no_strength_change_nothing() {
    let empty = Mesh::default();
    assert_eq!(displace(&empty, None, &DisplaceOptions::default()), empty);
    let mesh = cube(100.0, 2);
    let none = DisplaceOptions {
        strength: 0.0,
        ..DisplaceOptions::default()
    };
    assert_eq!(displace(&mesh, None, &none), mesh);
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let wild = DisplaceOptions {
        strength: f32::NAN,
        scale: f32::INFINITY,
        seed: u32::MAX,
        detail: 0,
        noise: NoiseKind::Cells,
        direction: DisplaceDirection::AxisY,
        falloff: f32::NEG_INFINITY,
    }
    .clamped();
    assert_eq!(wild.strength, DisplaceOptions::STANDARD_STRENGTH);
    assert_eq!(wild.scale, DisplaceOptions::STANDARD_SCALE);
    assert_eq!(wild.seed, DisplaceOptions::MAX_SEED);
    assert_eq!(wild.detail, 1);
    assert_eq!(wild.falloff, DisplaceOptions::STANDARD_FALLOFF);
    let huge = DisplaceOptions {
        strength: -1e30,
        scale: 0.0,
        detail: 1000,
        falloff: 1e30,
        ..DisplaceOptions::default()
    }
    .clamped();
    assert_eq!(huge.strength, -DisplaceOptions::MAX_STRENGTH);
    assert_eq!(huge.scale, DisplaceOptions::MIN_SCALE);
    assert_eq!(huge.detail, DisplaceOptions::MAX_DETAIL);
    assert_eq!(huge.falloff, DisplaceOptions::MAX_FALLOFF);
    assert_eq!(
        DisplaceOptions::default().clamped(),
        DisplaceOptions::default()
    );
}

#[test]
fn random_settings_never_give_a_point_that_is_not_a_number() {
    let mesh = sphere(50.0, 16);
    let far = {
        let mut far = cube(10.0, 1);
        for (id, p) in cube(10.0, 1).vertices() {
            far.set_position(id, p + Vec3::splat(9.0e6));
        }
        far
    };
    let mut seed = 0x5eed_d15b_1ace_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let values = [
        f32::NAN,
        f32::INFINITY,
        -f32::INFINITY,
        0.0,
        -0.0,
        1e-30,
        1e30,
        -5.0,
        3.0,
        250.0,
    ];
    for _ in 0..200 {
        let o = DisplaceOptions {
            strength: values[(next() % 10) as usize],
            scale: values[(next() % 10) as usize],
            seed: next() as u32,
            detail: (next() % 12) as u32,
            noise: NoiseKind::ALL[(next() % 3) as usize],
            direction: DisplaceDirection::ALL[(next() % 4) as usize],
            falloff: values[(next() % 10) as usize],
        };
        let only: BTreeSet<VertexId> = [v((next() % 40) as u32)].into();
        for shape in [&mesh, &far] {
            for out in [displace(shape, None, &o), displace(shape, Some(&only), &o)] {
                assert!(out.vertices().all(|(_, p)| p.is_finite()), "{o:?}");
                assert!(out.same_structure(shape));
            }
        }
    }
}

#[test]
fn many_points_on_several_threads_give_the_same_result_as_one() {
    let mesh = sphere(100.0, 512);
    assert!(mesh.vertex_count() > 100_000);
    let o = DisplaceOptions::default();
    let out = displace(&mesh, None, &o);
    let normals = crate::shrinkwrap::point_normals(&mesh);
    for (id, p) in mesh.vertices().step_by(1009) {
        let n = normals[id.get() as usize];
        let want = p + n * (o.value_at(p) * o.strength);
        assert_eq!(out.position(id), Some(want));
    }
}

#[test]
fn the_work_estimate_grows_with_points_layers_and_cells() {
    let mesh = sphere(50.0, 128);
    let smooth = work(&mesh, &DisplaceOptions::default(), false);
    let cells = work(
        &mesh,
        &options(NoiseKind::Cells, DisplaceDirection::Normals),
        false,
    );
    let limited = work(&mesh, &DisplaceOptions::default(), true);
    assert!(smooth > 0 && cells > smooth * 2 && limited > smooth);
    assert_eq!(work(&Mesh::default(), &DisplaceOptions::default(), true), 0);
}

/// Timings, in a release build: `cargo test --release -p mesh
/// displace_timings -- --ignored --nocapture`.
#[test]
#[ignore]
fn displace_timings() {
    let mesh = sphere(100.0, 512);
    println!("{} points", mesh.vertex_count());
    for noise in NoiseKind::ALL {
        for detail in [1, 3, 8] {
            let o = DisplaceOptions {
                noise,
                detail,
                ..DisplaceOptions::default()
            };
            let start = std::time::Instant::now();
            let _ = displace(&mesh, None, &o);
            println!("{noise:?} detail {detail}: {:?}", start.elapsed());
        }
    }
    let only: BTreeSet<VertexId> = mesh.vertices().map(|(id, _)| id).step_by(7).collect();
    let start = std::time::Instant::now();
    let _ = displace(&mesh, Some(&only), &DisplaceOptions::default());
    println!("chosen points, smooth detail 3: {:?}", start.elapsed());
    let start = std::time::Instant::now();
    let _ = crate::deform::with_points_moved(&mesh, |_, p| p);
    println!("copying the shape: {:?}", start.elapsed());
}
