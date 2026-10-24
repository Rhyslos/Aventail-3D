//! Tests for Quadify (decision 0150).

use super::*;
use crate::attributes::{AttributeValues, Domain, SEAM, UV, Value};
use crate::decimate::{Decimator, Keep};
use crate::material::set_material;
use crate::primitives::{CylinderParams, PlaneParams, SphereParams};
use crate::{MeshReport, Primitive};
use glam::{Vec2, Vec3};

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

/// `mesh` cut into triangles, with its UVs, materials and marks (through
/// Decimate's set-up, which does exactly that).
fn triangles_of(mesh: &Mesh) -> Mesh {
    Decimator::new(mesh, Keep::default(), Hardness::Marked)
        .unwrap()
        .mesh(false)
}

fn grid(segments: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        width: 100.0,
        depth: 100.0,
        segments: [segments, segments],
    })
    .mesh()
}

fn sphere(sides: u32, rings: u32) -> Mesh {
    Primitive::Sphere(SphereParams {
        radius: 50.0,
        sides,
        rings,
    })
    .mesh()
}

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

fn standard() -> QuadifyOptions {
    QuadifyOptions::default()
}

/// Two triangles making the unit square (0, 1, 2) and (0, 2, 3), sharing
/// the edge 0–2.
fn square() -> Mesh {
    mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        &[&[0, 1, 2], &[0, 2, 3]],
    )
}

/// The edge between points `a` and `b` (by their order of making).
fn edge(a: u32, b: u32) -> EdgeKey {
    EdgeKey::new(VertexId::from_raw(a), VertexId::from_raw(b))
}

fn sizes(mesh: &Mesh) -> Vec<usize> {
    let mut out: Vec<usize> = mesh.faces().map(|(_, c)| c.len()).collect();
    out.sort_unstable();
    out
}

fn total_area(mesh: &Mesh) -> f32 {
    mesh.faces()
        .map(|(_, corners)| {
            let mut sum = Vec3::ZERO;
            for (a, b) in crate::polygon::sides(corners) {
                sum += mesh.position(a).unwrap().cross(mesh.position(b).unwrap());
            }
            sum.length() / 2.0
        })
        .sum()
}

fn joined(mesh: &Mesh, options: &QuadifyOptions, hardness: Hardness) -> Mesh {
    let mut out = mesh.clone();
    quadify(&mut out, None, options, hardness);
    out
}

#[test]
fn the_standard_settings() {
    let o = standard();
    assert_eq!((o.max_angle, o.max_distortion), (40.0, 40.0));
    assert!(o.keep_seams && o.keep_hard_edges && o.keep_materials);
}

#[test]
fn odd_settings_are_brought_within_limits() {
    let o = QuadifyOptions {
        max_angle: f32::NAN,
        max_distortion: f32::INFINITY,
        ..standard()
    }
    .clamped();
    assert_eq!((o.max_angle, o.max_distortion), (40.0, 40.0));
    let o = QuadifyOptions {
        max_angle: 1e9,
        max_distortion: 500.0,
        ..standard()
    }
    .clamped();
    assert_eq!((o.max_angle, o.max_distortion), (MAX_ANGLE, MAX_DISTORTION));
    let o = QuadifyOptions {
        max_angle: -5.0,
        max_distortion: -1.0,
        ..standard()
    }
    .clamped();
    assert_eq!((o.max_angle, o.max_distortion), (0.0, 0.0));
}

#[test]
fn two_triangles_of_a_square_become_one_quad_keeping_the_lower_id() {
    let mut mesh = square();
    let outcome = quadify(&mut mesh, None, &standard(), Hardness::Marked);
    assert_eq!(outcome.matched, 1);
    assert_eq!(mesh.face_count(), 1);
    let (id, corners) = mesh.faces().next().unwrap();
    assert_eq!(id, FaceId::from_raw(0));
    assert_eq!(corners.len(), 4);
    // Still facing up, every point where it was.
    assert!(mesh.face_normal(id).z > 0.99);
    assert_eq!(mesh.vertex_count(), 4);
}

