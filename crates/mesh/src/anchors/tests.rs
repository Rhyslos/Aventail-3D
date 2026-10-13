//! Tests for finding an edit's elements again by position.

use super::*;
use crate::{CubeParams, PlaneParams, Primitive};

fn cube(size: Vec3, segments: u32) -> Mesh {
    Primitive::Cube(CubeParams {
        size,
        segments: [segments; 3],
    })
    .mesh()
}

const SIZE: Vec3 = Vec3::new(100.0, 100.0, 100.0);

/// The points of `mesh` within `filter`.
fn points_where(mesh: &Mesh, filter: impl Fn(Vec3) -> bool) -> Vec<VertexId> {
    mesh.vertices()
        .filter(|(_, p)| filter(*p))
        .map(|(v, _)| v)
        .collect()
}

/// The top polygon of a one-segment cube, as its corners.
fn top_polygon(mesh: &Mesh) -> Vec<VertexId> {
    mesh.faces()
        .find(|(f, _)| mesh.face_normal(*f).z > 0.9)
        .map(|(_, c)| c.to_vec())
        .unwrap()
}

#[test]
fn ids_still_fit_a_resized_shape_but_not_a_renumbered_one() {
    let small = cube(SIZE, 1);
    let anchors = Anchors::record(&small, ElementKind::Polygons, &[top_polygon(&small)]).unwrap();
    assert!(anchors.still_fit(&small));
    // Taller: the same points, in the same places relative to the box.
    assert!(anchors.still_fit(&cube(Vec3::new(100.0, 100.0, 250.0), 1)));
    // More segments: the IDs now name other points.
    assert!(!anchors.still_fit(&cube(SIZE, 2)));
    assert!(!anchors.still_fit(&Mesh::new()));
}

#[test]
fn a_polygon_finds_every_polygon_inside_it() {
    let small = cube(SIZE, 1);
    let anchors = Anchors::record(&small, ElementKind::Polygons, &[top_polygon(&small)]).unwrap();
    let finer = cube(SIZE, 2);
    let (found, missing) = anchors.find(&finer, &Topology::new(&finer));
    let top = points_where(&finer, |p| (p.z - 100.0).abs() < 1e-3);
    assert_eq!(missing, 0);
    assert_eq!(top.len(), 9, "a 3 × 3 grid of points on top");
    assert_eq!(found.len(), 9);
    assert!(top.iter().all(|v| found.contains(v)));
}

#[test]
fn an_edge_finds_the_edges_along_it() {
    let small = cube(SIZE, 1);
    let front_top: Vec<VertexId> = points_where(&small, |p| {
        (p.z - 100.0).abs() < 1e-3 && (p.y + 50.0).abs() < 1e-3
    });
    assert_eq!(front_top.len(), 2);
    let anchors = Anchors::record(&small, ElementKind::Edges, &[front_top]).unwrap();
    let finer = cube(Vec3::new(200.0, 100.0, 100.0), 3);
    let (found, missing) = anchors.find(&finer, &Topology::new(&finer));
    assert_eq!(missing, 0);
    assert_eq!(
        found.len(),
        4,
        "the line, now in three pieces, on the wider cube"
    );
    for v in &found {
        let p = finer.position(*v).unwrap();
        assert!(
            (p.z - 100.0).abs() < 1e-3 && (p.y + 50.0).abs() < 1e-3,
            "{p}"
        );
    }
}

#[test]
fn a_point_finds_the_point_in_its_place() {
    let small = cube(SIZE, 1);
    let corner = points_where(&small, |p| p == Vec3::new(50.0, -50.0, 100.0));
    let anchors = Anchors::record(&small, ElementKind::Points, &[corner]).unwrap();
    let finer = cube(SIZE, 4);
    let (found, missing) = anchors.find(&finer, &Topology::new(&finer));
    assert_eq!(missing, 0);
    assert_eq!(found.len(), 1);
    assert_eq!(
        finer.position(found[0]),
        Some(Vec3::new(50.0, -50.0, 100.0))
    );
}

