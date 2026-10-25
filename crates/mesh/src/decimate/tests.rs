use super::*;
use crate::attributes::SEAM;
use crate::material::{MATERIAL, set_material};
use crate::primitives::{CylinderParams, PlaneParams, SphereParams};
use crate::{FaceId, MeshReport, Primitive, VertexId};
use std::collections::HashSet;

fn sphere(sides: u32, rings: u32) -> Mesh {
    Primitive::Sphere(SphereParams {
        radius: 50.0,
        sides,
        rings,
    })
    .mesh()
}

fn grid(segments: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        width: 100.0,
        depth: 100.0,
        segments: [segments, segments],
    })
    .mesh()
}

fn ratio(ratio: f32) -> DecimateOptions {
    DecimateOptions {
        ratio,
        ..Default::default()
    }
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

fn assert_clean_solid(mesh: &Mesh) {
    let report = MeshReport::new(mesh);
    assert!(report.faces > 0);
    assert_eq!(report.border_edges, 0, "{report:?}");
    assert_eq!(report.non_manifold_edges, 0, "{report:?}");
    assert_eq!(report.flipped_edges, 0, "{report:?}");
    assert_eq!(report.loose_points, 0, "{report:?}");
}

fn places(mesh: &Mesh) -> Vec<[u32; 3]> {
    mesh.vertices()
        .map(|(_, p)| p.to_array().map(f32::to_bits))
        .collect()
}

fn all_triangles(mesh: &Mesh) -> bool {
    mesh.faces().all(|(_, c)| c.len() == 3)
}

/// The UVs of each corner, by the corner's point place.
fn corner_uvs(mesh: &Mesh) -> Vec<([u32; 3], [u32; 2])> {
    let mut out = Vec::new();
    for (face, corners) in mesh.faces() {
        for (k, v) in corners.iter().enumerate() {
            let p = mesh.position(*v).unwrap().to_array().map(f32::to_bits);
            let uv = crate::uvs::corner_uv(mesh, face, k).unwrap();
            out.push((p, uv.to_array().map(f32::to_bits)));
        }
    }
    out
}

#[test]
fn a_sphere_halves_and_stays_a_round_closed_solid() {
    let input = sphere(32, 16);
    let before = triangle_count(&input);
    let out = decimate(&input, &ratio(0.5), Hardness::Marked).unwrap();
    let after = triangle_count(&out);
    assert!(all_triangles(&out));
    assert!(
        after <= before / 2 && after + 2 >= before / 2,
        "{before} → {after}"
    );
    assert_clean_solid(&out);
    // Every point is one of the sphere's own, so it is on the sphere.
    let originals: HashSet<[u32; 3]> = places(&input).into_iter().collect();
    assert!(places(&out).iter().all(|p| originals.contains(p)));
    // And it stays round: the middles of its triangles stay near it (the
    // sphere stands on the ground, its middle 50 cm up).
    let centre = input.bounds().unwrap().centre();
    for (_, corners) in out.faces() {
        let mid = corners
            .iter()
            .map(|v| out.position(*v).unwrap())
            .sum::<Vec3>()
            / 3.0;
        assert!(mid.distance(centre) > 46.0, "{mid}");
    }
}

#[test]
fn decimating_is_deterministic() {
    let input = sphere(48, 24);
    let a = decimate(&input, &ratio(0.2), Hardness::Marked).unwrap();
    let b = decimate(&input, &ratio(0.2), Hardness::Marked).unwrap();
    assert_eq!(a, b);
    assert_eq!(places(&a), places(&b));
}

#[test]
fn a_target_in_triangles_is_met() {
    let input = sphere(32, 16);
    let options = DecimateOptions {
        reduce_by: ReduceBy::Triangles,
        triangles: 300,
        ..Default::default()
    };
    let out = decimate(&input, &options, Hardness::Marked).unwrap();
    let n = triangle_count(&out);
    assert!((299..=300).contains(&n), "{n}");
}

#[test]
fn a_target_at_or_above_the_shape_changes_nothing() {
    let input = sphere(16, 8);
    for options in [
        ratio(1.0),
        DecimateOptions {
            reduce_by: ReduceBy::Triangles,
            triangles: DecimateOptions::MAX_TRIANGLES,
            ..Default::default()
        },
    ] {
        assert_eq!(decimate(&input, &options, Hardness::Marked).unwrap(), input);
    }
}

#[test]
fn a_cube_keeps_its_corners_whatever_the_ratio() {
    // Its corners are where hard edges meet: they never move.
    let cube = Primitive::Cube(Default::default()).mesh();
    let out = decimate(&cube, &ratio(0.01), Hardness::Marked).unwrap();
    assert_eq!(triangle_count(&out), 12);
    assert_clean_solid(&out);
    assert_eq!(out.bounds(), cube.bounds());
    // Its hard edges are still marked.
    assert_eq!(out.hard_edge_count(), 12);
}

#[test]
fn hard_edges_by_angle_keep_a_cylinder_s_rims() {
    let cylinder = Primitive::Cylinder(CylinderParams {
        sides: 32,
        height_segments: 8,
        ..Default::default()
    })
    .mesh();
    let (lo, hi) = {
        let b = cylinder.bounds().unwrap();
        (b.min.z, b.max.z)
    };
    let out = decimate(&cylinder, &ratio(0.1), Hardness::ByAngle(30.0)).unwrap();
    assert_clean_solid(&out);
    // The rims are sharper than 30°: they stay lines, which may lose
    // points along them but keep their corners rounder than 30°.
    let rim = |m: &Mesh, z: f32| m.vertices().filter(|(_, p)| p.z == z).count();
    assert!(
        rim(&out, lo) >= 12 && rim(&out, hi) >= 12,
        "{}",
        rim(&out, lo)
    );
    let b = out.bounds().unwrap();
    assert_eq!((b.min.z, b.max.z), (lo, hi));
    // Nothing between the rims is left: the side lost its rings.
    assert!(out.vertices().all(|(_, p)| p.z == lo || p.z == hi));
}

#[test]
fn an_open_grid_keeps_its_border_and_stays_flat() {
    let input = grid(20);
    let out = decimate(&input, &ratio(0.05), Hardness::Marked).unwrap();
    assert!(triangle_count(&out) < 100);
    assert_eq!(out.bounds(), input.bounds());
    // Flat stays flat, and the border is still a square of four sides.
    assert!(out.vertices().all(|(_, p)| p.z == 0.0));
    let report = MeshReport::new(&out);
    assert_eq!(report.non_manifold_edges, 0);
    assert_eq!(report.flipped_edges, 0);
    let border_length: f32 = border_edges(&out).iter().map(|(a, b)| a.distance(*b)).sum();
    assert!((border_length - 400.0).abs() < 0.01, "{border_length}");
}

fn border_edges(mesh: &Mesh) -> Vec<(Vec3, Vec3)> {
    let topology = mesh.topology();
    topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 1)
        .map(|e| {
            let (a, b) = e.ends();
            (mesh.position(a).unwrap(), mesh.position(b).unwrap())
        })
        .collect()
}

