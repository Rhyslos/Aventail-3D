//! Tests for Weld by distance (decision 0152).

use super::*;
use crate::attributes::{AttributeValues, Domain, UV, Value};
use crate::primitives::PlaneParams;
use crate::{FaceId, MeshReport, Primitive};
use glam::{Mat4, Vec2};

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

fn v(i: u32) -> VertexId {
    VertexId::from_raw(i)
}

fn standard() -> WeldOptions {
    WeldOptions::default()
}

fn within(distance: f32) -> WeldOptions {
    WeldOptions {
        distance,
        ..WeldOptions::default()
    }
}

/// Two unit squares side by side, each with its own points: the right
/// edge of the first (points 1 and 2) on the left edge of the second
/// (points 4 and 7), `gap` apart along X.
fn two_squares(gap: f32) -> Mesh {
    mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0 + gap, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [1.0 + gap, 1.0, 0.0],
        ],
        &[&[0, 1, 2, 3], &[4, 5, 6, 7]],
    )
}

/// A cube 1 wide whose six sides each have their own four points (as a
/// shape split along every edge comes from a game file).
fn split_cube() -> Mesh {
    let corners = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
    ];
    let sides: [[usize; 4]; 6] = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ];
    let mut points = Vec::new();
    let mut faces: Vec<Vec<u32>> = Vec::new();
    for side in sides {
        let first = points.len() as u32;
        for c in side {
            points.push(corners[c]);
        }
        faces.push((first..first + 4).collect());
    }
    let faces: Vec<&[u32]> = faces.iter().map(Vec::as_slice).collect();
    mesh_of(&points, &faces)
}

fn plane(width: f32, segments: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        width,
        depth: width,
        segments: [segments, segments],
    })
    .mesh()
}

/// Each polygon's corner UVs, in its corners' order.
fn corner_uvs(mesh: &Mesh) -> Vec<(FaceId, Vec<Vec2>)> {
    let Some((Domain::Corner, _, AttributeValues::Vec2(uvs))) = mesh.attribute_values(UV) else {
        return Vec::new();
    };
    let (starts, _) = mesh.face_layout();
    mesh.faces()
        .map(|(f, corners)| {
            let start = starts[f.get() as usize] as usize;
            (f, uvs[start..start + corners.len()].to_vec())
        })
        .collect()
}

#[test]
fn two_squares_touching_become_one_piece() {
    let mut mesh = two_squares(0.0);
    let went = weld_by_distance(&mut mesh, None, &standard());
    assert_eq!(went, 2);
    assert_eq!((mesh.vertex_count(), mesh.face_count()), (6, 2));
    // The lower ids stay; the polygons keep theirs.
    assert!(mesh.position(v(1)).is_some() && mesh.position(v(4)).is_none());
    assert_eq!(
        mesh.face(FaceId::from_raw(1)).unwrap(),
        &[v(1), v(5), v(6), v(2)]
    );
    // The edge between them is shared now.
    let shared = EdgeKey::new(v(1), v(2));
    assert_eq!(mesh.topology().edge_uses(shared).len(), 2);
    assert_eq!(mesh.topology().border_edges().len(), 6);
}

#[test]
fn only_points_within_the_distance_weld() {
    let mut close = two_squares(0.005);
    assert_eq!(weld_by_distance(&mut close, None, &standard()), 2);
    let mut apart = two_squares(0.02);
    assert_eq!(weld_by_distance(&mut apart, None, &standard()), 0);
    assert_eq!(apart, two_squares(0.02));
    assert_eq!(weld_by_distance(&mut apart, None, &within(0.02)), 2);
}

#[test]
fn the_point_that_stays_keeps_its_place() {
    let mut mesh = two_squares(0.005);
    weld_by_distance(&mut mesh, None, &standard());
    assert_eq!(mesh.position(v(1)), Some(Vec3::new(1.0, 0.0, 0.0)));
    assert_eq!(mesh.position(v(2)), Some(Vec3::new(1.0, 1.0, 0.0)));
}

