//! Tests for the Live bevel modifier's work (decision 0149).

use super::*;
use crate::bevel_weight::set_bevel_weights;
use crate::{MeshReport, Primitive, VertexId};
use glam::Vec3;

/// A cube 100 cm across (its 12 edges marked hard, with UVs and seams, as
/// the primitive makes it), standing on the ground.
fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

/// A mesh of the given points and polygons.
fn mesh_of(points: &[[f32; 3]], faces: &[&[u32]]) -> Mesh {
    let mut mesh = Mesh::new();
    let ids: Vec<VertexId> = points
        .iter()
        .map(|p| mesh.add_vertex(Vec3::from(*p)).unwrap())
        .collect();
    for face in faces {
        let corners: Vec<VertexId> = face.iter().map(|i| ids[*i as usize]).collect();
        mesh.add_face(&corners).unwrap();
    }
    mesh
}

/// A polygon's area.
fn area(mesh: &Mesh, face: crate::FaceId) -> f32 {
    let corners = mesh.face(face).unwrap();
    let mut sum = Vec3::ZERO;
    for (a, b) in crate::polygon::sides(corners) {
        sum += mesh.position(a).unwrap().cross(mesh.position(b).unwrap());
    }
    sum.length() / 2.0
}

fn clean_solid(mesh: &Mesh) {
    let report = MeshReport::new(mesh);
    assert!(report.is_closed(), "{report:?}");
    assert!(!report.has_problems(), "{report:?}");
    assert!(mesh.vertices().all(|(_, p)| p.is_finite()));
}

fn standard() -> LiveBevelOptions {
    LiveBevelOptions::default()
}

const BY_ANGLE: Hardness = Hardness::ByAngle(30.0);

#[test]
fn the_standard_settings() {
    let o = standard();
    assert_eq!(o.edges, BevelEdges::ByAngle);
    assert_eq!(
        (o.angle, o.width, o.segments, o.profile),
        (30.0, 2.0, 2, 1.0)
    );
    assert!(o.clamp);
    assert!(
        !o.soften,
        "Soft bevels is off unless the game bevel turns it on"
    );
    let shape = o.shape();
    assert!(shape.options.improved && shape.options.even && shape.options.weld);
    assert_eq!(shape.options.weld_distance, WELD_DISTANCE);
}

#[test]
fn a_cube_gets_every_edge_bevelled_with_round_corners() {
    let out = live_bevel(&cube(), &standard(), BY_ANGLE).unwrap();
    clean_solid(&out);
    // 6 faces, 12 strips of 2 rows, 8 corners of 3 quads (decision 0141).
    assert_eq!(out.face_count(), 6 + 12 * 2 + 8 * 3);
    // Still 100 cm across: a bevel only cuts corners off.
    let b = crate::Bounds::around(out.vertices().map(|(_, p)| p)).unwrap();
    assert!((b.size() - Vec3::splat(100.0)).abs().max_element() < 1e-3);
    // The big faces are 2 cm smaller on each side.
    let top = out
        .faces()
        .map(|(f, _)| f)
        .find(|f| out.face_normal(*f).z > 0.999 && out.face(*f).unwrap().len() == 4)
        .map(|f| area(&out, f));
    assert!((top.unwrap() - 96.0 * 96.0).abs() < 1.0, "{top:?}");
}

#[test]
fn gentle_edges_are_left_alone_by_angle() {
    // A cylinder's sides meet at 360° / sides: under 30°, so only its rims
    // are bevelled.
    let cylinder = Primitive::Cylinder(Default::default()).mesh();
    let sides = cylinder.faces().filter(|(_, c)| c.len() == 4).count();
    let chosen = chosen_edges(&cylinder, &standard(), BY_ANGLE);
    assert_eq!(chosen.len(), 2 * sides);
    let out = live_bevel(&cylinder, &standard(), BY_ANGLE).unwrap();
    clean_solid(&out);
    // An angle of 90° or more bevels nothing, and says why.
    let steep = LiveBevelOptions {
        angle: 90.0,
        ..standard()
    };
    assert_eq!(
        live_bevel(&cylinder, &steep, BY_ANGLE),
        Err(LiveBevelProblem::NothingSharp)
    );
    // A small angle takes the sides too.
    let fine = LiveBevelOptions {
        angle: 5.0,
        ..standard()
    };
    assert!(chosen_edges(&cylinder, &fine, BY_ANGLE).len() > 2 * sides);
}