#[test]
fn borders_not_kept_may_wear_away_but_the_surface_stays_whole() {
    let input = grid(12);
    let options = DecimateOptions {
        ratio: 0.1,
        keep: Keep {
            borders: false,
            ..Keep::default()
        },
        ..Default::default()
    };
    let out = decimate(&input, &options, Hardness::Marked).unwrap();
    let report = MeshReport::new(&out);
    assert_eq!(report.non_manifold_edges, 0, "{report:?}");
    assert_eq!(report.flipped_edges, 0, "{report:?}");
    assert!(triangle_count(&out) <= 30);
}

#[test]
fn material_borders_stay_where_they_were() {
    let mut input = grid(16);
    // The half with X above the middle gets material 1.
    let right: Vec<FaceId> = input
        .faces()
        .filter(|(_, c)| c.iter().all(|v| input.position(*v).unwrap().x >= 0.0))
        .map(|(f, _)| f)
        .collect();
    set_material(&mut input, &right, 1);
    let out = decimate(&input, &ratio(0.1), Hardness::Marked).unwrap();
    assert!(out.attribute_values(MATERIAL).is_some());
    for (face, corners) in out.faces() {
        let xs: Vec<f32> = corners
            .iter()
            .map(|v| out.position(*v).unwrap().x)
            .collect();
        if crate::material::slot_of(&out, face) == 1 {
            assert!(xs.iter().all(|x| *x >= 0.0), "{xs:?}");
        } else {
            assert!(xs.iter().all(|x| *x <= 0.0), "{xs:?}");
        }
    }
}

