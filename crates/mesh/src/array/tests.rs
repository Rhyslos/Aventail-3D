//! Tests for the array (decision 0153).

use super::*;
use crate::attributes::{AttributeValues, Domain, UV};
use crate::primitives::{CubeParams, PlaneParams};
use crate::{FaceId, MeshReport, Primitive, VertexId};

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

fn line(count: u32) -> ArrayOptions {
    ArrayOptions {
        count,
        ..ArrayOptions::default()
    }
}

fn radial(count: u32, angle: f32) -> ArrayOptions {
    ArrayOptions {
        layout: ArrayLayout::Radial,
        count,
        angle,
        ..ArrayOptions::default()
    }
}

fn bounds(mesh: &Mesh) -> Bounds {
    Bounds::around(mesh.vertices().map(|(_, p)| p)).unwrap()
}

fn all_uvs(mesh: &Mesh) -> Vec<Vec2> {
    let Some((Domain::Corner, _, AttributeValues::Vec2(uvs))) = mesh.attribute_values(UV) else {
        return Vec::new();
    };
    let (starts, _) = mesh.face_layout();
    mesh.faces()
        .flat_map(|(f, corners)| {
            let start = starts[f.get() as usize] as usize;
            uvs[start..start + corners.len()].to_vec()
        })
        .collect()
}

fn close(a: Vec3, b: Vec3) -> bool {
    a.distance(b) < 1e-3
}

#[test]
fn the_standard_is_three_in_a_line_touching() {
    let options = ArrayOptions::default();
    assert_eq!(options.layout, ArrayLayout::Line);
    assert_eq!(options.copies(), 3);
    assert!(options.relative);
    assert!(!options.merge);
    let out = array(&cube(), &options, Vec3::ZERO, MOST).unwrap();
    assert_eq!(out.face_count(), 18);
    assert_eq!(out.vertex_count(), 24);
    let b = bounds(&out);
    let c = bounds(&cube());
    assert!((b.size().x - 3.0 * c.size().x).abs() < 1e-3);
    assert!((b.size().y - c.size().y).abs() < 1e-3);
    assert!(close(b.min, c.min));
}

#[test]
fn the_first_copy_keeps_its_ids_and_places() {
    let input = cube();
    let out = array(&input, &line(4), Vec3::ZERO, MOST).unwrap();
    for (v, p) in input.vertices() {
        assert_eq!(out.position(v), Some(p));
    }
    for (f, corners) in input.faces() {
        assert_eq!(out.face(f), Some(corners));
    }
}

#[test]
fn absolute_offsets_are_distances() {
    let options = ArrayOptions {
        relative: false,
        offset: Vec3::new(0.0, 250.0, 10.0),
        ..line(3)
    };
    let input = cube();
    let out = array(&input, &options, Vec3::ZERO, MOST).unwrap();
    let b = bounds(&out);
    let c = bounds(&input);
    assert!(close(b.max, c.max + Vec3::new(0.0, 500.0, 20.0)));
    assert!(close(b.min, c.min));
}

#[test]
fn a_diagonal_relative_step_makes_stairs() {
    let options = ArrayOptions {
        offset: Vec3::new(0.0, 1.0, 1.0),
        ..line(5)
    };
    let out = array(&cube(), &options, Vec3::ZERO, MOST).unwrap();
    let size = bounds(&out).size();
    assert!(close(size, Vec3::new(100.0, 500.0, 500.0)));
}

#[test]
fn gaps_and_overlaps_follow_the_offset() {
    let gap = ArrayOptions {
        offset: Vec3::new(1.5, 0.0, 0.0),
        ..line(2)
    };
    let out = array(&cube(), &gap, Vec3::ZERO, MOST).unwrap();
    assert!((bounds(&out).size().x - 250.0).abs() < 1e-3);
    let back = ArrayOptions {
        offset: Vec3::new(-1.0, 0.0, 0.0),
        ..line(2)
    };
    let out = array(&cube(), &back, Vec3::ZERO, MOST).unwrap();
    assert!((bounds(&out).min.x - (bounds(&cube()).min.x - 100.0)).abs() < 1e-3);
}

