//! Tests for loop cut.

use super::*;
use crate::attributes::{AttributeElement, Domain, HARD, Value};
use crate::{CylinderParams, MeshReport, PlaneParams, Primitive};
use glam::{Vec2, Vec3};

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

fn plane(x: u32, y: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        segments: [x, y],
        ..Default::default()
    })
    .mesh()
}

fn cylinder(sides: u32, cap_centre: bool) -> Mesh {
    Primitive::Cylinder(CylinderParams {
        sides,
        height_segments: 1,
        caps: true,
        cap_segments: 1,
        cap_centre,
        ..Default::default()
    })
    .mesh()
}

/// The first edge whose two ends both pass `test`.
fn edge_where(mesh: &Mesh, test: impl Fn(Vec3, Vec3) -> bool) -> EdgeKey {
    *mesh
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            test(mesh.position(a).unwrap(), mesh.position(b).unwrap())
        })
        .expect("no such edge")
}

/// The cube's edge along its top front (along X, at y = -50, z = 100).
fn top_front(mesh: &Mesh) -> EdgeKey {
    edge_where(mesh, |a, b| {
        a.y < -49.0 && b.y < -49.0 && a.z > 99.0 && b.z > 99.0
    })
}

/// Checks nothing went wrong: no flipped polygons, flat polygons, loose
/// points or edges shared by three polygons, every point a number, and
/// the surface as closed as it was.
fn clean_like(mesh: &Mesh, before: &Mesh) {
    let (r, b) = (MeshReport::new(mesh), MeshReport::new(before));
    assert!(!r.has_problems(), "{r:?}");
    assert_eq!(r.is_closed(), b.is_closed(), "{r:?}");
    assert_eq!(r.pieces, b.pieces, "{r:?}");
    for (v, p) in mesh.vertices() {
        assert!(p.is_finite(), "point {v:?} is {p}");
    }
}

fn new_points(mesh: &Mesh, before: &Mesh) -> Vec<Vec3> {
    mesh.vertices()
        .filter(|(v, _)| before.position(*v).is_none())
        .map(|(_, p)| p)
        .collect()
}

const DONE: Outcome = Outcome {
    matched: 1,
    missing: 0,
    skipped: 0,
};

// ------------------------------------------------------------- shares ---

#[test]
fn cuts_are_spread_evenly_and_slid_together() {
    let near = |a: &[f32], b: &[f32]| {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5)
    };
    assert!(near(&shares(1, 0.5), &[0.5]));
    assert!(near(&shares(2, 0.5), &[1.0 / 3.0, 2.0 / 3.0]));
    assert!(near(&shares(3, 0.5), &[0.25, 0.5, 0.75]));
    assert!(near(&shares(1, 0.2), &[0.2]));
    // Slid together, and kept inside the edge.
    assert!(near(&shares(2, 0.6), &[1.0 / 3.0 + 0.1, 2.0 / 3.0 + 0.1]));
    assert!(near(&shares(1, 0.0), &[EDGE_MARGIN]));
    assert!(near(&shares(1, 1.0), &[1.0 - EDGE_MARGIN]));
    assert!(near(
        &shares(3, 0.0),
        &[EDGE_MARGIN, 0.25 + EDGE_MARGIN, 0.5 + EDGE_MARGIN]
    ));
    // Not a number: the middle. No cuts: one. Too many: the most.
    assert!(near(&shares(1, f32::NAN), &[0.5]));
    assert!(near(&shares(1, f32::INFINITY), &[0.5]));
    assert!(near(&shares(0, 0.5), &[0.5]));
    assert_eq!(shares(10_000, 0.5).len(), MAX_CUTS as usize);
}

// ------------------------------------------------------- normal cases ---

#[test]
fn a_cut_runs_all_the_way_round_a_cube() {
    let before = cube();
    let mut mesh = before.clone();
    let out = loop_cut(&mut mesh, top_front(&before), 1, 0.5);
    assert_eq!(out, DONE);
    // Four polygons cut in two: four new polygons, four new points.
    assert_eq!(mesh.face_count(), 10);
    assert_eq!(mesh.vertex_count(), 12);
    clean_like(&mesh, &before);
    // Halfway along X, all round.
    let points = new_points(&mesh, &before);
    assert_eq!(points.len(), 4);
    assert!(points.iter().all(|p| p.x.abs() < 1e-4), "{points:?}");
    // The two side polygons it doesn't reach are untouched.
    let untouched = mesh
        .faces()
        .filter(|(f, c)| before.face(*f) == Some(c))
        .count();
    assert_eq!(untouched, 2);
}