#[test]
fn uv_seams_stay_and_every_corner_keeps_one_of_its_own_uvs() {
    let input = sphere(32, 16);
    assert!(crate::uvs::has_uvs(&input));
    let before: HashSet<([u32; 3], [u32; 2])> = corner_uvs(&input).into_iter().collect();
    let out = decimate(&input, &ratio(0.25), Hardness::Marked).unwrap();
    assert!(crate::uvs::has_uvs(&out));
    for corner in corner_uvs(&out) {
        assert!(before.contains(&corner), "{corner:?}");
    }
    // The UVs still split where they did.
    assert!(!crate::uvs::uv_splits(&out).is_empty());
    let report = MeshReport::new(&out);
    assert_eq!(report.border_edges, 0);
}

#[test]
fn marked_seams_and_creases_are_carried_along_their_lines() {
    let mut input = grid(10);
    // A seam and a crease along the line X = 0.
    let line: Vec<crate::EdgeKey> = input
        .topology()
        .edges()
        .filter(|e| {
            let (a, b) = e.ends();
            input.position(a).unwrap().x == 0.0 && input.position(b).unwrap().x == 0.0
        })
        .collect();
    assert_eq!(line.len(), 10);
    crate::uvs::set_seams(&mut input, &line, true);
    crate::crease::set_creases(&mut input, &line, 0.5);
    let out = decimate(&input, &ratio(0.1), Hardness::Marked).unwrap();
    let on_line = |e: &crate::EdgeKey| {
        let (a, b) = e.ends();
        out.position(a).unwrap().x == 0.0 && out.position(b).unwrap().x == 0.0
    };
    let seams = crate::uvs::seams(&out);
    assert!(!seams.is_empty());
    assert!(seams.iter().all(on_line));
    let length: f32 = seams
        .iter()
        .map(|e| {
            let (a, b) = e.ends();
            out.position(a).unwrap().distance(out.position(b).unwrap())
        })
        .sum();
    assert!((length - 100.0).abs() < 0.01, "the whole line: {length}");
    let creases = crate::crease::creases(&out);
    assert_eq!(creases.len(), seams.len());
    assert!(creases.iter().all(|(e, c)| on_line(e) && *c == 0.5));
    assert!(out.attribute_values(SEAM).is_some());
}

#[test]
fn quads_where_flat_join_flat_pairs() {
    let input = grid(20);
    let options = DecimateOptions {
        ratio: 0.3,
        quads_where_flat: true,
        ..Default::default()
    };
    let out = decimate(&input, &options, Hardness::Marked).unwrap();
    let quads = out.faces().filter(|(_, c)| c.len() == 4).count();
    assert!(quads > 0);
    assert!(out.faces().all(|(_, c)| c.len() <= 4));
    let target = options.target(triangle_count(&input));
    assert!(triangle_count(&out) <= target, "triangles still counted");
    let report = MeshReport::new(&out);
    assert_eq!(report.flipped_edges, 0);
    assert_eq!(report.non_manifold_edges, 0);
    // On a sphere only pairs still lying flat (from one of its own flat
    // quads) join.
    let round = decimate(&sphere(24, 12), &options, Hardness::Marked).unwrap();
    for (_, corners) in round.faces().filter(|(_, c)| c.len() == 4) {
        let p: Vec<Vec3> = corners
            .iter()
            .map(|v| round.position(*v).unwrap())
            .collect();
        let n1 = (p[1] - p[0]).cross(p[2] - p[0]).normalize();
        let n2 = (p[2] - p[0]).cross(p[3] - p[0]).normalize();
        assert!(n1.dot(n2) > 0.999, "{p:?}");
    }
}

#[test]
fn n_gons_are_cut_into_triangles_first() {
    // A cylinder with flat many-sided caps.
    let cylinder = Primitive::Cylinder(CylinderParams {
        sides: 24,
        height_segments: 6,
        cap_centre: false,
        ..Default::default()
    })
    .mesh();
    assert!(cylinder.faces().any(|(_, c)| c.len() > 4));
    let out = decimate(&cylinder, &ratio(0.5), Hardness::Marked).unwrap();
    assert!(all_triangles(&out));
    assert_clean_solid(&out);
}

#[test]
fn zero_length_edges_go_first() {
    // Two triangles of a strip with a point doubled at one place.
    let input = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 10.0, 0.0],
            [0.0, 10.0, 0.0],
            [5.0, 5.0, 0.0],
            [5.0, 5.0, 0.0],
        ],
        &[&[0, 1, 4], &[1, 2, 5, 4], &[2, 3, 5], &[3, 0, 4, 5]],
    );
    let options = DecimateOptions {
        reduce_by: ReduceBy::Triangles,
        triangles: 4,
        ..Default::default()
    };
    let out = decimate(&input, &options, Hardness::Marked).unwrap();
    assert_eq!(triangle_count(&out), 4);
    // The doubled point is gone, the square's corners stay.
    assert_eq!(out.vertex_count(), 5);
    assert_eq!(out.bounds(), input.bounds());
}