#[test]
fn a_triangulated_grid_comes_back_as_its_quads() {
    let original = grid(4);
    let tris = triangles_of(&original);
    assert_eq!(tris.face_count(), 32);
    let out = joined(&tris, &standard(), Hardness::Marked);
    assert_eq!(sizes(&out), vec![4; 16]);
    assert!((total_area(&out) - total_area(&original)).abs() < 1e-2);
    // The border is untouched and the points haven't moved.
    let report = MeshReport::new(&out);
    assert_eq!(report.border_edges, 16, "{report:?}");
    assert_eq!(report.non_manifold_edges, 0);
    assert_eq!(report.flipped_edges, 0);
    let mut before: Vec<[u32; 3]> = tris
        .vertices()
        .map(|(_, p)| p.to_array().map(f32::to_bits))
        .collect();
    let mut after: Vec<[u32; 3]> = out
        .vertices()
        .map(|(_, p)| p.to_array().map(f32::to_bits))
        .collect();
    before.sort_unstable();
    after.sort_unstable();
    assert_eq!(before, after);
}

#[test]
fn a_triangulated_cube_comes_back_as_six_quads_and_a_clean_solid() {
    let tris = triangles_of(&cube());
    assert_eq!(tris.face_count(), 12);
    // Its box edges are hard and its diagonals soft: kept either way.
    for hardness in [Hardness::Marked, Hardness::ByAngle(30.0)] {
        let out = joined(&tris, &standard(), hardness);
        assert_eq!(sizes(&out), vec![4; 6], "{hardness:?}");
        let report = MeshReport::new(&out);
        assert!(report.is_closed(), "{report:?}");
        assert!(!report.has_problems(), "{report:?}");
    }
}

#[test]
fn a_triangulated_sphere_gets_its_quads_back_and_its_caps_stay_triangles() {
    let original = sphere(16, 8);
    let quads = original.faces().filter(|(_, c)| c.len() == 4).count();
    let tris = triangles_of(&original);
    let out = joined(&tris, &standard(), Hardness::ByAngle(30.0));
    let got = out.faces().filter(|(_, c)| c.len() == 4).count();
    assert_eq!(got, quads, "{:?}", sizes(&out));
    let report = MeshReport::new(&out);
    assert!(report.is_closed(), "{report:?}");
    assert!(!report.has_problems(), "{report:?}");
}

#[test]
fn uvs_come_along_with_their_corners() {
    let tris = triangles_of(&grid(3));
    assert!(crate::uvs::has_uvs(&tris));
    let uv_at = |mesh: &Mesh| {
        let mut out = Vec::new();
        for (face, corners) in mesh.faces() {
            for (k, v) in corners.iter().enumerate() {
                let p = mesh.position(*v).unwrap().to_array().map(f32::to_bits);
                let uv = crate::uvs::corner_uv(mesh, face, k).unwrap();
                out.push((p, uv.to_array().map(f32::to_bits)));
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    };
    let out = joined(&tris, &standard(), Hardness::Marked);
    assert_eq!(sizes(&out), vec![4; 9]);
    assert_eq!(uv_at(&out), uv_at(&tris));
}

/// The unit square's two triangles, sharing its diagonal 0–1, the second
/// turned up by `degrees` about it.
fn hinge(degrees: f32) -> Mesh {
    let (s, c) = degrees.to_radians().sin_cos();
    let r = 0.5f32.sqrt();
    mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.5 + 0.5 * c, 0.5 - 0.5 * c, r * s],
        ],
        &[&[0, 1, 2], &[1, 0, 3]],
    )
}

#[test]
fn max_angle_limits_how_far_the_two_may_lean() {
    let bent = hinge(60.0);
    assert_eq!(joined(&bent, &standard(), Hardness::Marked).face_count(), 2);
    let wide = QuadifyOptions {
        max_angle: 70.0,
        ..standard()
    };
    assert_eq!(joined(&bent, &wide, Hardness::Marked).face_count(), 1);
    // Flat is always within the angle, even at 0°.
    let flat = QuadifyOptions {
        max_angle: 0.0,
        ..standard()
    };
    assert_eq!(joined(&hinge(0.0), &flat, Hardness::Marked).face_count(), 1);
}

/// A flat rhombus with corners of 60° and 120°, cut along its short
/// diagonal.
fn rhombus() -> Mesh {
    let h = 3f32.sqrt() / 2.0;
    mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.5, h, 0.0],
            [0.5, h, 0.0],
        ],
        &[&[0, 1, 3], &[1, 2, 3]],
    )
}