#[test]
fn polygons_keep_their_ids_and_new_ones_come_next() {
    let before = cube();
    let mut mesh = before.clone();
    loop_cut(&mut mesh, top_front(&before), 2, 0.5);
    for (f, _) in before.faces() {
        assert!(mesh.face(f).is_some(), "{f:?} lost");
    }
    let ids: Vec<u32> = mesh.faces().map(|(f, _)| f.get()).collect();
    assert_eq!(ids, (0..14).collect::<Vec<_>>());
    let points: Vec<u32> = mesh.vertices().map(|(v, _)| v.get()).collect();
    assert_eq!(points, (0..16).collect::<Vec<_>>());
    // Doing it again gives the same mesh, ids and all.
    let mut again = before.clone();
    loop_cut(&mut again, top_front(&before), 2, 0.5);
    assert_eq!(mesh, again);
}

#[test]
fn several_cuts_are_spread_evenly() {
    let before = cube();
    let mut mesh = before.clone();
    assert_eq!(loop_cut(&mut mesh, top_front(&before), 3, 0.5), DONE);
    assert_eq!(mesh.face_count(), 6 + 4 * 3);
    assert_eq!(mesh.vertex_count(), 8 + 4 * 3);
    clean_like(&mesh, &before);
    let mut xs: Vec<i32> = new_points(&mesh, &before)
        .iter()
        .map(|p| p.x.round() as i32)
        .collect();
    xs.sort_unstable();
    xs.dedup();
    assert_eq!(xs, vec![-25, 0, 25]);
}

#[test]
fn the_position_slides_the_loop_and_keeps_it_straight() {
    let before = cube();
    let edge = top_front(&before);
    let (a, _) = edge.ends();
    let from = before.position(a).unwrap().x;
    let mut mesh = before.clone();
    loop_cut(&mut mesh, edge, 1, 0.25);
    clean_like(&mesh, &before);
    let points = new_points(&mesh, &before);
    // A quarter of the way from the picked edge's lower point, on every
    // edge of the ring: one straight loop.
    let want = from + (0.0 - from) * 0.5;
    assert!(
        points.iter().all(|p| (p.x - want).abs() < 1e-3),
        "{points:?}, want x = {want}"
    );
}

#[test]
fn an_open_ring_runs_from_border_to_border() {
    let before = plane(3, 3);
    let mut mesh = before.clone();
    // A middle edge along X.
    let edge = edge_where(&before, |a, b| {
        a.y.abs() < 20.0 && b.y.abs() < 20.0 && a.x.abs() < 20.0 && b.x.abs() < 20.0
    });
    assert_eq!(loop_cut(&mut mesh, edge, 1, 0.5), DONE);
    // Three polygons cut, four edges split.
    assert_eq!(mesh.face_count(), 12);
    assert_eq!(mesh.vertex_count(), 20);
    clean_like(&mesh, &before);
    let (r, b) = (MeshReport::new(&mesh), MeshReport::new(&before));
    assert_eq!(r.border_edges, b.border_edges + 2);
}

#[test]
fn a_cut_runs_round_a_cylinder_and_leaves_the_caps_whole() {
    let before = cylinder(16, false);
    let mut mesh = before.clone();
    let side = edge_where(&before, |a, b| (a.z - b.z).abs() > 1.0);
    assert_eq!(loop_cut(&mut mesh, side, 1, 0.5), DONE);
    assert_eq!(mesh.face_count(), before.face_count() + 16);
    assert_eq!(mesh.vertex_count(), before.vertex_count() + 16);
    clean_like(&mesh, &before);
    // The caps keep their corners.
    let caps = before.faces().filter(|(_, c)| c.len() == 16);
    for (f, c) in caps {
        assert_eq!(mesh.face(f), Some(c));
    }
}

#[test]
fn the_preview_matches_the_cut() {
    let before = cube();
    let edge = top_front(&before);
    assert_eq!(ring_polygons(&before, edge), 4);
    let lines = preview(&before, edge, 2, 0.4);
    assert_eq!(lines.len(), 8);
    let mut mesh = before.clone();
    loop_cut(&mut mesh, edge, 2, 0.4);
    let points = new_points(&mesh, &before);
    for [a, b] in &lines {
        for end in [a, b] {
            assert!(points.iter().any(|p| p.distance(*end) < 1e-4), "{end}");
        }
    }
    let nowhere = EdgeKey::new(VertexId::from_raw(100), VertexId::from_raw(200));
    assert!(preview(&before, nowhere, 1, 0.5).is_empty());
    assert_eq!(ring_polygons(&before, nowhere), 0);
}

// ----------------------------------------------------------- ring ends ---

#[test]
fn many_sided_polygons_at_the_ends_take_the_new_points() {
    let before = cylinder(8, false);
    let mut mesh = before.clone();
    // A rim edge: the ring crosses one side polygon and ends at both caps.
    let rim = edge_where(&before, |a, b| a.z > 1.0 && b.z > 1.0);
    assert_eq!(loop_cut(&mut mesh, rim, 1, 0.5), DONE);
    assert_eq!(mesh.face_count(), before.face_count() + 1);
    assert_eq!(mesh.vertex_count(), before.vertex_count() + 2);
    clean_like(&mesh, &before);
    // Each cap now has nine corners.
    assert_eq!(mesh.faces().filter(|(_, c)| c.len() == 9).count(), 2);
}

