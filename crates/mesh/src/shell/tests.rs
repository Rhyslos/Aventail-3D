//! Tests for the shell (decision 0154).

use super::offsets::{MAX_STRETCH, offsets};
use super::*;
use crate::attributes::{AttributeValues, Domain, UV};
use crate::material::{set_material, slot_of};
use crate::primitives::{CubeParams, CylinderParams, PlaneParams};
use crate::uvs::{has_uvs, is_seam};
use crate::{EdgeKey, MeshReport, Primitive};
use glam::{Vec2, Vec3};

const MOST: usize = 30_000_000;

fn cube() -> Mesh {
    Primitive::Cube(CubeParams::default()).mesh()
}

fn plane(width: f32) -> Mesh {
    Primitive::Plane(PlaneParams {
        width,
        depth: width,
        segments: [1, 1],
    })
    .mesh()
}

/// An open pipe: a cylinder with no ends.
fn pipe() -> Mesh {
    Primitive::Cylinder(CylinderParams {
        caps: false,
        ..CylinderParams::default()
    })
    .mesh()
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

fn with(thickness: f32, direction: ShellDirection) -> ShellOptions {
    ShellOptions {
        thickness,
        direction,
        ..ShellOptions::default()
    }
}

/// Every point of `mesh` that `input` didn't have.
fn new_points(input: &Mesh, mesh: &Mesh) -> Vec<Vec3> {
    mesh.vertices()
        .filter(|(v, _)| input.position(*v).is_none())
        .map(|(_, p)| p)
        .collect()
}

/// A closed, clean surface: no open borders, no edge of three
/// polygons, none facing the wrong way.
fn assert_clean_and_closed(mesh: &Mesh) {
    let report = MeshReport::new(mesh);
    assert_eq!(report.border_edges, 0, "{report:?}");
    assert_eq!(report.non_manifold_edges, 0, "{report:?}");
    assert_eq!(report.flipped_edges, 0, "{report:?}");
    assert_eq!(report.flat_faces, 0, "{report:?}");
}

fn all_finite(mesh: &Mesh) -> bool {
    mesh.vertices().all(|(_, p)| p.is_finite())
}

fn uv_values(mesh: &Mesh) -> Vec<Vec2> {
    match mesh.attribute_values(UV) {
        Some((Domain::Corner, _, AttributeValues::Vec2(list))) => list,
        _ => Vec::new(),
    }
}

#[test]
fn the_standard_is_one_centimetre_inward_even_with_rims() {
    let options = ShellOptions::default();
    assert_eq!(options.thickness, 1.0);
    assert_eq!(options.direction, ShellDirection::Inward);
    assert!(options.even);
    assert!(options.fill_rims);
    assert_eq!(ShellDirection::ALL.len(), 3);
}

#[test]
fn settings_are_clamped_and_never_not_numbers() {
    let clamp = |thickness: f32| {
        ShellOptions {
            thickness,
            ..ShellOptions::default()
        }
        .clamped()
        .thickness
    };
    assert_eq!(clamp(f32::NAN), ShellOptions::STANDARD_THICKNESS);
    assert_eq!(clamp(f32::INFINITY), ShellOptions::STANDARD_THICKNESS);
    assert_eq!(clamp(-3.0), 3.0);
    assert_eq!(clamp(1e9), ShellOptions::MAX_THICKNESS);
    assert_eq!(clamp(0.0), 0.0);
}

#[test]
fn a_plane_becomes_a_closed_slab() {
    let input = plane(100.0);
    let out = shell(&input, &ShellOptions::default(), MOST).unwrap();
    // The plane, its copy behind and four rims.
    assert_eq!(out.face_count(), 6);
    assert_eq!(out.vertex_count(), 8);
    assert_clean_and_closed(&out);
    // Inward: the plane stays where it was, keeping its ids.
    for (v, p) in input.vertices() {
        assert_eq!(out.position(v), Some(p));
    }
    for (f, corners) in input.faces() {
        assert_eq!(out.face(f), Some(corners));
    }
    // The copy is 1 cm behind (the plane faces up).
    for p in new_points(&input, &out) {
        assert!((p.z - (-1.0)).abs() < 1e-5, "{p}");
    }
}

#[test]
fn outward_and_both_sides_move_the_layers_the_other_ways() {
    let input = plane(100.0);
    let out = shell(&input, &with(2.0, ShellDirection::Outward), MOST).unwrap();
    for (v, _) in input.vertices() {
        assert!((out.position(v).unwrap().z - 2.0).abs() < 1e-5);
    }
    for p in new_points(&input, &out) {
        assert!(p.z.abs() < 1e-5);
    }
    let out = shell(&input, &with(2.0, ShellDirection::Both), MOST).unwrap();
    for (v, _) in input.vertices() {
        assert!((out.position(v).unwrap().z - 1.0).abs() < 1e-5);
    }
    for p in new_points(&input, &out) {
        assert!((p.z + 1.0).abs() < 1e-5);
    }
    assert_clean_and_closed(&out);
}

#[test]
fn a_closed_cube_gets_a_cube_inside_it_at_an_even_thickness() {
    let input = cube();
    let size = input.bounds().unwrap();
    let out = shell(&input, &with(5.0, ShellDirection::Inward), MOST).unwrap();
    // No rims on a closed shape: the cube and the one inside.
    assert_eq!(out.face_count(), input.face_count() * 2);
    assert_clean_and_closed(&out);
    assert_eq!(MeshReport::new(&out).pieces, 2);
    // Even: every side of the inner cube is 5 cm in from the outer's.
    for p in new_points(&input, &out) {
        for axis in 0..3 {
            let (low, high) = (size.min[axis], size.max[axis]);
            let gap = (p[axis] - low).min(high - p[axis]);
            assert!((gap - 5.0).abs() < 1e-3, "{p} along {axis}: {gap}");
        }
    }
}

#[test]
fn without_even_thickness_corners_come_out_thinner() {
    let input = cube();
    let size = input.bounds().unwrap();
    let options = ShellOptions {
        even: false,
        ..with(5.0, ShellDirection::Inward)
    };
    let out = shell(&input, &options, MOST).unwrap();
    // A corner moves 5 cm along its diagonal: 5 / √3 in from each side.
    let p = new_points(&input, &out)[0];
    let gap = (p.x - size.min.x).min(size.max.x - p.x);
    assert!((gap - 5.0 / 3f32.sqrt()).abs() < 1e-3, "{gap}");
}

#[test]
fn an_open_pipe_gets_two_rims_with_hard_edges() {
    let input = pipe();
    let borders = input.topology().border_edges().len();
    assert!(borders > 0);
    let out = shell(&input, &ShellOptions::default(), MOST).unwrap();
    assert_eq!(out.face_count(), input.face_count() * 2 + borders);
    assert_clean_and_closed(&out);
    // Each border edge, and its copy behind, is hard.
    for edge in input.topology().border_edges() {
        let (a, b) = edge.ends();
        assert!(out.is_hard(a, b), "{edge:?}");
    }
    assert!(out.hard_edge_count() >= borders * 2);
}

#[test]
fn rims_get_their_own_uvs_above_the_texture_square_with_seams() {
    let input = pipe();
    assert!(has_uvs(&input));
    let before = uv_values(&input);
    let out = shell(&input, &ShellOptions::default(), MOST).unwrap();
    let after = uv_values(&out);
    // The surface keeps its UVs.
    assert_eq!(&after[..before.len()], &before[..]);
    // The rims' are above the square, in two rows (two borders).
    let (starts, _) = out.face_layout();
    let rims: Vec<FaceId> = out
        .faces()
        .map(|(f, _)| f)
        .filter(|f| f.get() as usize >= input.face_count() * 2)
        .collect();
    assert_eq!(rims.len(), input.topology().border_edges().len());
    let mut rows = std::collections::BTreeSet::new();
    for f in &rims {
        let start = starts[f.get() as usize] as usize;
        let uvs = &after[start..start + 4];
        assert!(uvs.iter().all(|uv| uv.y > 1.0 && uv.x >= 0.0), "{uvs:?}");
        rows.insert((uvs[2].y * 1000.0).round() as i64);
        // A rim's row is as tall as the shell is thick, at the shape's
        // texel density: never flat.
        assert!(uvs[1].y > uvs[2].y);
        assert!(uvs[0].x > uvs[1].x);
    }
    assert_eq!(rows.len(), 2);
    // Seams along each rim, front and behind.
    for edge in input.topology().border_edges() {
        let (a, b) = edge.ends();
        assert!(is_seam(&out, EdgeKey::new(a, b)));
    }
    // And one cut across each closed border's strip.
    let across: usize = out
        .faces()
        .filter(|(f, _)| rims.contains(f))
        .map(|(_, c)| {
            usize::from(is_seam(&out, EdgeKey::new(c[1], c[2])))
                + usize::from(is_seam(&out, EdgeKey::new(c[0], c[3])))
        })
        .sum();
    // Each cut edge is counted by its two rims.
    assert_eq!(across, 2 * 2);
}

#[test]
fn a_shape_without_uvs_gets_none() {
    let input = mesh_of(
        &[[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0]],
        &[&[0, 1, 2]],
    );
    let out = shell(&input, &ShellOptions::default(), MOST).unwrap();
    assert!(!has_uvs(&out));
    assert_eq!(out.face_count(), 5);
    assert_clean_and_closed(&out);
}

#[test]
fn without_rims_the_two_layers_stay_open() {
    let input = plane(100.0);
    let options = ShellOptions {
        fill_rims: false,
        ..ShellOptions::default()
    };
    let out = shell(&input, &options, MOST).unwrap();
    assert_eq!(out.face_count(), 2);
    let report = MeshReport::new(&out);
    assert_eq!(report.pieces, 2);
    assert_eq!(report.border_edges, 8);
    assert_eq!(out.hard_edge_count(), input.hard_edge_count() * 2);
}

#[test]
fn the_layer_behind_faces_the_other_way() {
    let input = plane(100.0);
    let out = shell(&input, &ShellOptions::default(), MOST).unwrap();
    let behind = out.faces().map(|(f, _)| f).find(|f| f.get() == 1).unwrap();
    assert!(out.face_normal(behind).z < -0.99);
}

#[test]
fn rims_and_the_layer_behind_keep_materials() {
    let mut input = plane(100.0);
    let face = input.faces().next().unwrap().0;
    set_material(&mut input, &[face], 3);
    assert_eq!(slot_of(&input, face), 3);
    let out = shell(&input, &ShellOptions::default(), MOST).unwrap();
    assert_eq!(out.face_count(), 6);
    assert!(out.faces().all(|(f, _)| slot_of(&out, f) == 3));
}

#[test]
fn n_gons_zero_length_edges_and_folds_never_panic() {
    // A pentagon.
    let pentagon = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [13.0, 8.0, 0.0],
            [5.0, 13.0, 0.0],
            [-3.0, 8.0, 0.0],
        ],
        &[&[0, 1, 2, 3, 4]],
    );
    let out = shell(&pentagon, &ShellOptions::default(), MOST).unwrap();
    assert_eq!(out.face_count(), 2 + 5);
    assert_clean_and_closed(&out);
    // A zero-length edge: two points on one spot.
    let squashed = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 10.0, 0.0],
        ],
        &[&[0, 1, 2, 3]],
    );
    let out = shell(&squashed, &ShellOptions::default(), MOST).unwrap();
    assert!(all_finite(&out));
    // A polygon with no area at all.
    let flat = mesh_of(
        &[[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [20.0, 0.0, 0.0]],
        &[&[0, 1, 2]],
    );
    let out = shell(&flat, &ShellOptions::default(), MOST).unwrap();
    assert!(all_finite(&out));
    // Non-manifold: three polygons on one edge.
    let fins = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 10.0],
            [10.0, 0.0, 0.0],
            [-10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
        ],
        &[&[0, 1, 2], &[1, 0, 3], &[0, 1, 4]],
    );
    let out = shell(&fins, &ShellOptions::default(), MOST).unwrap();
    assert!(all_finite(&out));
    // Two polygons folded flat onto each other: their facings cancel.
    let folded = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [10.0, 10.0, 0.0],
        ],
        &[&[0, 1, 2], &[1, 0, 2]],
    );
    let out = shell(&folded, &ShellOptions::default(), MOST).unwrap();
    assert!(all_finite(&out));
    // Neighbours facing opposite ways (a flipped polygon).
    let flipped = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 10.0, 0.0],
            [0.0, 10.0, 0.0],
        ],
        &[&[0, 1, 2], &[0, 3, 2]],
    );
    let out = shell(&flipped, &ShellOptions::default(), MOST).unwrap();
    assert!(all_finite(&out));
}