#[test]
fn edges_of_three_polygons_are_kept() {
    // Three fins sharing one edge, each a 4 × 1 strip.
    let mut points = Vec::new();
    let mut faces: Vec<Vec<u32>> = Vec::new();
    for i in 0..5 {
        points.push([i as f32 * 10.0, 0.0, 0.0]);
    }
    for (fin, dir) in [[0.0, 10.0, 0.0], [0.0, -10.0, 0.0], [0.0, 0.0, 10.0]]
        .iter()
        .enumerate()
    {
        let base = points.len() as u32;
        for i in 0..5 {
            points.push([i as f32 * 10.0 + dir[0], dir[1], dir[2]]);
        }
        for i in 0..4u32 {
            let quad = vec![i, i + 1, base + i + 1, base + i];
            faces.push(if fin == 1 {
                quad.into_iter().rev().collect()
            } else {
                quad
            });
        }
    }
    let face_refs: Vec<&[u32]> = faces.iter().map(Vec::as_slice).collect();
    let input = mesh_of(&points, &face_refs);
    let out = decimate(&input, &ratio(0.1), Hardness::Marked).unwrap();
    // The shared edge's points stay (each lies on it).
    let on_spine = out
        .vertices()
        .filter(|(_, p)| p.y == 0.0 && p.z == 0.0)
        .count();
    assert_eq!(on_spine, 5);
    assert!(triangle_count(&out) < triangle_count(&input));
}

