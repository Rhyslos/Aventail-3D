//! Tests for remeshing for sculpting (decision 0312).

use super::*;
use crate::{MeshReport, Primitive, PrimitiveKind};
use glam::Mat4;

fn options(voxel_size: f32) -> SculptRemeshOptions {
    SculptRemeshOptions {
        voxel_size,
        ..Default::default()
    }
}

fn run(mesh: &Mesh, options: &SculptRemeshOptions) -> Result<Remeshed, RemeshProblem> {
    remesh_for_sculpting(mesh, options, None, &Progress::default())
}

fn assert_clean_solid(mesh: &Mesh) {
    let report = MeshReport::new(mesh);
    assert!(report.faces > 0);
    assert_eq!(report.border_edges, 0, "{report:?}");
    assert_eq!(report.non_manifold_edges, 0, "{report:?}");
    assert_eq!(report.flipped_edges, 0, "{report:?}");
    assert_eq!(report.loose_points, 0, "{report:?}");
    assert!(mesh.faces().all(|(_, c)| c.len() == 4), "quads only");
}

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

/// A block-out: a ball, a cylinder through it and a box overlapping it.
fn block_out() -> Mesh {
    let ball = PrimitiveKind::Sphere.standard().mesh();
    let cylinder = PrimitiveKind::Cylinder.standard().mesh();
    crate::parts::combine(&[
        (&ball, Mat4::IDENTITY),
        (
            &cylinder,
            Mat4::from_translation(Vec3::new(30.0, 0.0, 10.0)),
        ),
        (&cube(), Mat4::from_translation(Vec3::new(-40.0, 10.0, 0.0))),
    ])
}

#[test]
fn overlapping_primitives_become_one_closed_surface() {
    let out = run(&block_out(), &options(3.0)).unwrap();
    assert_clean_solid(&out.mesh);
    assert_eq!(MeshReport::new(&out.mesh).pieces, 1, "one surface");
    assert!(out.labels.is_empty());
}

#[test]
fn it_matches_the_remesh_modifier_without_keeping_detail() {
    // The same grid, field and surface: the same polygons (in another
    // order, block by block).
    let sphere = PrimitiveKind::Sphere.standard().mesh();
    let modifier = remesh(
        &sphere,
        &RemeshOptions {
            voxel_size: 4.0,
            smooth: 0,
        },
    )
    .unwrap();
    let sculpt = run(
        &sphere,
        &SculptRemeshOptions {
            voxel_size: 4.0,
            keep_detail: false,
            fix_holes: false,
            smooth: 0,
        },
    )
    .unwrap();
    assert_eq!(modifier.face_count(), sculpt.mesh.face_count());
    assert_eq!(modifier.vertex_count(), sculpt.mesh.vertex_count());
    let mut a: Vec<[i64; 3]> = modifier
        .vertices()
        .map(|(_, p)| (p * 1000.0).round().as_i64vec3().to_array())
        .collect();
    let mut b: Vec<[i64; 3]> = sculpt
        .mesh
        .vertices()
        .map(|(_, p)| (p * 1000.0).round().as_i64vec3().to_array())
        .collect();
    a.sort_unstable();
    b.sort_unstable();
    assert_eq!(a, b);
}

#[test]
fn the_polygon_count_is_worked_out_before_it_starts() {
    // One piece: close.
    let ball = PrimitiveKind::Sphere.standard().mesh();
    for voxel in [2.0, 4.0] {
        let made = run(&ball, &options(voxel)).unwrap().mesh.face_count() as f64;
        let guessed = polygons_from(projected_area(&ball), voxel) as f64;
        assert!((guessed - made).abs() < made * 0.15, "{guessed} vs {made}");
    }
    // Overlapping pieces: what is inside goes, so the guess is high,
    // never low.
    let shape = block_out();
    let made = run(&shape, &options(3.0)).unwrap().mesh.face_count() as f64;
    let guessed = polygons_from(projected_area(&shape), 3.0) as f64;
    assert!(guessed >= made * 0.95, "{guessed} vs {made}");
}