#[test]
fn hard_edges_follow_the_objects_smoothing() {
    let hard = LiveBevelOptions {
        edges: BevelEdges::Hard,
        ..standard()
    };
    let mut mesh = cube();
    // The primitive marks its 12 edges hard.
    assert_eq!(chosen_edges(&mesh, &hard, Hardness::Marked).len(), 12);
    assert_eq!(chosen_edges(&mesh, &hard, BY_ANGLE).len(), 12);
    // Soften four: shaded by marks, those four aren't bevelled.
    let four: Vec<EdgeKey> = mesh.hard_edges().take(4).collect();
    crate::hardness::set_hard_edges(&mut mesh, &four, false);
    assert_eq!(chosen_edges(&mesh, &hard, Hardness::Marked).len(), 8);
    assert_eq!(chosen_edges(&mesh, &hard, BY_ANGLE).len(), 12);
    let out = live_bevel(&mesh, &hard, Hardness::Marked).unwrap();
    clean_solid(&out);
    // None hard: refused with a reason.
    let all: Vec<EdgeKey> = mesh.hard_edges().collect();
    crate::hardness::set_hard_edges(&mut mesh, &all, false);
    assert_eq!(
        live_bevel(&mesh, &hard, Hardness::Marked),
        Err(LiveBevelProblem::NothingHard)
    );
}

#[test]
fn marked_edges_are_bevelled_by_their_weight() {
    let marked = LiveBevelOptions {
        edges: BevelEdges::Marked,
        width: 10.0,
        segments: 1,
        ..standard()
    };
    let mut mesh = cube();
    assert_eq!(
        live_bevel(&mesh, &marked, BY_ANGLE),
        Err(LiveBevelProblem::NothingMarked)
    );
    // The top's four edges: two at full weight, two at half.
    let top: Vec<EdgeKey> = mesh
        .edges()
        .iter()
        .copied()
        .filter(|e| {
            let (a, b) = e.ends();
            mesh.position(a).unwrap().z > 99.0 && mesh.position(b).unwrap().z > 99.0
        })
        .collect();
    assert_eq!(top.len(), 4);
    set_bevel_weights(&mut mesh, &top[..2], 1.0);
    set_bevel_weights(&mut mesh, &top[2..], 0.5);
    assert_eq!(chosen_edges(&mesh, &marked, BY_ANGLE).len(), 4);
    let out = live_bevel(&mesh, &marked, BY_ANGLE).unwrap();
    clean_solid(&out);
    assert_eq!(out.face_count(), 6 + 4);
    // The chamfers' heights: 10 cm down for full weight, 5 for half.
    let mut drops: Vec<f32> = out
        .vertices()
        .map(|(_, p)| 100.0 - p.z)
        .filter(|d| *d > 0.01 && *d < 50.0)
        .collect();
    drops.sort_by(f32::total_cmp);
    drops.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    // (Where a full and a half edge meet, the corner between them slides
    // down the upright edge by their average, 7.5 cm.)
    let near = |x: f32| drops.iter().any(|d| (d - x).abs() < 0.01);
    assert!(near(5.0) && near(10.0), "{drops:?}");
    assert_eq!(drops.first().copied(), Some(5.0));
    assert_eq!(drops.last().copied(), Some(10.0));
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let wild = LiveBevelOptions {
        edges: BevelEdges::ByAngle,
        angle: f32::NAN,
        width: f32::INFINITY,
        segments: 0,
        profile: -7.0,
        clamp: true,
        soften: true,
    }
    .clamped();
    assert!(wild.soften, "a switch is kept as it is");
    assert_eq!(wild.angle, 30.0);
    assert_eq!(wild.width, 2.0);
    assert_eq!(wild.segments, 1);
    assert_eq!(wild.profile, -1.0);
    let huge = LiveBevelOptions {
        angle: 1e9,
        width: 1e9,
        segments: u32::MAX,
        ..standard()
    }
    .clamped();
    assert_eq!(
        (huge.angle, huge.width, huge.segments),
        (180.0, MAX_WIDTH, MAX_SEGMENTS)
    );
    let negative = LiveBevelOptions {
        width: -5.0,
        angle: -10.0,
        ..standard()
    }
    .clamped();
    assert_eq!((negative.width, negative.angle), (0.0, 0.0));
}