#[test]
fn max_shape_distortion_limits_how_far_corners_are_from_square() {
    // Its corners are 30° from square.
    let mesh = rhombus();
    assert_eq!(joined(&mesh, &standard(), Hardness::Marked).face_count(), 1);
    let strict = QuadifyOptions {
        max_distortion: 20.0,
        ..standard()
    };
    assert_eq!(joined(&mesh, &strict, Hardness::Marked).face_count(), 2);
}

#[test]
fn a_quad_with_a_dent_is_never_made() {
    // An arrowhead: the two triangles would make a quad bent inwards.
    let mesh = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.5, 1.0, 0.0],
        ],
        &[&[0, 1, 3], &[3, 1, 2]],
    );
    let loose = QuadifyOptions {
        max_angle: 180.0,
        max_distortion: 90.0,
        ..standard()
    };
    assert_eq!(joined(&mesh, &loose, Hardness::Marked).face_count(), 2);
}

#[test]
fn the_squarest_pairs_go_first() {
    // Two squares side by side, each cut corner to corner the same way:
    // the middle triangles could also pair into a slanted shape, but each
    // square's own two triangles are squarer.
    let mesh = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [2.0, 1.0, 0.0],
        ],
        &[&[0, 1, 4], &[0, 4, 3], &[1, 2, 5], &[1, 5, 4]],
    );
    let loose = QuadifyOptions {
        max_distortion: 90.0,
        ..standard()
    };
    let chosen = pairs(&mesh, None, &loose, Hardness::Marked);
    assert_eq!(
        chosen,
        vec![
            [FaceId::from_raw(0), FaceId::from_raw(1)],
            [FaceId::from_raw(2), FaceId::from_raw(3)]
        ]
    );
}

#[test]
fn materials_are_kept_apart_unless_untucked() {
    let mut mesh = square();
    set_material(&mut mesh, &[FaceId::from_raw(1)], 1);
    assert_eq!(joined(&mesh, &standard(), Hardness::Marked).face_count(), 2);
    let any = QuadifyOptions {
        keep_materials: false,
        ..standard()
    };
    let out = joined(&mesh, &any, Hardness::Marked);
    assert_eq!(out.face_count(), 1);
    // The joined polygon takes the lower id's material.
    assert_eq!(crate::material::slot_of(&out, FaceId::from_raw(0)), 0);
}

#[test]
fn seams_and_uv_splits_are_kept_unless_unticked() {
    let any = QuadifyOptions {
        keep_seams: false,
        ..standard()
    };
    // A marked seam on the diagonal.
    let mut marked = square();
    crate::uvs::set_seams(&mut marked, &[edge(0, 2)], true);
    assert_eq!(
        joined(&marked, &standard(), Hardness::Marked).face_count(),
        2
    );
    assert_eq!(joined(&marked, &any, Hardness::Marked).face_count(), 1);
    // UVs split along the diagonal: each triangle its own piece.
    let mut split = square();
    let uvs = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(0.5, 0.0),
        Vec2::new(0.5, 0.5),
        Vec2::new(0.6, 0.6),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.6, 1.0),
    ];
    split
        .set_attribute_values(
            UV,
            Domain::Corner,
            Value::Vec2(Vec2::ZERO),
            AttributeValues::Vec2(uvs),
        )
        .unwrap();
    assert_eq!(
        joined(&split, &standard(), Hardness::Marked).face_count(),
        2
    );
    assert_eq!(joined(&split, &any, Hardness::Marked).face_count(), 1);
    // A seam layer with nothing marked keeps nothing.
    let mut unmarked = square();
    let _ = unmarked.set_attribute_values(
        SEAM,
        Domain::Edge,
        Value::Bool(false),
        AttributeValues::Edges(Vec::new()),
    );
    assert_eq!(
        joined(&unmarked, &standard(), Hardness::Marked).face_count(),
        1
    );
}

#[test]
fn hard_edges_are_the_ones_the_shading_shows() {
    let mut mesh = square();
    mesh.set_hard(VertexId::from_raw(0), VertexId::from_raw(2), true);
    // Shaded by its marks, the diagonal shows hard: kept.
    assert_eq!(joined(&mesh, &standard(), Hardness::Marked).face_count(), 2);
    // Shaded by angle, a flat edge isn't hard, whatever its mark.
    assert_eq!(
        joined(&mesh, &standard(), Hardness::ByAngle(30.0)).face_count(),
        1
    );
    let any = QuadifyOptions {
        keep_hard_edges: false,
        ..standard()
    };
    assert_eq!(joined(&mesh, &any, Hardness::Marked).face_count(), 1);
    // By angle, a fold sharper than the angle is hard.
    let bent = hinge(35.0);
    assert_eq!(
        joined(&bent, &standard(), Hardness::ByAngle(30.0)).face_count(),
        2
    );
    assert_eq!(joined(&bent, &standard(), Hardness::Marked).face_count(), 1);
}