#[test]
fn points_are_never_pulled_along_a_chain() {
    // Three points in a row 0.008 apart: the first takes the second, but
    // the third is 0.016 from the first and stays.
    let mesh = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [0.008, 0.0, 0.0],
            [0.016, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        &[&[0, 1, 2, 3]],
    );
    let groups = weld_groups_of(&mesh, None, &standard());
    assert_eq!(groups, vec![vec![v(0), v(1)]]);
    let mut welded = mesh.clone();
    assert_eq!(weld_by_distance(&mut welded, None, &standard()), 1);
    assert_eq!(
        welded.face(FaceId::from_raw(0)).unwrap(),
        &[v(0), v(2), v(3)]
    );
}

#[test]
fn a_cube_split_along_every_edge_closes_into_a_solid() {
    let mut mesh = split_cube();
    assert!(!MeshReport::new(&mesh).is_closed());
    assert_eq!(weld_by_distance(&mut mesh, None, &standard()), 16);
    assert_eq!((mesh.vertex_count(), mesh.face_count()), (8, 6));
    let report = MeshReport::new(&mesh);
    assert!(report.is_closed(), "{report:?}");
    assert!(mesh.topology().non_manifold_edges().is_empty());
    assert!(mesh.topology().flipped_edges().is_empty());
}

#[test]
fn only_at_open_borders_leaves_the_inside_alone() {
    // A 2 × 2 grid with its middle point moved next to a border point.
    let mut mesh = plane(100.0, 2);
    let middle = mesh
        .vertices()
        .find(|(_, p)| p.x.abs() < 1e-3 && p.y.abs() < 1e-3)
        .map(|(v, _)| v)
        .unwrap();
    let border = mesh
        .vertices()
        .find(|(_, p)| (p.x - 50.0).abs() < 1e-3 && p.y.abs() < 1e-3)
        .map(|(v, _)| v)
        .unwrap();
    mesh.set_position(middle, Vec3::new(49.995, 0.0, 0.0));
    let borders = WeldOptions {
        only_borders: true,
        ..standard()
    };
    assert_eq!(weld_by_distance(&mut mesh.clone(), None, &borders), 0);
    assert_eq!(welded(&mesh, &borders), Err(WeldProblem::NothingClose));
    let mut anywhere = mesh.clone();
    assert_eq!(weld_by_distance(&mut anywhere, None, &standard()), 1);
    assert!(anywhere.position(middle).is_none() || anywhere.position(border).is_none());
    // Two squares' touching edges are borders: they weld either way.
    let mut squares = two_squares(0.0);
    assert_eq!(weld_by_distance(&mut squares, None, &borders), 2);
}

#[test]
fn a_closed_shape_has_no_borders_to_weld() {
    let cube = Primitive::Cube(Default::default()).mesh();
    let borders = WeldOptions {
        only_borders: true,
        ..standard()
    };
    assert_eq!(welded(&cube, &borders), Err(WeldProblem::NoBorders));
    assert_eq!(welded(&cube, &standard()), Err(WeldProblem::NothingClose));
    assert!(!WeldProblem::NoBorders.to_string().is_empty());
}

#[test]
fn only_the_given_points_weld() {
    let mesh = two_squares(0.0);
    let mut some = mesh.clone();
    assert_eq!(
        weld_by_distance(&mut some, Some(&[v(1), v(4)]), &standard()),
        1
    );
    assert!(some.position(v(4)).is_none() && some.position(v(7)).is_some());
    // One point alone has nothing to weld with; missing points are left out.
    let mut one = mesh.clone();
    assert_eq!(
        weld_by_distance(&mut one, Some(&[v(1), v(99)]), &standard()),
        0
    );
    assert_eq!(one, mesh);
    assert_eq!(weld_by_distance(&mut one, Some(&[]), &standard()), 0);
}

#[test]
fn a_zero_length_edge_goes_and_its_polygon_loses_a_corner() {
    // A quad with two corners on one spot, beside a triangle using one.
    let mut mesh = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
        ],
        &[&[0, 1, 2, 3], &[1, 4, 2]],
    );
    assert_eq!(weld_by_distance(&mut mesh, None, &standard()), 1);
    // The quad is a triangle now; the sliver triangle folded away.
    assert_eq!(mesh.face(FaceId::from_raw(0)).unwrap(), &[v(0), v(1), v(3)]);
    assert!(mesh.face(FaceId::from_raw(1)).is_none());
    assert!(mesh.position(v(4)).is_none(), "its lone point goes too");
}