#[test]
fn a_huge_width_stops_at_neighbours_and_stays_sound() {
    let wide = LiveBevelOptions {
        width: 1e6,
        segments: 3,
        ..standard()
    };
    let out = live_bevel(&cube(), &wide, BY_ANGLE).unwrap();
    assert!(out.vertices().all(|(_, p)| p.is_finite()));
    let b = crate::Bounds::around(out.vertices().map(|(_, p)| p)).unwrap();
    assert!(b.size().max_element() <= 100.0 + 1e-3);
}

#[test]
fn a_zero_width_gives_the_shape_as_it_is() {
    let none = LiveBevelOptions {
        width: 0.0,
        ..standard()
    };
    let mesh = cube();
    assert_eq!(live_bevel(&mesh, &none, BY_ANGLE).unwrap(), mesh);
}

#[test]
fn a_fine_bevel_isnt_welded_away() {
    let fine = LiveBevelOptions {
        width: 0.05,
        segments: 4,
        ..standard()
    };
    let out = live_bevel(&cube(), &fine, BY_ANGLE).unwrap();
    clean_solid(&out);
    assert_eq!(out.face_count(), 6 + 12 * 4 + 8 * 12);
}

#[test]
fn n_gons_open_borders_and_empty_shapes() {
    // An open box: the rim's edges are borders and are skipped; the rest
    // are bevelled cleanly.
    let mut open = cube();
    let top = open
        .faces()
        .map(|(f, _)| f)
        .find(|f| open.face_normal(*f).z > 0.999)
        .unwrap();
    open.delete_faces(&[top]);
    let out = live_bevel(&open, &standard(), BY_ANGLE).unwrap();
    let report = MeshReport::new(&out);
    assert_eq!(report.non_manifold_edges, 0, "{report:?}");
    assert_eq!(report.flipped_edges, 0, "{report:?}");
    assert!(out.vertices().all(|(_, p)| p.is_finite()));
    assert!(out.face_count() > open.face_count());
    // A prism with n-gon caps.
    let prism = Primitive::Cylinder(crate::CylinderParams {
        sides: 6,
        ..Default::default()
    })
    .mesh();
    let out = live_bevel(&prism, &standard(), BY_ANGLE).unwrap();
    clean_solid(&out);
    // An empty mesh, and a flat sheet: nothing to bevel.
    assert_eq!(
        live_bevel(&Mesh::new(), &standard(), BY_ANGLE),
        Err(LiveBevelProblem::NothingSharp)
    );
    let plane = Primitive::Plane(Default::default()).mesh();
    assert!(live_bevel(&plane, &standard(), BY_ANGLE).is_err());
}

fn soft() -> LiveBevelOptions {
    LiveBevelOptions {
        soften: true,
        ..standard()
    }
}

/// The edges shaded hard by `mesh`'s marks.
fn marked_hard(mesh: &Mesh) -> usize {
    crate::hardness::shading_hard_edges(mesh, Hardness::Marked).len()
}