#[test]
fn a_grid_counts_each_axis() {
    let options = ArrayOptions {
        layout: ArrayLayout::Grid,
        counts: [4, 3, 2],
        ..ArrayOptions::default()
    };
    assert_eq!(options.copies(), 24);
    let out = array(&cube(), &options, Vec3::ZERO, MOST).unwrap();
    assert_eq!(out.face_count(), 24 * 6);
    assert!(close(bounds(&out).size(), Vec3::new(400.0, 300.0, 200.0)));
    let matrices = copy_matrices(&options, Vec3::ZERO, Vec3::splat(100.0));
    // X first, then Y, then Z.
    assert!(close(
        matrices[1].transform_point3(Vec3::ZERO),
        Vec3::X * 100.0
    ));
    assert!(close(
        matrices[4].transform_point3(Vec3::ZERO),
        Vec3::Y * 100.0
    ));
    assert!(close(
        matrices[12].transform_point3(Vec3::ZERO),
        Vec3::Z * 100.0
    ));
}

#[test]
fn a_whole_ring_shares_the_turn_out() {
    let options = radial(4, 360.0);
    assert!((options.radial_step() - 90.0).abs() < 1e-4);
    let matrices = copy_matrices(&options, Vec3::ZERO, Vec3::ONE);
    let at = |i: usize| matrices[i].transform_point3(Vec3::X * 10.0);
    assert!(close(at(0), Vec3::X * 10.0));
    assert!(close(at(1), Vec3::Y * 10.0));
    assert!(close(at(2), -Vec3::X * 10.0));
    assert!(close(at(3), -Vec3::Y * 10.0));
}

#[test]
fn part_of_a_turn_is_a_fan_from_end_to_end() {
    let options = radial(3, 90.0);
    assert!((options.radial_step() - 45.0).abs() < 1e-4);
    let matrices = copy_matrices(&options, Vec3::ZERO, Vec3::ONE);
    let last = matrices[2].transform_point3(Vec3::X);
    assert!(close(last, Vec3::Y));
    // Backwards with a negative angle.
    let back = copy_matrices(&radial(2, -90.0), Vec3::ZERO, Vec3::ONE);
    assert!(close(back[1].transform_point3(Vec3::X), -Vec3::Y));
    // One copy: no turn.
    assert_eq!(radial(1, 90.0).radial_step(), 0.0);
}

#[test]
fn a_ring_turns_round_its_centre_and_axis() {
    let centre = Vec3::new(50.0, 0.0, 0.0);
    let options = ArrayOptions {
        axis: ArrayAxis::X,
        ..radial(2, 360.0)
    };
    let matrices = copy_matrices(&options, centre, Vec3::ONE);
    // Turned half way round X through the centre: Y and Z flip.
    let p = matrices[1].transform_point3(Vec3::new(0.0, 10.0, 20.0));
    assert!(close(p, Vec3::new(0.0, -10.0, -20.0)));
    let options = ArrayOptions {
        axis: ArrayAxis::Z,
        ..radial(2, 360.0)
    };
    let matrices = copy_matrices(&options, centre, Vec3::ONE);
    let p = matrices[1].transform_point3(Vec3::ZERO);
    assert!(close(p, Vec3::new(100.0, 0.0, 0.0)));
}

#[test]
fn the_radius_moves_each_copy_out_first() {
    let options = ArrayOptions {
        radius: 200.0,
        ..radial(4, 360.0)
    };
    let out = array(&cube(), &options, Vec3::ZERO, MOST).unwrap();
    assert_eq!(out.face_count(), 24);
    let b = bounds(&out);
    // The cube (100 wide, centred on its pivot) moved out 200 along X,
    // then turned a quarter at a time.
    assert!(close(b.max, Vec3::new(250.0, 250.0, 100.0)));
    assert!(close(b.min, Vec3::new(-250.0, -250.0, 0.0)));
    // Each axis moves out across it.
    assert_eq!(ArrayAxis::X.outward(), Vec3::Y);
    assert_eq!(ArrayAxis::Y.outward(), Vec3::X);
}