#[test]
fn elements_with_no_place_left_are_counted_missing() {
    // A point in the middle of the top of a two-segment cube has no place
    // on a one-segment cube.
    let finer = cube(SIZE, 2);
    let middle = points_where(&finer, |p| p == Vec3::new(0.0, 0.0, 100.0));
    let anchors = Anchors::record(&finer, ElementKind::Points, &[middle]).unwrap();
    let coarse = cube(SIZE, 1);
    let (found, missing) = anchors.find(&coarse, &Topology::new(&coarse));
    assert!(found.is_empty());
    assert_eq!(missing, 1);
    assert_eq!(anchors.element_count(), 1);
    // An empty shape has nothing at all.
    let (found, missing) = anchors.find(&Mesh::new(), &Topology::new(&Mesh::new()));
    assert!(found.is_empty() && missing == 1);
}

#[test]
fn flat_shapes_keep_their_height() {
    // A plane has no height: its box is flat, which must not divide by
    // zero.
    let plane = |segments| {
        Primitive::Plane(PlaneParams {
            width: 100.0,
            depth: 100.0,
            segments: [segments, segments],
        })
        .mesh()
    };
    let one = plane(1);
    let face = one.faces().next().unwrap().1.to_vec();
    let anchors = Anchors::record(&one, ElementKind::Polygons, &[face]).unwrap();
    assert!(anchors.still_fit(&one));
    let finer = plane(3);
    let (found, missing) = anchors.find(&finer, &Topology::new(&finer));
    assert_eq!(missing, 0);
    assert_eq!(found.len(), 16, "every point of the 3 × 3 plane");
    assert!(anchors.memory_size() > 0);
    assert!(Anchors::record(&Mesh::new(), ElementKind::Points, &[]).is_none());
}

/// A long strip of narrow polygons: each is narrower than the shape-wide
/// tolerance, as on a very dense mesh.
fn narrow_strip() -> Mesh {
    let mut mesh = Mesh::new();
    let columns: Vec<[VertexId; 2]> = (0..=2000)
        .map(|i| {
            let x = i as f32 * 0.5 - 500.0;
            [
                mesh.add_vertex(Vec3::new(x, 0.0, 0.0)).unwrap(),
                mesh.add_vertex(Vec3::new(x, 1.0, 0.0)).unwrap(),
            ]
        })
        .collect();
    for pair in columns.windows(2) {
        mesh.add_face(&[pair[0][0], pair[1][0], pair[1][1], pair[0][1]])
            .unwrap();
    }
    mesh
}

#[test]
fn on_a_dense_mesh_a_polygon_finds_only_itself() {
    let strip = narrow_strip();
    let middle = strip
        .faces()
        .find(|(_, c)| {
            let x: f32 = c.iter().map(|v| strip.position(*v).unwrap().x).sum::<f32>() / 4.0;
            x.abs() < 0.3
        })
        .map(|(_, c)| c.to_vec())
        .unwrap();
    let anchors =
        Anchors::record(&strip, ElementKind::Polygons, std::slice::from_ref(&middle)).unwrap();
    let (found, missing) = anchors.find(&strip, &Topology::new(&strip));
    assert_eq!(missing, 0);
    let mut expected = middle;
    expected.sort();
    assert_eq!(found, expected, "not its neighbours too");
}

#[test]
fn on_a_dense_mesh_an_edge_finds_only_itself() {
    let strip = narrow_strip();
    let topology = Topology::new(&strip);
    let edge = topology
        .edges()
        .find(|e| {
            let (a, b) = e.ends();
            let (pa, pb) = (strip.position(a).unwrap(), strip.position(b).unwrap());
            (pa.x - pb.x).abs() > 0.1 && pa.x.abs() < 0.3 && pa.y > 0.0
        })
        .unwrap();
    let (a, b) = edge.ends();
    let anchors = Anchors::record(&strip, ElementKind::Edges, &[vec![a, b]]).unwrap();
    let (found, missing) = anchors.find(&strip, &topology);
    assert_eq!(missing, 0);
    assert_eq!(found, vec![a.min(b), a.max(b)]);
}

#[test]
fn every_point_of_a_dense_mesh_is_found_in_its_own_place() {
    let strip = narrow_strip();
    let all: Vec<VertexId> = strip.vertices().map(|(v, _)| v).collect();
    let anchors = Anchors::record(&strip, ElementKind::Points, std::slice::from_ref(&all)).unwrap();
    let (found, missing) = anchors.find(&strip, &Topology::new(&strip));
    assert_eq!(missing, 0);
    assert_eq!(found, all);
}