#[test]
fn soft_bevels_shade_round_on_a_shape_shaded_by_its_marks() {
    // The cube's 12 edges are marked hard: without Soft bevels, each
    // bevel keeps hard borders (decision 0040).
    let cube = cube();
    let plain = live_bevel(&cube, &standard(), Hardness::Marked).unwrap();
    assert!(marked_hard(&plain) >= 24, "{}", marked_hard(&plain));
    // With it, nothing is hard: every strip shades round.
    let round = live_bevel(&cube, &soft(), Hardness::Marked).unwrap();
    clean_solid(&round);
    assert_eq!(marked_hard(&round), 0);
    // The same polygons either way; only the marks differ.
    assert_eq!(round.face_count(), plain.face_count());
    // Softening is the bevel's own: the input keeps its marks.
    assert_eq!(cube.hard_edges().count(), 12);
}

#[test]
fn soft_bevels_leave_unbevelled_edges_as_they_are() {
    // Only marked edges bevelled: one of the twelve. The other eleven
    // stay hard, and so does the bevelled edge's neighbourhood's marks.
    let mut cube = cube();
    let one: Vec<EdgeKey> = cube.hard_edges().take(1).collect();
    set_bevel_weights(&mut cube, &one, 1.0);
    let marked = LiveBevelOptions {
        edges: BevelEdges::Marked,
        ..soft()
    };
    let out = live_bevel(&cube, &marked, Hardness::Marked).unwrap();
    assert_eq!(marked_hard(&out), 11);
    // A width of 0 bevels nothing, so nothing is softened.
    let none = LiveBevelOptions {
        width: 0.0,
        ..soft()
    };
    assert_eq!(live_bevel(&cube, &none, Hardness::Marked).unwrap(), cube);
}

#[test]
fn soft_bevels_keep_the_marks_of_edges_the_bevel_skips() {
    // Three quads fanned round one edge, marked hard: the bevel can't
    // take it, so it stays hard rather than smearing.
    let mut fan = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 10.0],
            [10.0, 0.0, 0.0],
            [10.0, 0.0, 10.0],
            [-10.0, 0.0, 0.0],
            [-10.0, 0.0, 10.0],
            [0.0, 10.0, 0.0],
            [0.0, 10.0, 10.0],
        ],
        &[&[0, 2, 3, 1], &[1, 5, 4, 0], &[0, 1, 7, 6]],
    );
    let spine: Vec<EdgeKey> = fan
        .topology()
        .edges()
        .filter(|e| fan.topology().edge_uses(*e).len() == 3)
        .collect();
    assert_eq!(spine.len(), 1);
    crate::hardness::set_hard_edges(&mut fan, &spine, true);
    let out = live_bevel(&fan, &soft(), BY_ANGLE).unwrap();
    assert_eq!(out, fan);
    // An open box (a cube missing its top): its rim is a border, never
    // bevelled, and its other edges soften.
    let mut open = cube();
    let top = open
        .faces()
        .find(|(_, c)| c.iter().all(|v| open.position(*v).unwrap().z > 99.0))
        .map(|(f, _)| f)
        .unwrap();
    assert_eq!(open.delete_faces(&[top]), 1);
    let out = live_bevel(&open, &soft(), Hardness::Marked).unwrap();
    assert!(out.vertices().all(|(_, p)| p.is_finite()));
    let topology = out.topology();
    assert!(
        crate::hardness::shading_hard_edges(&out, Hardness::Marked)
            .iter()
            .all(|e| topology.edge_uses(*e).len() < 2),
        "only the rim's marks are left"
    );
}