#[test]
fn one_copy_is_the_shape_unchanged() {
    let input = cube();
    assert_eq!(array(&input, &line(1), Vec3::ZERO, MOST).unwrap(), input);
    let grid = ArrayOptions {
        layout: ArrayLayout::Grid,
        counts: [1, 1, 1],
        ..ArrayOptions::default()
    };
    assert_eq!(array(&input, &grid, Vec3::ZERO, MOST).unwrap(), input);
    // A ring of one is the shape moved out by the radius.
    let ring = ArrayOptions {
        radius: 10.0,
        ..radial(1, 360.0)
    };
    let out = array(&input, &ring, Vec3::ZERO, MOST).unwrap();
    assert!(close(bounds(&out).min, bounds(&input).min + Vec3::X * 10.0));
}

#[test]
fn copies_keep_their_uvs() {
    let input = cube();
    let out = array(&input, &line(3), Vec3::ZERO, MOST).unwrap();
    let original = all_uvs(&input);
    let uvs = all_uvs(&out);
    assert_eq!(uvs.len(), original.len() * 3);
    for copy in uvs.chunks(original.len()) {
        assert_eq!(copy, original.as_slice());
    }
}

#[test]
fn uvs_side_by_side_give_each_copy_its_own_square() {
    let input = cube();
    let options = ArrayOptions {
        uvs_side_by_side: true,
        ..line(5)
    };
    let out = array(&input, &options, Vec3::ZERO, MOST).unwrap();
    let uvs = all_uvs(&out);
    let per = all_uvs(&input).len();
    let boxes: Vec<(Vec2, Vec2)> = uvs
        .chunks(per)
        .map(|c| {
            let min = c.iter().fold(Vec2::splat(f32::INFINITY), |m, v| m.min(*v));
            let max = c
                .iter()
                .fold(Vec2::splat(f32::NEG_INFINITY), |m, v| m.max(*v));
            (min, max)
        })
        .collect();
    assert_eq!(boxes.len(), 5);
    for (min, max) in &boxes {
        assert!(min.min_element() >= 0.0 && max.max_element() <= 1.0);
    }
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            let apart = a.1.x <= b.0.x || b.1.x <= a.0.x || a.1.y <= b.0.y || b.1.y <= a.0.y;
            assert!(apart, "{a:?} overlaps {b:?}");
        }
    }
    // The shape of each copy's UVs is kept (only shrunk and moved).
    let first = &uvs[..per];
    let original = all_uvs(&input);
    let scale = (first[1] - first[0]).length() / (original[1] - original[0]).length();
    for k in 2..per {
        let a = (first[k] - first[0]).length();
        let b = (original[k] - original[0]).length() * scale;
        assert!((a - b).abs() < 1e-4);
    }
}

#[test]
fn uvs_side_by_side_on_a_shape_without_uvs_is_harmless() {
    let mut input = mesh_of(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
        &[&[0, 1, 2]],
    );
    input.remove_attribute(UV);
    let options = ArrayOptions {
        uvs_side_by_side: true,
        ..line(3)
    };
    let out = array(&input, &options, Vec3::ZERO, MOST).unwrap();
    assert_eq!(out.face_count(), 3);
    assert!(all_uvs(&out).is_empty());
}

#[test]
fn merging_joins_open_copies_into_one_piece() {
    let input = plane(100.0);
    let apart = array(&input, &line(3), Vec3::ZERO, MOST).unwrap();
    assert_eq!(MeshReport::new(&apart).pieces, 3);
    assert_eq!(apart.vertex_count(), 12);
    let options = ArrayOptions {
        merge: true,
        ..line(3)
    };
    let out = array(&input, &options, Vec3::ZERO, MOST).unwrap();
    let report = MeshReport::new(&out);
    assert_eq!(report.pieces, 1);
    assert_eq!(out.vertex_count(), 8);
    assert_eq!(report.non_manifold_edges, 0);
    // The first copy's points stay put.
    for (v, p) in input.vertices() {
        assert_eq!(out.position(v), Some(p));
    }
}

#[test]
fn merging_leaves_closed_copies_closed() {
    let options = ArrayOptions {
        merge: true,
        ..line(3)
    };
    let out = array(&cube(), &options, Vec3::ZERO, MOST).unwrap();
    let report = MeshReport::new(&out);
    assert_eq!(out.vertex_count(), 24);
    assert_eq!(report.border_edges, 0);
    assert_eq!(report.non_manifold_edges, 0);
    assert_eq!(report.pieces, 3);
}