#[test]
fn a_thin_spike_is_held_back() {
    // Two triangles meeting at a 4° edge: an even offset there would be
    // 1 / sin(2°), about 29 times the thickness.
    let angle = 2f32.to_radians();
    let input = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [100.0 * angle.cos(), 5.0, 100.0 * angle.sin()],
            [100.0 * angle.cos(), 5.0, -100.0 * angle.sin()],
        ],
        &[&[0, 1, 2], &[1, 0, 3]],
    );
    for (_, offset) in offsets(&input, true) {
        assert!(offset.length() <= MAX_STRETCH + 1e-4, "{offset}");
    }
}

#[test]
fn even_offsets_meet_each_side_at_the_thickness() {
    // A box's edge: the two sides at a right angle.
    let input = cube();
    for (v, offset) in offsets(&input, true) {
        let topology = input.topology();
        for f in topology.vertex_faces(v) {
            let n = input.face_normal(*f);
            assert!((offset.dot(n) - 1.0).abs() < 1e-3, "{offset} {n}");
        }
    }
}

#[test]
fn nothing_to_do_gives_the_shape_back() {
    let input = cube();
    let zero = with(0.0, ShellDirection::Inward);
    assert_eq!(shell(&input, &zero, MOST).unwrap(), input);
    let empty = Mesh::new();
    assert_eq!(
        shell(&empty, &ShellOptions::default(), MOST).unwrap(),
        empty
    );
    // Loose points alone: no polygons, nothing to thicken.
    let loose = mesh_of(&[[0.0, 0.0, 0.0]], &[]);
    assert_eq!(
        shell(&loose, &ShellOptions::default(), MOST).unwrap(),
        loose
    );
}