#[test]
fn zero_length_edges_and_non_manifold_edges_never_give_bad_numbers() {
    // A tent with two points on the same spot at its ridge.
    let tent = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 10.0, 0.0],
            [0.0, 10.0, 0.0],
            [5.0, 0.0, 5.0],
            [5.0, 5.0, 5.0],
            [5.0, 5.0, 5.0],
            [5.0, 10.0, 5.0],
        ],
        &[
            &[0, 1, 4],
            &[1, 2, 7, 6, 5, 4],
            &[2, 3, 7],
            &[3, 0, 4, 5, 6, 7],
        ],
    );
    let out = live_bevel(&tent, &standard(), BY_ANGLE).unwrap();
    assert!(out.vertices().all(|(_, p)| p.is_finite()));
    // Three quads fanned round one edge: that edge is skipped.
    let fan = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 10.0],
            [10.0, 0.0, 0.0],
            [10.0, 0.0, 10.0],
            [-10.0, 0.0, 0.0],
            [-10.0, 0.0, 10.0],
            [0.0, 10.0, 0.0],
            [0.0, 10.0, 10.0],
        ],
        &[&[0, 2, 3, 1], &[1, 5, 4, 0], &[0, 1, 7, 6]],
    );
    let out = live_bevel(&fan, &standard(), BY_ANGLE).unwrap();
    assert_eq!(out, fan);
    // A point that isn't a number: no panic, no new bad numbers.
    let mut broken = cube();
    let v = broken.vertices().next().unwrap().0;
    broken.set_position(v, Vec3::new(f32::NAN, 0.0, 0.0));
    let _ = live_bevel(&broken, &standard(), BY_ANGLE);
}

#[test]
fn uvs_seams_and_materials_come_along() {
    let mesh = cube();
    let out = live_bevel(&mesh, &standard(), BY_ANGLE).unwrap();
    assert!(crate::uvs::has_uvs(&mesh) && crate::uvs::has_uvs(&out));
    assert!(crate::uvs::seams(&out).len() >= crate::uvs::seams(&mesh).len());
    // Every polygon keeps a material slot of the input's.
    let slots: std::collections::BTreeSet<u32> = mesh
        .faces()
        .map(|(f, _)| crate::material::slot_of(&mesh, f))
        .collect();
    assert!(
        out.faces()
            .all(|(f, _)| slots.contains(&crate::material::slot_of(&out, f)))
    );
}

#[test]
fn the_same_shape_always_gives_the_same_result() {
    let torus = Primitive::Torus(Default::default()).mesh();
    let o = LiveBevelOptions {
        angle: 20.0,
        width: 1.0,
        segments: 3,
        ..standard()
    };
    let a = live_bevel(&torus, &o, BY_ANGLE);
    let b = live_bevel(&torus, &o, BY_ANGLE);
    assert_eq!(a, b);
    let a = live_bevel(&cube(), &standard(), BY_ANGLE).unwrap();
    let b = live_bevel(&cube(), &standard(), BY_ANGLE).unwrap();
    assert_eq!(a, b);
}

#[test]
fn every_primitive_bevels_without_bad_numbers() {
    for kind in crate::PrimitiveKind::ALL {
        let mesh = kind.standard().mesh();
        let before = MeshReport::new(&mesh);
        if let Ok(out) = live_bevel(&mesh, &standard(), BY_ANGLE) {
            let after = MeshReport::new(&out);
            assert!(out.vertices().all(|(_, p)| p.is_finite()), "{kind:?}");
            assert_eq!(after.non_manifold_edges, 0, "{kind:?}");
            assert_eq!(after.flipped_edges, 0, "{kind:?}");
            assert_eq!(after.is_closed(), before.is_closed(), "{kind:?}");
        }
    }
}

#[test]
fn the_work_estimate_counts_the_edges() {
    let mesh = cube();
    assert_eq!(edge_estimate(&mesh, &standard()), 12);
    let marked = LiveBevelOptions {
        edges: BevelEdges::Marked,
        ..standard()
    };
    assert_eq!(edge_estimate(&mesh, &marked), 0);
    let hard = LiveBevelOptions {
        edges: BevelEdges::Hard,
        ..standard()
    };
    assert_eq!(edge_estimate(&mesh, &hard), 12);
}