#[test]
fn merging_only_welds_within_the_distance() {
    let options = ArrayOptions {
        merge: true,
        merge_distance: 0.01,
        offset: Vec3::new(1.001, 0.0, 0.0),
        ..line(2)
    };
    // 0.1 cm apart: further than 0.1 mm.
    let out = array(&plane(100.0), &options, Vec3::ZERO, MOST).unwrap();
    assert_eq!(MeshReport::new(&out).pieces, 2);
    let wider = ArrayOptions {
        merge_distance: 0.2,
        ..options
    };
    let out = array(&plane(100.0), &wider, Vec3::ZERO, MOST).unwrap();
    assert_eq!(MeshReport::new(&out).pieces, 1);
}

#[test]
fn a_whole_ring_closes_its_last_copy_onto_its_first() {
    // A strip from the axis out along X: four of them round Z make a
    // cross; standing upright (a fin) the ring of four meets on the axis.
    let fin = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 0.0, 10.0],
            [0.0, 0.0, 10.0],
        ],
        &[&[0, 1, 2, 3]],
    );
    let options = ArrayOptions {
        merge: true,
        ..radial(4, 360.0)
    };
    let out = array(&fin, &options, Vec3::ZERO, MOST).unwrap();
    // The four fins share their inner edge.
    assert_eq!(out.vertex_count(), 2 + 4 * 2);
    assert_eq!(MeshReport::new(&out).pieces, 1);
}

#[test]
fn n_gons_zero_length_edges_and_open_borders_come_through() {
    let ngon = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [15.0, 5.0, 0.0],
            [10.0, 10.0, 0.0],
            [0.0, 10.0, 0.0],
            [0.0, 10.0, 0.0],
        ],
        &[&[0, 1, 2, 3, 4, 5]],
    );
    let options = ArrayOptions {
        merge: true,
        ..line(3)
    };
    let out = array(&ngon, &options, Vec3::ZERO, MOST).unwrap();
    assert_eq!(out.face_count(), 3);
    for (_, corners) in out.faces() {
        assert!(corners.len() >= 5);
    }
}

#[test]
fn non_manifold_input_is_copied_as_it_is() {
    // Three polygons on one edge.
    let fan = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 10.0],
            [10.0, 0.0, 0.0],
            [-10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
        ],
        &[&[0, 2, 1], &[0, 1, 3], &[1, 0, 4]],
    );
    assert_eq!(MeshReport::new(&fan).non_manifold_edges, 1);
    let out = array(&fan, &line(2), Vec3::ZERO, MOST).unwrap();
    assert_eq!(out.face_count(), 6);
    assert_eq!(MeshReport::new(&out).non_manifold_edges, 2);
}

#[test]
fn an_empty_shape_stays_empty() {
    let out = array(&Mesh::new(), &line(10), Vec3::ZERO, MOST).unwrap();
    assert_eq!(out.face_count(), 0);
    assert_eq!(work(&Mesh::new(), &line(10)), 0);
}

#[test]
fn a_flat_shape_with_relative_offsets_stacks_where_it_has_no_size() {
    let options = ArrayOptions {
        offset: Vec3::new(0.0, 0.0, 1.0),
        ..line(3)
    };
    // The plane has no height: its copies lie on top of each other.
    let out = array(&plane(10.0), &options, Vec3::ZERO, MOST).unwrap();
    assert_eq!(out.face_count(), 3);
    assert!(bounds(&out).size().z.abs() < 1e-6);
}