#[test]
fn the_structure_fingerprint_follows_faces_and_point_slots_only() {
    let a = cube(SIZE, 1);
    let mut moved = a.clone();
    let v = moved.vertices().next().unwrap().0;
    moved.set_position(v, Vec3::splat(999.0));
    assert_eq!(a.structure_fingerprint(), moved.structure_fingerprint());
    assert_eq!(
        a.structure_fingerprint(),
        cube(Vec3::new(10.0, 20.0, 30.0), 1).structure_fingerprint(),
        "a resized cube has the same structure"
    );
    assert_ne!(
        a.structure_fingerprint(),
        cube(SIZE, 2).structure_fingerprint()
    );
    let mut more = a.clone();
    more.add_vertex(Vec3::ZERO).unwrap();
    assert_ne!(a.structure_fingerprint(), more.structure_fingerprint());
    let mut fewer = a.clone();
    fewer.delete_faces(&[FaceId::from_raw(0)]);
    assert_ne!(a.structure_fingerprint(), fewer.structure_fingerprint());
    // It is saved in project files, so it must never change between
    // versions of the app: this value is fixed.
    assert_eq!(Mesh::new().structure_fingerprint(), 0xdfeb_b0bf_8d1d_4a20);
    assert_eq!(a.structure_fingerprint(), 0xeac8_5ef3_7e0f_d7e8);
}

#[test]
fn anchors_know_the_structure_they_were_made_on() {
    let small = cube(SIZE, 1);
    let anchors = Anchors::record(&small, ElementKind::Polygons, &[top_polygon(&small)]).unwrap();
    assert!(anchors.same_structure(&small));
    assert!(anchors.same_structure(&cube(Vec3::new(10.0, 20.0, 30.0), 1)));
    assert!(!anchors.same_structure(&cube(SIZE, 2)));
}

#[test]
fn the_reach_holds_every_edge_from_a_moved_point_once() {
    let small = cube(SIZE, 1);
    let top = top_polygon(&small);
    let anchors = Anchors::record(&small, ElementKind::Polygons, &[top])
        .unwrap()
        .with_reach(&small);
    // Four edges round the top (between moved points, once each), and four
    // down to points that stayed.
    let between = anchors
        .reach
        .iter()
        .filter(|r| matches!(r.to, ReachEnd::Moved(_)))
        .count();
    let down = anchors
        .reach
        .iter()
        .filter(|r| matches!(r.to, ReachEnd::Still(p) if p.z.abs() < 1e-3))
        .count();
    assert_eq!((between, down, anchors.reach.len()), (4, 4, 8));
    assert!(
        anchors.memory_size()
            > Anchors::record(&small, ElementKind::Polygons, &[top_polygon(&small)])
                .unwrap()
                .memory_size()
    );
}

#[test]
fn new_points_along_the_reach_get_their_share() {
    let small = cube(SIZE, 1);
    let top = top_polygon(&small);
    let anchors = Anchors::record(&small, ElementKind::Polygons, std::slice::from_ref(&top))
        .unwrap()
        .with_reach(&small);
    let tall = cube(SIZE, 4);
    let found: Vec<VertexId> = points_where(&tall, |p| p.z > 99.0);
    let shares = anchors.stretch(&tall, &found);
    // The new rows on the four upright edges: a quarter, half, three
    // quarters of the way up.
    assert_eq!(shares.len(), 12);
    for (v, share) in &shares {
        let z = tall.position(*v).unwrap().z;
        assert!((share - z / 100.0).abs() < 1e-4, "at {z}: {share}");
    }
    // New points along the top's own edges move all the way.
    let wide = cube(SIZE, 2);
    let top_points = points_where(&wide, |p| p.z > 99.0);
    let corners: Vec<VertexId> = top_points
        .iter()
        .copied()
        .filter(|v| {
            let p = wide.position(*v).unwrap();
            p.x.abs() > 49.0 && p.y.abs() > 49.0
        })
        .collect();
    let shares = anchors.stretch(&wide, &corners);
    let on_top: Vec<f32> = shares
        .iter()
        .filter(|(v, _)| wide.position(*v).unwrap().z > 99.0)
        .map(|(_, s)| *s)
        .collect();
    assert_eq!(on_top.len(), 4, "the four edge middles, not the centre");
    assert!(on_top.iter().all(|s| (s - 1.0).abs() < 1e-6));
    // Without a reach, or on an empty mesh, nothing stretches.
    let plain = Anchors::record(&small, ElementKind::Polygons, &[top]).unwrap();
    assert!(plain.stretch(&tall, &found).is_empty());
    assert!(anchors.stretch(&Mesh::new(), &[]).is_empty());
}