#[test]
fn a_tiny_voxel_size_is_refused_with_the_polygons_it_would_make() {
    let ball = PrimitiveKind::Sphere.standard().mesh();
    let Err(RemeshProblem::TooManyPolygons { polygons, smallest }) = run(&ball, &options(0.01))
    else {
        panic!("not refused");
    };
    assert!(polygons > MAX_SCULPT_POLYGONS, "{polygons}");
    let area = projected_area(&ball);
    assert!(polygons_from(area, smallest) <= MAX_SCULPT_POLYGONS);
    assert!(polygons_from(area, smallest - 0.02) > MAX_SCULPT_POLYGONS);
    let reason = RemeshProblem::TooManyPolygons { polygons, smallest }.to_string();
    assert!(reason.contains("million polygons"), "{reason}");
    assert!(reason.contains(&format!("{smallest:.2} cm")), "{reason}");
}

#[test]
fn pieces_far_apart_for_tiny_voxels_are_refused_not_run_out_of_memory() {
    // Two small cubes a kilometre apart: little surface, a huge box.
    let small = Primitive::Cube(Default::default()).mesh();
    let apart = crate::parts::combine(&[
        (&small, Mat4::from_scale(Vec3::splat(0.01))),
        (
            &small,
            Mat4::from_scale_rotation_translation(
                Vec3::splat(0.01),
                glam::Quat::IDENTITY,
                Vec3::splat(100_000.0),
            ),
        ),
    ]);
    assert_eq!(
        run(&apart, &options(0.05)).err(),
        Some(RemeshProblem::TooSpread)
    );
    assert!(!RemeshProblem::TooSpread.to_string().is_empty());
}

#[test]
fn an_open_shape_is_closed_first() {
    let mut open = cube();
    let first = open.faces().next().unwrap().0;
    open.delete_faces(&[first]);
    let out = run(&open, &options(5.0)).unwrap();
    assert_clean_solid(&out.mesh);
    // Still a whole box, not a shell.
    let b = out.mesh.bounds().unwrap();
    assert!(b.size().min_element() > 90.0, "{b:?}");
    // Without Fix holes it is refused, as the modifier refuses it.
    let refused = run(
        &open,
        &SculptRemeshOptions {
            fix_holes: false,
            ..options(5.0)
        },
    );
    assert_eq!(refused.err(), Some(RemeshProblem::NotClosed));
    // A flat plane closes into a thin slab, thinner than the voxels:
    // nothing is left, said plainly.
    let plane = PrimitiveKind::Plane.standard().mesh();
    assert_eq!(
        run(&plane, &options(5.0)).err(),
        Some(RemeshProblem::NothingLeft)
    );
}

#[test]
fn pieces_meeting_at_the_same_places_count_as_closed() {
    // A cube whose sides don't share points, but meet where they are.
    let cube = cube();
    let mut apart = Mesh::new();
    for (_, corners) in cube.faces() {
        let ids: Vec<_> = corners
            .iter()
            .map(|v| apart.add_vertex(cube.position(*v).unwrap()).unwrap())
            .collect();
        apart.add_face(&ids).unwrap();
    }
    assert!(holes::open_edges(&apart).is_empty());
    assert!(holes::open_edges(&cube).is_empty());
    let mut open = apart.clone();
    let first = open.faces().next().unwrap().0;
    open.delete_faces(&[first]);
    assert_eq!(holes::open_edges(&open).len(), 4);
}

#[test]
fn keep_detail_keeps_a_sculpted_bump_within_a_voxel() {
    // A ball of fine polygons with a sharp bump on top.
    let ball = crate::samples::heavy_ball(40_000, 50.0);
    let mut bumped = ball.clone();
    let top = Vec3::new(0.0, 0.0, 100.0);
    let ids: Vec<_> = bumped.vertices().map(|(v, _)| v).collect();
    for v in ids {
        let p = bumped.position(v).unwrap();
        let d = p.distance(top);
        if d < 15.0 {
            let lift = 8.0 * (1.0 - d / 15.0).powi(2);
            bumped.set_position(v, p + (p - Vec3::new(0.0, 0.0, 50.0)).normalize() * lift);
        }
    }
    let voxel = 2.0;
    let peak = |m: &Mesh| m.vertices().map(|(_, p)| p.z).fold(f32::MIN, f32::max);
    let kept = run(&bumped, &options(voxel)).unwrap().mesh;
    let smoothed = run(
        &bumped,
        &SculptRemeshOptions {
            keep_detail: false,
            smooth: 4,
            ..options(voxel)
        },
    )
    .unwrap()
    .mesh;
    let was = peak(&bumped);
    assert!(
        (was - peak(&kept)).abs() < voxel,
        "{was} vs {}",
        peak(&kept)
    );
    assert!(
        peak(&kept) > peak(&smoothed),
        "keeping detail keeps the peak"
    );
    // Every point lies on the old surface, near enough.
    let tree = crate::bvh::TriangleTree::of_mesh(&bumped, Mat4::IDENTITY);
    for (_, p) in kept.vertices() {
        assert!(tree.nearest(p).unwrap().distance < voxel, "{p}");
    }
    assert_clean_solid(&kept);
}