#[test]
fn triangles_at_the_ends_take_the_new_points() {
    let before = cylinder(8, true);
    let mut mesh = before.clone();
    let rim = edge_where(&before, |a, b| {
        a.z > 1.0 && b.z > 1.0 && a.length() > 1.0 && b.truncate().length() > 1.0
    });
    assert_eq!(loop_cut(&mut mesh, rim, 2, 0.5), DONE);
    clean_like(&mesh, &before);
    // Two triangles became five-sided polygons.
    assert_eq!(mesh.faces().filter(|(_, c)| c.len() == 5).count(), 2);
}

#[test]
fn a_ring_meeting_itself_is_cut_once() {
    // A one-polygon-wide strip bent round into a closed band of four: each
    // polygon's ring is the whole band.
    let before = Primitive::Tube(Default::default()).mesh();
    let mut mesh = before.clone();
    for edge in before.edges() {
        let mut m = before.clone();
        loop_cut(&mut m, *edge, 1, 0.5);
        clean_like(&m, &before);
    }
    let edge = before.edges()[0];
    loop_cut(&mut mesh, edge, 1, 0.5);
    let r = MeshReport::new(&mesh);
    assert_eq!(r.flipped_edges, 0);
}

// ----------------------------------------------------- what is refused ---

#[test]
fn an_edge_not_in_the_mesh_is_missing() {
    let before = cube();
    let mut mesh = before.clone();
    let nowhere = EdgeKey::new(VertexId::from_raw(100), VertexId::from_raw(200));
    let out = loop_cut(&mut mesh, nowhere, 1, 0.5);
    assert_eq!(out.missing, 1);
    assert_eq!(mesh, before);
    // Two points of the cube that share no edge (opposite corners).
    let across = EdgeKey::new(VertexId::from_raw(0), VertexId::from_raw(6));
    assert!(!before.topology().has_edge(across));
    assert_eq!(loop_cut(&mut mesh, across, 1, 0.5).missing, 1);
    assert_eq!(mesh, before);
}

#[test]
fn an_edge_with_no_four_sided_polygon_is_skipped() {
    let mut before = Mesh::new();
    let v = [
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ]
    .map(|p| before.add_vertex(p).unwrap());
    before.add_face(&v).unwrap();
    let mut mesh = before.clone();
    let out = loop_cut(&mut mesh, EdgeKey::new(v[0], v[1]), 1, 0.5);
    assert_eq!(out.skipped, 1);
    assert_eq!(mesh, before);
}

#[test]
fn an_edge_shared_by_three_polygons_is_skipped() {
    let (before, shared, _) = fan();
    let mut mesh = before.clone();
    assert_eq!(loop_cut(&mut mesh, shared, 1, 0.5).skipped, 1);
    assert_eq!(mesh, before);
}

#[test]
fn a_ring_reaching_an_edge_shared_by_three_polygons_stops_there() {
    let (before, _, near) = fan();
    let mut mesh = before.clone();
    assert_eq!(loop_cut(&mut mesh, near, 1, 0.5), DONE);
    // The first polygon is cut; the three beyond take the new point.
    assert_eq!(mesh.face_count(), before.face_count() + 1);
    assert_eq!(mesh.vertex_count(), before.vertex_count() + 2);
    let r = MeshReport::new(&mesh);
    assert_eq!(r.flipped_edges, 0);
    assert_eq!(r.loose_points, 0);
    assert_eq!(r.flat_faces, 0);
    assert_eq!(mesh.faces().filter(|(_, c)| c.len() == 5).count(), 3);
}

/// A square, beyond whose far edge three squares share that edge (one flat,
/// one up, one down). Returns the mesh, the shared edge, and the square's
/// near edge (opposite the shared one).
fn fan() -> (Mesh, EdgeKey, EdgeKey) {
    let mut mesh = Mesh::new();
    let mut at = |x: f32, y: f32, z: f32| mesh.add_vertex(Vec3::new(x, y, z)).unwrap();
    let (n0, n1) = (at(0.0, 0.0, 0.0), at(1.0, 0.0, 0.0));
    let (s0, s1) = (at(0.0, 1.0, 0.0), at(1.0, 1.0, 0.0));
    let (f0, f1) = (at(0.0, 2.0, 0.0), at(1.0, 2.0, 0.0));
    let (u0, u1) = (at(0.0, 1.0, 1.0), at(1.0, 1.0, 1.0));
    let (d0, d1) = (at(0.0, 1.0, -1.0), at(1.0, 1.0, -1.0));
    mesh.add_face(&[n0, n1, s1, s0]).unwrap();
    mesh.add_face(&[s0, s1, f1, f0]).unwrap();
    mesh.add_face(&[s0, s1, u1, u0]).unwrap();
    mesh.add_face(&[d0, d1, s1, s0]).unwrap();
    (mesh, EdgeKey::new(s0, s1), EdgeKey::new(n0, n1))
}