#[test]
fn empty_and_tiny_shapes_are_fine() {
    let empty = Mesh::new();
    assert_eq!(
        decimate(&empty, &ratio(0.5), Hardness::Marked).unwrap(),
        empty
    );
    let one = mesh_of(&[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], &[&[0, 1, 2]]);
    assert_eq!(decimate(&one, &ratio(0.01), Hardness::Marked).unwrap(), one);
    // Two triangles of a square: its four corners are a kept border's
    // corners, so they stay.
    let square = mesh_of(
        &[[0.0; 3], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
        &[&[0, 1, 2, 3]],
    );
    let out = decimate(&square, &ratio(0.01), Hardness::Marked).unwrap();
    assert_eq!(triangle_count(&out), 2);
    assert_eq!(out.vertex_count(), 4);
}

#[test]
fn loose_points_are_left_out() {
    let mut input = sphere(12, 6);
    input.add_vertex(Vec3::splat(1000.0)).unwrap();
    let out = decimate(&input, &ratio(0.5), Hardness::Marked).unwrap();
    assert_eq!(MeshReport::new(&out).loose_points, 0);
}

#[test]
fn points_not_at_real_positions_are_refused() {
    let mut input = sphere(12, 6);
    let v = input.vertices().next().unwrap().0;
    input.set_position(v, Vec3::new(f32::NAN, 0.0, 0.0));
    assert_eq!(
        decimate(&input, &ratio(0.5), Hardness::Marked),
        Err(DecimateProblem::NotANumber)
    );
    // A loose point that isn't a number doesn't matter.
    let mut loose = sphere(12, 6);
    loose.add_vertex(Vec3::splat(f32::INFINITY)).unwrap();
    assert!(decimate(&loose, &ratio(0.5), Hardness::Marked).is_ok());
}

#[test]
fn odd_settings_are_brought_within_limits_never_a_panic() {
    let o = DecimateOptions {
        ratio: f32::NAN,
        triangles: 0,
        ..Default::default()
    }
    .clamped();
    assert_eq!(o.ratio, DecimateOptions::STANDARD_RATIO);
    assert_eq!(o.triangles, DecimateOptions::MIN_TRIANGLES);
    assert_eq!(ratio(-3.0).clamped().ratio, DecimateOptions::MIN_RATIO);
    assert_eq!(
        ratio(f32::INFINITY).clamped().ratio,
        DecimateOptions::STANDARD_RATIO
    );
    assert_eq!(ratio(7.0).clamped().ratio, 1.0);
    assert_eq!(ratio(0.5).target(0), 0);
    assert_eq!(ratio(0.0001).target(100), 1);
    let input = sphere(16, 8);
    for options in [ratio(f32::NAN), ratio(-1.0), ratio(0.0), ratio(1e9)] {
        let out = decimate(&input, &options, Hardness::ByAngle(f32::NAN)).unwrap();
        assert!(out.vertices().all(|(_, p)| p.is_finite()));
    }
}

#[test]
fn the_smallest_ratio_goes_as_far_as_it_can_and_stays_whole() {
    let input = sphere(24, 12);
    let out = decimate(&input, &ratio(0.001), Hardness::Marked).unwrap();
    let report = MeshReport::new(&out);
    assert_eq!(report.non_manifold_edges, 0, "{report:?}");
    assert_eq!(report.flipped_edges, 0, "{report:?}");
    // The poles' fans of UVs and the seam down one side keep the most.
    assert!(triangle_count(&out) < 120, "{}", triangle_count(&out));
    // Without kept lines it goes much further.
    let options = DecimateOptions {
        ratio: 0.001,
        keep: Keep {
            seams: false,
            hard_edges: false,
            materials: false,
            borders: false,
        },
        ..Default::default()
    };
    let out = decimate(&input, &options, Hardness::Marked).unwrap();
    assert_clean_solid(&out);
    assert!(triangle_count(&out) < 30, "{}", triangle_count(&out));
}

#[test]
fn levels_of_detail_are_nested_and_match_decimating_alone() {
    let input = sphere(32, 16);
    let n = triangle_count(&input);
    let targets = [n / 2, n / 4, n / 8];
    let levels =
        decimate_levels(&input, Keep::default(), Hardness::Marked, &targets, false).unwrap();
    assert_eq!(levels.len(), 3);
    for pair in levels.windows(2) {
        let finer: HashSet<[u32; 3]> = places(&pair[0]).into_iter().collect();
        assert!(places(&pair[1]).iter().all(|p| finer.contains(p)));
        assert!(triangle_count(&pair[1]) < triangle_count(&pair[0]));
    }
    // A level is what decimating to it alone gives.
    let options = DecimateOptions {
        reduce_by: ReduceBy::Triangles,
        triangles: targets[2] as u32,
        ..Default::default()
    };
    assert_eq!(
        levels[2],
        decimate(&input, &options, Hardness::Marked).unwrap()
    );
}

#[test]
fn random_settings_and_shapes_never_give_a_broken_mesh() {
    let shapes = [
        sphere(20, 10),
        grid(8),
        Primitive::Cube(Default::default()).mesh(),
        Primitive::Cylinder(CylinderParams {
            sides: 12,
            height_segments: 3,
            ..Default::default()
        })
        .mesh(),
    ];
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for round in 0..60 {
        let shape = &shapes[round % shapes.len()];
        let bits = next();
        let options = DecimateOptions {
            reduce_by: if bits & 1 == 0 {
                ReduceBy::Ratio
            } else {
                ReduceBy::Triangles
            },
            ratio: (next() % 1000) as f32 / 1000.0,
            triangles: (next() % 300) as u32,
            keep: Keep {
                seams: bits & 2 != 0,
                hard_edges: bits & 4 != 0,
                materials: bits & 8 != 0,
                borders: bits & 16 != 0,
            },
            quads_where_flat: bits & 32 != 0,
        };
        let hardness = if bits & 64 == 0 {
            Hardness::Marked
        } else {
            Hardness::ByAngle(30.0)
        };
        let out = decimate(shape, &options, hardness).unwrap();
        let report = MeshReport::new(&out);
        assert_eq!(report.non_manifold_edges, 0, "{round}: {report:?}");
        assert_eq!(report.flipped_edges, 0, "{round}: {report:?}");
        assert!(out.vertices().all(|(_, p)| p.is_finite()));
    }
}

#[test]
#[ignore = "timing: run with --release -- --ignored --nocapture"]
fn timings() {
    let input = sphere(512, 100);
    let n = triangle_count(&input);
    for target in [n / 2, n / 10, n / 100] {
        let start = std::time::Instant::now();
        let options = DecimateOptions {
            reduce_by: ReduceBy::Triangles,
            triangles: target as u32,
            ..Default::default()
        };
        let out = decimate(&input, &options, Hardness::Marked).unwrap();
        println!(
            "sphere of {n} triangles to {}: {:?}",
            triangle_count(&out),
            start.elapsed()
        );
    }
    let start = std::time::Instant::now();
    let levels = decimate_levels(
        &input,
        Keep::default(),
        Hardness::Marked,
        &[n / 2, n / 4, n / 8],
        false,
    )
    .unwrap();
    println!(
        "three levels ({:?} triangles): {:?}",
        levels.iter().map(triangle_count).collect::<Vec<_>>(),
        start.elapsed()
    );
}
