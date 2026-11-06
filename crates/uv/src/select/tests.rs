//! Tests for UV selection and copying (decision 0239).

use super::*;
use mesh::{CubeParams, PlaneParams, Primitive};

fn plane(n: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        segments: [n, n],
        ..Default::default()
    })
    .mesh()
}

fn faces(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

/// Moves polygon `f`'s UVs by `by`.
fn shift(mesh: &mut Mesh, f: FaceId, by: Vec2) {
    let uvs = Uvs::of(mesh).unwrap();
    let (starts, _) = mesh.face_layout();
    let s = starts[f.get() as usize] as usize;
    let corners: Vec<Corner> = mesh
        .face(f)
        .unwrap()
        .iter()
        .enumerate()
        .map(|(k, v)| (f, *v, uvs.at(s + k) + by))
        .collect();
    mesh::uvs::set_corner_uvs(mesh, &corners);
}

#[test]
fn overlapping_and_mirrored_pieces_are_found() {
    let mut mesh = plane(2);
    let fs = faces(&mesh);
    assert!(overlapping(&mesh).is_empty());
    assert!(mirrored(&mesh).is_empty());
    // Lay one polygon over another.
    let uvs = Uvs::of(&mesh).unwrap();
    let (starts, _) = mesh.face_layout();
    let a = uvs.at(starts[fs[0].get() as usize] as usize);
    let b = uvs.at(starts[fs[1].get() as usize] as usize);
    shift(&mut mesh, fs[1], a - b);
    assert!(!overlapping(&mesh).is_empty());
}

#[test]
fn stretched_polygons_are_found_by_how_far_off() {
    let mut mesh = plane(2);
    assert!(stretched(&mesh, 1.5).is_empty());
    // Squash one polygon's UVs flat one way.
    let f = faces(&mesh)[0];
    let uvs = Uvs::of(&mesh).unwrap();
    let (starts, _) = mesh.face_layout();
    let s = starts[f.get() as usize] as usize;
    let corners: Vec<Corner> = mesh
        .face(f)
        .unwrap()
        .iter()
        .enumerate()
        .map(|(k, v)| (f, *v, uvs.at(s + k) * Vec2::new(1.0, 0.1)))
        .collect();
    mesh::uvs::set_corner_uvs(&mut mesh, &corners);
    assert!(stretched(&mesh, 1.5).contains(&f));
}

#[test]
fn similar_pieces_match_by_make_up_and_size() {
    // A cube's six sides are alike.
    let cube = Primitive::Cube(CubeParams::default()).mesh();
    let pieces = mesh::uvs::islands(&cube);
    let first = &pieces[0];
    let like = similar(&cube, &first[..1], 0.02);
    assert!(like.len() >= first.len());
    // Nothing selected, nothing similar.
    assert!(similar(&cube, &[], 0.02).is_empty());
}

#[test]
fn growing_stops_at_seams_and_shrinking_trims_the_rim() {
    let mesh = plane(3);
    let fs = faces(&mesh);
    // The middle polygon of the 3 × 3 grow to all nine, shrink back.
    let middle = fs[4];
    let grown = grow(&mesh, &[middle]);
    assert_eq!(grown.len(), 9);
    // Shrinking: the shape's open edge is the rim, so only the middle
    // stays.
    assert_eq!(shrink(&mesh, &grown), vec![middle]);
    // A cut-off polygon isn't grown into.
    let mut cut = plane(3);
    shift(&mut cut, fs[5], Vec2::new(0.5, 0.0));
    assert!(!grow(&cut, &[middle]).contains(&fs[5]));
}

#[test]
fn the_shortest_path_runs_along_joined_edges() {
    let mesh = plane(2);
    let corners: Vec<(VertexId, glam::Vec3)> = mesh.vertices().collect();
    let low = corners
        .iter()
        .min_by(|a, b| (a.1.x + a.1.y).total_cmp(&(b.1.x + b.1.y)))
        .unwrap()
        .0;
    let high = corners
        .iter()
        .max_by(|a, b| (a.1.x + a.1.y).total_cmp(&(b.1.x + b.1.y)))
        .unwrap()
        .0;
    let path = shortest_path(&mesh, low, high).unwrap();
    assert_eq!(path.len(), 4, "two across and two up on a 2 × 2 grid");
    assert_eq!(shortest_path(&mesh, low, low), Some(Vec::new()));
    assert_eq!(shortest_path(&mesh, low, VertexId::from_raw(999)), None);
}

#[test]
fn copying_a_layout_onto_a_matching_piece_keeps_its_place() {
    let cube = Primitive::Cube(CubeParams::default()).mesh();
    let pieces = mesh::uvs::islands(&cube);
    assert!(pieces.len() >= 2);
    let (corners, count) = copy_layout(&cube, &pieces[0], &pieces[1..2]);
    assert_eq!(count, 1);
    assert_eq!(corners.len(), 4 * pieces[1].len());
    let mut copied = cube.clone();
    mesh::uvs::set_corner_uvs(&mut copied, &corners);
    // The target's box middle stayed.
    let before = mesh::uvs::uv_bounds(&cube, &pieces[1], &[]).unwrap();
    let after = mesh::uvs::uv_bounds(&copied, &pieces[1], &[]).unwrap();
    let mid = |(a, b): (Vec2, Vec2)| (a + b) / 2.0;
    assert!(mid(before).distance(mid(after)) < 1e-4);
    // A piece of another make-up is skipped.
    let mesh = plane(2);
    let all = faces(&mesh);
    let (none, count) = copy_layout(&mesh, &all[..1], std::slice::from_ref(&all));
    assert!(none.is_empty() && count == 0);
}