/// Timings, in a release build: `cargo test --release -p mesh
/// live_bevel_timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, for release builds"]
fn live_bevel_timing() {
    // A box cut into many quads: only its 240 box edges are sharp.
    let boxy = Primitive::Cube(crate::CubeParams {
        size: Vec3::splat(100.0),
        segments: [20, 20, 20],
    })
    .mesh();
    let o = LiveBevelOptions {
        angle: 0.0,
        width: 0.5,
        ..standard()
    };
    let start = std::time::Instant::now();
    let edges = chosen_edges(&boxy, &o, BY_ANGLE).len();
    let out = live_bevel(&boxy, &o, BY_ANGLE).unwrap();
    println!(
        "box of {} polygons, {edges} sharp edges, by angle: {} polygons in {:.1} ms",
        boxy.face_count(),
        out.face_count(),
        start.elapsed().as_secs_f64() * 1e3
    );
    // A sphere of about 10 000 polygons, every edge bevelled.
    let sphere = Primitive::Sphere(crate::SphereParams {
        sides: 128,
        rings: 80,
        ..Default::default()
    })
    .mesh();
    let start = std::time::Instant::now();
    let out = live_bevel(&sphere, &o, BY_ANGLE).unwrap();
    let edges = chosen_edges(&sphere, &o, BY_ANGLE).len();
    println!(
        "sphere of {} polygons, {edges} edges: {} polygons in {:.1} ms",
        sphere.face_count(),
        out.face_count(),
        start.elapsed().as_secs_f64() * 1e3
    );
}

#[test]
fn a_corner_with_a_fan_of_polygons_is_cut_away() {
    // A cube whose bottom is a fan of triangles from one corner (as a
    // boolean leaves a box's corners), three of them touching no bevelled
    // edge. Their edges slide too, so the corner point goes and the corner
    // patch doesn't fold round it (decision 0156).
    let fanned = mesh_of(
        &[
            [-50.0, -50.0, 0.0],
            [50.0, -50.0, 0.0],
            [50.0, 50.0, 0.0],
            [-50.0, 50.0, 0.0],
            [-50.0, -50.0, 100.0],
            [50.0, -50.0, 100.0],
            [50.0, 50.0, 100.0],
            [-50.0, 50.0, 100.0],
            [-23.0, -9.5, 0.0],
            [-17.7, -17.7, 0.0],
            [-9.5, -23.0, 0.0],
            [0.0, -25.0, 0.0],
        ],
        &[
            &[0, 3, 8],
            &[0, 8, 9],
            &[0, 9, 10],
            &[0, 10, 11],
            &[0, 11, 1],
            &[3, 2, 1, 11, 10, 9, 8],
            &[4, 5, 6, 7],
            &[0, 1, 5, 4],
            &[1, 2, 6, 5],
            &[2, 3, 7, 6],
            &[3, 0, 4, 7],
        ],
    );
    clean_solid(&fanned);
    for segments in [1, 2, 3] {
        let o = LiveBevelOptions { segments, ..soft() };
        let out = live_bevel(&fanned, &o, BY_ANGLE).unwrap();
        clean_solid(&out);
        let corner = Vec3::new(-50.0, -50.0, 0.0);
        assert!(
            out.vertices().all(|(_, p)| p.distance(corner) > 1.0),
            "{segments}: the corner is cut away"
        );
        // The bottom's slid points stay on the bottom, outside the 2 cm
        // strips and close to them.
        let slid: Vec<Vec3> = out
            .vertices()
            .map(|(_, p)| p)
            .filter(|p| p.z.abs() < 1e-3 && p.x < -40.0 && p.y < -40.0)
            .collect();
        assert!(!slid.is_empty());
        for p in slid {
            let inside = p.x.min(p.y) + 50.0;
            assert!(inside > 1.99 && p.x.max(p.y) < -44.0, "{segments}: {p}");
        }
    }
}