#[test]
fn settings_that_are_not_numbers_or_too_large_are_clamped() {
    let wild = ArrayOptions {
        count: 0,
        counts: [0, u32::MAX, 5],
        offset: Vec3::new(f32::NAN, f32::INFINITY, -1e30),
        spacing: Vec3::splat(f32::NAN),
        angle: f32::NAN,
        radius: f32::NEG_INFINITY,
        merge_distance: f32::NAN,
        ..ArrayOptions::default()
    }
    .clamped();
    assert_eq!(wild.count, 1);
    assert_eq!(wild.counts, [1, ArrayOptions::MAX_COUNT, 5]);
    assert_eq!(
        wild.offset,
        Vec3::new(1.0, 0.0, -ArrayOptions::MAX_DISTANCE)
    );
    assert_eq!(wild.spacing, Vec3::ONE);
    assert_eq!(wild.angle, 360.0);
    assert_eq!(wild.radius, 0.0);
    assert_eq!(wild.merge_distance, WeldOptions::STANDARD_DISTANCE);
    let turned = ArrayOptions {
        angle: 1000.0,
        merge_distance: 1e9,
        ..ArrayOptions::default()
    }
    .clamped();
    assert_eq!(turned.angle, 360.0);
    assert_eq!(turned.merge_distance, WeldOptions::MAX_DISTANCE);
    // Clamped settings never panic, whatever the centre.
    let options = ArrayOptions {
        layout: ArrayLayout::Radial,
        ..wild
    };
    let out = array(&cube(), &options, Vec3::splat(f32::NAN), MOST).unwrap();
    assert_eq!(out.face_count(), 6);
}

#[test]
fn too_many_copies_or_polygons_are_refused() {
    let grid = ArrayOptions {
        layout: ArrayLayout::Grid,
        counts: [1000, 1000, 1],
        ..ArrayOptions::default()
    };
    assert_eq!(
        array(&cube(), &grid, Vec3::ZERO, MOST),
        Err(ArrayProblem::TooManyCopies { copies: 1_000_000 })
    );
    assert!(
        ArrayProblem::TooManyCopies { copies: 1_000_000 }
            .to_string()
            .contains("1000000")
    );
    assert_eq!(
        array(&cube(), &line(10), Vec3::ZERO, 50),
        Err(ArrayProblem::TooManyPolygons {
            polygons: 60,
            most: 50
        })
    );
}

#[test]
fn the_same_settings_always_give_the_same_mesh() {
    let options = ArrayOptions {
        layout: ArrayLayout::Radial,
        count: 7,
        angle: 200.0,
        radius: 30.0,
        merge: true,
        uvs_side_by_side: true,
        ..ArrayOptions::default()
    };
    let a = array(&cube(), &options, Vec3::new(1.0, 2.0, 3.0), MOST).unwrap();
    let b = array(&cube(), &options, Vec3::new(1.0, 2.0, 3.0), MOST).unwrap();
    assert_eq!(a, b);
}

#[test]
fn work_grows_with_copies_and_merging() {
    let input = cube();
    let three = work(&input, &line(3));
    let thirty = work(&input, &line(300));
    assert!(thirty > three);
    let merged = work(
        &input,
        &ArrayOptions {
            merge: true,
            ..line(300)
        },
    );
    assert!(merged > thirty);
    // Refused at once: no work.
    let grid = ArrayOptions {
        layout: ArrayLayout::Grid,
        counts: [1000, 1000, 1],
        ..ArrayOptions::default()
    };
    assert_eq!(work(&input, &grid), 0);
}

#[test]
fn faces_of_copies_are_new_ids_after_the_shape() {
    let input = cube();
    let out = array(&input, &line(2), Vec3::ZERO, MOST).unwrap();
    let highest = input.faces().map(|(f, _)| f).max().unwrap();
    let new: Vec<FaceId> = out
        .faces()
        .map(|(f, _)| f)
        .filter(|f| input.face(*f).is_none())
        .collect();
    assert_eq!(new.len(), 6);
    assert!(new.iter().all(|f| *f > highest));
}

/// Timing (run with `--ignored --nocapture` in a release build).
#[test]
#[ignore = "timing"]
fn timing() {
    let sphere = Primitive::Sphere(Default::default()).mesh();
    for (count, merge) in [(100, false), (1000, false), (100, true), (1000, true)] {
        let options = ArrayOptions {
            count,
            merge,
            ..ArrayOptions::default()
        };
        let start = std::time::Instant::now();
        let out = array(&sphere, &options, Vec3::ZERO, MOST).unwrap();
        println!(
            "{count} copies of {} polygons, merge {merge}: {} polygons in {:?}",
            sphere.face_count(),
            out.face_count(),
            start.elapsed()
        );
    }
}