#[test]
fn a_tiny_polygon_melts_away() {
    let mut mesh = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [0.001, 0.0, 0.0],
            [0.0, 0.001, 0.0],
            [5.0, 0.0, 0.0],
            [5.0, 5.0, 0.0],
        ],
        &[&[0, 1, 2], &[1, 3, 4, 2]],
    );
    assert_eq!(weld_by_distance(&mut mesh, None, &standard()), 2);
    assert_eq!(mesh.face_count(), 1);
    assert_eq!(mesh.face(FaceId::from_raw(1)).unwrap(), &[v(0), v(3), v(4)]);
}

#[test]
fn n_gons_and_non_manifold_edges_are_handled() {
    // Two pentagons side by side with their own points, and a third
    // polygon on their shared edge.
    let mut mesh = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.5, 1.5, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [1.5, 1.5, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 0.5, 1.0],
        ],
        &[&[0, 1, 2, 3, 4], &[5, 6, 7, 8, 9], &[1, 10, 2]],
    );
    assert_eq!(weld_by_distance(&mut mesh, None, &standard()), 2);
    assert_eq!(mesh.face(FaceId::from_raw(1)).unwrap().len(), 5);
    assert_eq!(mesh.topology().non_manifold_edges().len(), 1);
    // Welding again finds nothing more.
    assert_eq!(weld_by_distance(&mut mesh, None, &standard()), 0);
}

#[test]
fn empty_and_odd_meshes_weld_nothing() {
    let mut empty = Mesh::new();
    assert_eq!(weld_by_distance(&mut empty, None, &standard()), 0);
    assert_eq!(welded(&empty, &standard()), Err(WeldProblem::NothingClose));
    // A point not at a real place is never welded.
    let mut odd = two_squares(0.0);
    odd.set_position(v(4), Vec3::NAN);
    odd.set_position(v(1), Vec3::splat(f32::INFINITY));
    assert_eq!(weld_by_distance(&mut odd, None, &standard()), 1);
    assert!(odd.position(v(4)).is_some());
    // Points far out still find their neighbours.
    let mut far = two_squares(0.0);
    let away = Mat4::from_translation(Vec3::splat(1.0e5));
    for (id, p) in far.clone().vertices() {
        far.set_position(id, away.transform_point3(p));
    }
    assert_eq!(weld_by_distance(&mut far, None, &within(0.001)), 2);
    // Beyond what the cells can count: no panic.
    let mut beyond = two_squares(0.0);
    for (id, p) in beyond.clone().vertices() {
        beyond.set_position(id, p + Vec3::splat(1.0e30));
    }
    weld_by_distance(&mut beyond, None, &within(0.0));
}

#[test]
fn settings_are_brought_within_limits() {
    let nan = WeldOptions {
        distance: f32::NAN,
        ..standard()
    };
    assert_eq!(nan.clamped().distance, WeldOptions::STANDARD_DISTANCE);
    assert_eq!(within(-1.0).clamped().distance, 0.0);
    assert_eq!(within(1e9).clamped().distance, WeldOptions::MAX_DISTANCE);
    assert_eq!(
        within(f32::INFINITY).clamped().distance,
        WeldOptions::STANDARD_DISTANCE
    );
    // No distance: only points exactly on one spot.
    let mut exact = two_squares(0.0);
    assert_eq!(weld_by_distance(&mut exact, None, &within(0.0)), 2);
    let mut near = two_squares(1e-4);
    assert_eq!(weld_by_distance(&mut near, None, &within(-5.0)), 0);
    // A huge distance melts everything into a point, without a panic.
    let mut grid = plane(50.0, 100);
    let went = weld_by_distance(&mut grid, None, &within(1e9));
    assert_eq!(went, 101 * 101 - 1);
    assert_eq!(grid.face_count(), 0);
    assert_eq!(grid.vertex_count(), 0);
}