// -------------------------------------------------- attribute values ---

#[test]
fn values_come_along() {
    let mut before = plane(1, 1);
    before
        .add_attribute("material", Domain::Face, Value::Int(0))
        .unwrap();
    before
        .add_attribute("uv", Domain::Corner, Value::Vec2(Vec2::ZERO))
        .unwrap();
    before
        .add_attribute("seam", Domain::Edge, Value::Bool(false))
        .unwrap();
    before
        .add_attribute("w", Domain::Point, Value::Float(0.0))
        .unwrap();
    let faces: Vec<(FaceId, Vec<VertexId>)> =
        before.faces().map(|(f, c)| (f, c.to_vec())).collect();
    for (f, corners) in faces {
        before
            .set_attribute("material", AttributeElement::Face(f), Value::Int(7))
            .unwrap();
        for (i, c) in corners.iter().enumerate() {
            let p = before.position(*c).unwrap();
            before
                .set_attribute(
                    "uv",
                    AttributeElement::Corner(f, i),
                    Value::Vec2(p.truncate()),
                )
                .unwrap();
            before
                .set_attribute("w", AttributeElement::Point(*c), Value::Float(p.x))
                .unwrap();
        }
    }
    let edge = before.edges()[0];
    let (a, b) = edge.ends();
    before
        .set_attribute("seam", AttributeElement::Edge(edge), Value::Bool(true))
        .unwrap();
    before.set_hard(a, b, true);

    let mut mesh = before.clone();
    assert_eq!(loop_cut(&mut mesh, edge, 1, 0.5), DONE);
    // The new polygon is like the one it was cut from.
    let new_face = FaceId::from_raw(1);
    assert_eq!(
        mesh.attribute("material", AttributeElement::Face(new_face)),
        Some(Value::Int(7))
    );
    // Every corner's UV is still its point's position, and every point's
    // `w` its X.
    for (f, corners) in mesh.faces() {
        for (i, c) in corners.iter().enumerate() {
            let p = mesh.position(*c).unwrap();
            let Some(Value::Vec2(uv)) = mesh.attribute("uv", AttributeElement::Corner(f, i)) else {
                panic!("no uv");
            };
            assert!(uv.distance(p.truncate()) < 1e-4, "{f:?} {i}: {uv} at {p}");
            let Some(Value::Float(w)) = mesh.attribute("w", AttributeElement::Point(*c)) else {
                panic!("no w");
            };
            assert!((w - p.x).abs() < 1e-4);
        }
    }
    // The cut edge's halves are both seams and hard; the whole edge is gone.
    let middle = VertexId::from_raw(4);
    for end in [a, b] {
        let half = EdgeKey::new(end, middle);
        assert_eq!(
            mesh.attribute("seam", AttributeElement::Edge(half)),
            Some(Value::Bool(true))
        );
        assert!(mesh.is_hard(end, middle));
    }
    assert!(!mesh.is_hard(a, b));
    assert_eq!(mesh.hard_edge_count(), 2);
    assert!(mesh.attributes().any(|(n, _, _)| n == HARD));
    // The new edge across is neither.
    let other = VertexId::from_raw(5);
    assert!(!mesh.is_hard(middle, other));
}

// ------------------------------------------------------- every shape ---

#[test]
fn every_edge_of_every_shape_cuts_cleanly() {
    for primitive in Primitive::all() {
        let before = primitive.mesh();
        for (n, edge) in before.edges().iter().enumerate() {
            let mut mesh = before.clone();
            let out = loop_cut(&mut mesh, *edge, 1 + (n % 3) as u32, 0.3);
            assert_eq!(out.missing, 0);
            assert_eq!(out.matched + out.skipped, 1);
            if out.skipped == 1 {
                assert_eq!(mesh, before, "{}: {edge:?}", primitive.name());
                continue;
            }
            let (r, b) = (MeshReport::new(&mesh), MeshReport::new(&before));
            assert_eq!(r.flipped_edges, 0, "{}: {edge:?}", primitive.name());
            assert_eq!(r.non_manifold_edges, 0, "{}: {edge:?}", primitive.name());
            assert_eq!(r.loose_points, 0, "{}: {edge:?}", primitive.name());
            assert_eq!(r.border_edges > 0, b.border_edges > 0);
            assert_eq!(r.pieces, b.pieces);
            assert!(mesh.face_count() > before.face_count());
        }
    }
}