#[test]
fn creased_and_bevel_weighted_edges_always_stay() {
    let any = QuadifyOptions {
        keep_seams: false,
        keep_hard_edges: false,
        keep_materials: false,
        ..standard()
    };
    let mut creased = square();
    crate::crease::set_creases(&mut creased, &[edge(0, 2)], 0.5);
    assert_eq!(joined(&creased, &any, Hardness::Marked).face_count(), 2);
    let mut weighted = square();
    crate::bevel_weight::set_bevel_weights(&mut weighted, &[edge(0, 2)], 1.0);
    assert_eq!(joined(&weighted, &any, Hardness::Marked).face_count(), 2);
}

#[test]
fn only_the_chosen_polygons_are_joined() {
    let tris = triangles_of(&grid(2));
    let all: Vec<FaceId> = tris.faces().map(|(f, _)| f).collect();
    assert_eq!(all.len(), 8);
    // One square's pair, and one triangle whose partner isn't chosen.
    let full = pairs(&tris, None, &standard(), Hardness::Marked);
    let [a, b] = full[0];
    let lone = full[1][0];
    let mut out = tris.clone();
    let gone = FaceId::from_raw(10_000);
    let outcome = quadify(
        &mut out,
        Some(&[a, b, lone, gone]),
        &standard(),
        Hardness::Marked,
    );
    assert_eq!((outcome.matched, outcome.missing), (1, 1));
    assert_eq!(out.face_count(), 7);
    // Nothing chosen joins nothing.
    let mut out = tris.clone();
    let outcome = quadify(&mut out, Some(&[]), &standard(), Hardness::Marked);
    assert_eq!((outcome.matched, outcome.skipped), (0, 1));
    assert_eq!(out, tris);
}

#[test]
fn n_gons_and_quads_are_left_as_they_are() {
    let cylinder = Primitive::Cylinder(CylinderParams {
        radius: 20.0,
        height: 50.0,
        sides: 8,
        caps: true,
        cap_centre: false,
        ..Default::default()
    })
    .mesh();
    let out = joined(&cylinder, &standard(), Hardness::Marked);
    assert_eq!(sizes(&out), sizes(&cylinder));
    assert_eq!(
        quadified(&cylinder, &standard(), Hardness::Marked),
        Err(QuadifyProblem::NoTriangles)
    );
    // A triangle beside an n-gon stays a triangle.
    let mesh = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.5, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 2.0, 0.0],
        ],
        &[&[0, 1, 2, 3, 4], &[4, 3, 5]],
    );
    assert_eq!(
        quadified(&mesh, &standard(), Hardness::Marked),
        Err(QuadifyProblem::NothingJoins)
    );
}