#[test]
fn labels_come_from_the_nearest_polygon() {
    // Label 7 on the cube's top, 0 elsewhere.
    let cube = cube();
    let b = cube.bounds().unwrap();
    let (top, middle) = (b.max.z, b.centre());
    let mut labels = vec![0u32; cube.face_slots().len()];
    for (f, corners) in cube.faces() {
        if corners
            .iter()
            .all(|v| cube.position(*v).unwrap().z > top - 0.01)
        {
            labels[f.get() as usize] = 7;
        }
    }
    let out =
        remesh_for_sculpting(&cube, &options(5.0), Some(&labels), &Progress::default()).unwrap();
    assert_eq!(out.labels.len(), out.mesh.face_count());
    let mut on_top = 0;
    for ((_, corners), label) in out.mesh.faces().zip(&out.labels) {
        let at = corners
            .iter()
            .map(|v| out.mesh.position(*v).unwrap())
            .sum::<Vec3>()
            / 4.0;
        let inner = (at - middle).x.abs() < 40.0 && (at - middle).y.abs() < 40.0;
        if at.z > top - 2.0 && inner {
            assert_eq!(*label, 7, "{at}");
            on_top += 1;
        } else if at.z < top - 10.0 {
            assert_eq!(*label, 0, "{at}");
        }
    }
    assert!(on_top > 100);
}

#[test]
fn it_tells_its_progress_and_stops_when_asked() {
    let progress = Progress::default();
    remesh_for_sculpting(&block_out(), &options(4.0), None, &progress).unwrap();
    assert_eq!(progress.share(), 1.0);
    let stopped = Progress::default();
    stopped.stop();
    assert_eq!(
        remesh_for_sculpting(&block_out(), &options(4.0), None, &stopped).err(),
        Some(RemeshProblem::Cancelled)
    );
}

#[test]
fn the_same_shape_gives_the_same_result() {
    let a = run(&block_out(), &options(3.0)).unwrap().mesh;
    let b = run(&block_out(), &options(3.0)).unwrap().mesh;
    assert_eq!(a, b);
}

#[test]
fn odd_voxel_sizes_never_panic() {
    let shape = block_out();
    for voxel in [f32::NAN, f32::INFINITY, -3.0, 0.0, 1e-9, 1e9, 500.0] {
        match run(&shape, &options(voxel)) {
            Ok(out) => assert_clean_solid(&out.mesh),
            Err(why) => assert!(!why.to_string().is_empty()),
        }
    }
    assert_eq!(
        run(&Mesh::new(), &options(2.0)).err(),
        Some(RemeshProblem::Empty)
    );
}

#[test]
fn voxel_sizes_for_a_polygon_count() {
    assert_eq!(polygons_from(10_000.0, 1.0), 10_000);
    assert_eq!(polygons_from(10_000.0, 10.0), 100);
    let v = voxel_for(1.0e6, 10_000);
    assert!(polygons_from(1.0e6, v) <= 10_000);
    assert!((v - 10.0).abs() < 0.02, "{v}");
}

#[test]
#[ignore = "benchmark: run with --release -- --ignored --nocapture"]
fn sculpt_remesh_benchmark() {
    // The heavy test shape (3 million polygons, 2 m tall), remeshed to
    // about 1 and 3 million polygons.
    let ball = crate::samples::heavy_ball(3_000_000, 100.0);
    let area = projected_area(&ball);
    for target in [1_000_000, 3_000_000] {
        let voxel = voxel_for(area, target);
        let start = std::time::Instant::now();
        let out = run(&ball, &options(voxel)).unwrap();
        println!(
            "heavy ball to about {target} polygons (voxel {voxel} cm): {} polygons in {:?}",
            out.mesh.face_count(),
            start.elapsed()
        );
    }
    let blocks = block_out();
    let start = std::time::Instant::now();
    let out = run(&blocks, &options(0.25)).unwrap();
    println!(
        "block-out at 0.25 cm: {} polygons in {:?}",
        out.mesh.face_count(),
        start.elapsed()
    );
}