#[test]
fn huge_and_not_a_number_settings_never_panic() {
    let input = pipe();
    for thickness in [f32::NAN, f32::INFINITY, -1e30, 1e30, f32::MIN_POSITIVE] {
        let options = ShellOptions {
            thickness,
            ..ShellOptions::default()
        };
        let out = shell(&input, &options, MOST).unwrap();
        assert!(all_finite(&out), "{thickness}");
    }
}

#[test]
fn too_many_polygons_are_refused() {
    let input = pipe();
    let most = input.face_count();
    let why = shell(&input, &ShellOptions::default(), most).unwrap_err();
    assert!(matches!(why, ShellProblem::TooManyPolygons { .. }));
    assert!(why.to_string().contains("polygons"));
}

#[test]
fn it_is_the_same_every_time() {
    let input = pipe();
    let options = with(3.0, ShellDirection::Both);
    let first = shell(&input, &options, MOST).unwrap();
    for _ in 0..3 {
        assert_eq!(shell(&input, &options, MOST).unwrap(), first);
    }
}

#[test]
fn its_work_grows_with_the_polygons() {
    let small = work(&plane(10.0), &ShellOptions::default());
    let big = Primitive::Sphere(crate::primitives::SphereParams::default()).mesh();
    assert!(work(&big, &ShellOptions::default()) > small);
    assert_eq!(polygon_count(&plane(10.0), &ShellOptions::default()), 2 + 4);
}

#[test]
fn creases_and_seams_come_along_to_the_layer_behind() {
    let input = cube();
    let seams = crate::uvs::seams(&input);
    let out = shell(&input, &ShellOptions::default(), MOST).unwrap();
    let new_seams = crate::uvs::seams(&out).len();
    assert_eq!(new_seams, seams.len() * 2);
    assert_eq!(out.hard_edge_count(), input.hard_edge_count() * 2);
}

/// Timing (run with `--ignored --nocapture` in a release build).
#[test]
#[ignore = "timing"]
fn timing() {
    for segments in [10u32, 50, 100, 300] {
        let input = Primitive::Plane(PlaneParams {
            width: 100.0,
            depth: 100.0,
            segments: [segments, segments],
        })
        .mesh();
        let start = std::time::Instant::now();
        let out = shell(&input, &ShellOptions::default(), MOST).unwrap();
        println!(
            "a plane of {} polygons: {} polygons in {:?} (work {})",
            input.face_count(),
            out.face_count(),
            start.elapsed(),
            work(&input, &ShellOptions::default())
        );
    }
}