#[test]
fn degenerate_input_is_left_alone_never_a_panic() {
    let loose = QuadifyOptions {
        max_angle: 180.0,
        max_distortion: 90.0,
        keep_seams: false,
        keep_hard_edges: false,
        keep_materials: false,
    };
    // An empty mesh.
    let empty = Mesh::new();
    assert_eq!(
        quadified(&empty, &standard(), Hardness::Marked),
        Err(QuadifyProblem::NoTriangles)
    );
    // A lone triangle: an open border all round.
    let lone = mesh_of(&[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], &[&[0, 1, 2]]);
    assert_eq!(
        quadified(&lone, &loose, Hardness::Marked),
        Err(QuadifyProblem::NothingJoins)
    );
    // A triangle with no area (a zero-length edge) beside a good one.
    let flat = mesh_of(
        &[[0.0; 3], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
        &[&[0, 1, 2], &[0, 2, 3]],
    );
    assert_eq!(joined(&flat, &loose, Hardness::Marked).face_count(), 2);
    // An edge shared by three triangles.
    let fan = mesh_of(
        &[
            [0.0; 3],
            [1.0, 0.0, 0.0],
            [0.5, 1.0, 0.0],
            [0.5, -1.0, 0.0],
            [0.5, 0.0, 1.0],
        ],
        &[&[0, 1, 2], &[1, 0, 3], &[1, 0, 4]],
    );
    assert_eq!(joined(&fan, &loose, Hardness::Marked).face_count(), 3);
    // Neighbours facing opposite ways round.
    let flipped = mesh_of(
        &[[0.0; 3], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
        &[&[0, 1, 2], &[0, 3, 2]],
    );
    assert_eq!(joined(&flipped, &loose, Hardness::Marked).face_count(), 2);
    // A point that isn't at a real place.
    let nan = mesh_of(
        &[
            [0.0; 3],
            [1.0, 0.0, 0.0],
            [f32::NAN, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        &[&[0, 1, 2], &[0, 2, 3]],
    );
    assert_eq!(joined(&nan, &loose, Hardness::Marked).face_count(), 2);
    // Settings past their limits are brought within them: no distortion
    // at all still lets a true square be joined, but not a rhombus.
    let wild = QuadifyOptions {
        max_angle: f32::NAN,
        max_distortion: -1e30,
        ..loose
    };
    assert_eq!(joined(&square(), &wild, Hardness::Marked).face_count(), 1);
    assert_eq!(joined(&rhombus(), &wild, Hardness::Marked).face_count(), 2);
}

#[test]
fn quadifying_is_deterministic() {
    let tris = triangles_of(&sphere(24, 12));
    let one = joined(&tris, &standard(), Hardness::ByAngle(30.0));
    let two = joined(&tris.clone(), &standard(), Hardness::ByAngle(30.0));
    assert_eq!(one, two);
    let ids = |m: &Mesh| m.faces().map(|(f, c)| (f, c.to_vec())).collect::<Vec<_>>();
    assert_eq!(ids(&one), ids(&two));
}

#[test]
fn quadifying_twice_changes_nothing_more() {
    let tris = triangles_of(&sphere(12, 6));
    let once = joined(&tris, &standard(), Hardness::Marked);
    let mut twice = once.clone();
    let outcome = quadify(&mut twice, None, &standard(), Hardness::Marked);
    assert_eq!(outcome.matched, 0);
    assert_eq!(once, twice);
}

#[test]
fn random_settings_and_shapes_never_give_a_broken_mesh() {
    let shapes = [
        triangles_of(&cube()),
        triangles_of(&sphere(10, 6)),
        triangles_of(&grid(5)),
        Primitive::Icosphere(Default::default()).mesh(),
        sphere(8, 4),
    ];
    let mut seed: u64 = 0x5eed;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for round in 0..60 {
        let shape = &shapes[round % shapes.len()];
        let bits = next();
        let options = QuadifyOptions {
            max_angle: (next() % 200) as f32 - 10.0,
            max_distortion: (next() % 120) as f32 - 10.0,
            keep_seams: bits & 1 != 0,
            keep_hard_edges: bits & 2 != 0,
            keep_materials: bits & 4 != 0,
        };
        let hardness = if bits & 8 != 0 {
            Hardness::Marked
        } else {
            Hardness::ByAngle(30.0)
        };
        let out = joined(shape, &options, hardness);
        let before = MeshReport::new(shape);
        let after = MeshReport::new(&out);
        assert!(out.face_count() <= shape.face_count());
        assert!(out.face_count() * 2 >= shape.face_count());
        assert_eq!(after.border_edges, before.border_edges, "{round}");
        assert_eq!(after.non_manifold_edges, 0, "{round}: {after:?}");
        assert_eq!(after.flipped_edges, 0, "{round}: {after:?}");
        assert_eq!(after.loose_points, 0, "{round}: {after:?}");
        // Points never move or go.
        assert_eq!(out.vertex_count(), shape.vertex_count(), "{round}");
        if options.max_angle <= 1.0 {
            assert!((total_area(&out) - total_area(shape)).abs() < total_area(shape) * 1e-3);
        }
    }
}

#[test]
#[ignore = "timing: run with --release -- --ignored --nocapture"]
fn timings() {
    let input = triangles_of(&sphere(256, 100));
    let n = triangle_count(&input);
    let start = std::time::Instant::now();
    let out = quadified(&input, &standard(), Hardness::ByAngle(30.0)).unwrap();
    println!(
        "sphere of {n} triangles to {} polygons: {:?}",
        out.face_count(),
        start.elapsed()
    );
}