#[test]
fn joined_pieces_keep_their_uvs_and_get_a_seam() {
    // Two planes with their own UVs (0–1 each), touching along X.
    let one = plane(100.0, 1);
    assert!(one.attribute_values(SEAM).is_some());
    let both = crate::parts::combine(&[
        (&one, Mat4::IDENTITY),
        (&one, Mat4::from_translation(Vec3::X * 100.0)),
    ]);
    let before = corner_uvs(&both);
    assert!(!before.is_empty());
    let mut kept = both.clone();
    assert_eq!(weld_by_distance(&mut kept, None, &standard()), 2);
    // Every corner keeps its own UV.
    assert_eq!(corner_uvs(&kept), before);
    // The joining edge splits the UVs, and is marked a seam.
    let splits = crate::uvs::uv_splits(&kept);
    assert_eq!(splits.len(), 1);
    assert_eq!(crate::uvs::seams(&kept), splits);
    // Off, it is left unmarked.
    let mut unmarked = both.clone();
    let off = WeldOptions {
        keep_seams: false,
        ..standard()
    };
    weld_by_distance(&mut unmarked, None, &off);
    assert!(crate::uvs::seams(&unmarked).is_empty());
    assert_eq!(crate::uvs::uv_splits(&unmarked).len(), 1);
    // A shape with no seam marks at all gets none: Unwrap cuts it where
    // its UVs split anyway.
    let mut bare = both.clone();
    bare.remove_attribute(SEAM);
    weld_by_distance(&mut bare, None, &standard());
    assert!(bare.attribute_values(SEAM).is_none());
}

#[test]
fn pieces_whose_uvs_meet_get_no_seam() {
    // Two squares with their own points, their UVs meeting where they
    // touch, and seam marks (none yet).
    let mut mesh = two_squares(0.0);
    let uvs: Vec<Vec2> = mesh
        .faces()
        .flat_map(|(_, corners)| corners.to_vec())
        .map(|c| mesh.position(c).unwrap().truncate() / 2.0)
        .collect();
    assert!(crate::uvs::set_uvs(&mut mesh, uvs));
    mesh.add_attribute(SEAM, Domain::Edge, Value::Bool(false))
        .unwrap();
    let before = corner_uvs(&mesh);
    weld_by_distance(&mut mesh, None, &standard());
    assert!(crate::uvs::uv_splits(&mesh).is_empty());
    assert!(crate::uvs::seams(&mesh).is_empty());
    assert_eq!(corner_uvs(&mesh), before);
}

#[test]
fn the_result_is_always_the_same() {
    let mesh = split_cube();
    let mut a = mesh.clone();
    let mut b = mesh.clone();
    weld_by_distance(&mut a, None, &standard());
    weld_by_distance(&mut b, None, &standard());
    assert_eq!(a, b);
    assert_eq!(
        weld_groups_of(&mesh, None, &standard()),
        weld_groups_of(&mesh, None, &standard())
    );
}

/// A grid of `n` × `n` unit squares, each with its own four points.
fn exploded_grid(n: u32) -> Mesh {
    let mut points = Vec::new();
    let mut faces: Vec<Vec<u32>> = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let (x, y) = (i as f32, j as f32);
            let first = points.len() as u32;
            points.extend([
                [x, y, 0.0],
                [x + 1.0, y, 0.0],
                [x + 1.0, y + 1.0, 0.0],
                [x, y + 1.0, 0.0],
            ]);
            faces.push((first..first + 4).collect());
        }
    }
    let faces: Vec<&[u32]> = faces.iter().map(Vec::as_slice).collect();
    mesh_of(&points, &faces)
}

#[test]
fn a_big_exploded_grid_welds_back_into_one_quickly() {
    let n = 150;
    let mut mesh = exploded_grid(n);
    let started = std::time::Instant::now();
    let went = weld_by_distance(&mut mesh, None, &standard());
    let took = started.elapsed();
    assert_eq!(mesh.vertex_count(), ((n + 1) * (n + 1)) as usize);
    assert_eq!(went, (4 * n * n - (n + 1) * (n + 1)) as usize);
    assert_eq!(mesh.topology().border_edges().len(), (4 * n) as usize);
    eprintln!("welded {went} of {} points in {took:?}", 4 * n * n);
    // 90 000 points: well under a second even in a debug build.
    assert!(took.as_secs_f32() < 10.0, "{took:?}");
}
