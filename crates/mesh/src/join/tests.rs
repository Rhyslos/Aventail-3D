//! Tests for Connect, Bridge and Weld.

use super::*;
use crate::{CylinderParams, MeshReport, Primitive};

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

fn report(mesh: &Mesh) -> MeshReport {
    MeshReport::new(mesh)
}

fn clean(mesh: &Mesh) {
    let r = report(mesh);
    assert!(!r.has_problems(), "{r:?}");
}

/// The cube's polygon facing `way`.
fn facing(mesh: &Mesh, way: Vec3) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .max_by(|a, b| {
            mesh.face_normal(*a)
                .dot(way)
                .total_cmp(&mesh.face_normal(*b).dot(way))
        })
        .unwrap()
}

/// `b` added to `a`, moved by `offset`.
fn combine(a: &Mesh, b: &Mesh, offset: Vec3) -> Mesh {
    let mut out = a.clone();
    let mut map = BTreeMap::new();
    for (v, p) in b.vertices() {
        map.insert(v, out.add_vertex(p + offset).unwrap());
    }
    for (_, corners) in b.faces() {
        let c: Vec<VertexId> = corners.iter().map(|v| map[v]).collect();
        out.add_face(&c).unwrap();
    }
    out
}

fn tube() -> Mesh {
    Primitive::Cylinder(CylinderParams {
        sides: 16,
        height_segments: 1,
        caps: false,
        ..Default::default()
    })
    .mesh()
}

/// The border edges at height `z`.
fn border_at(mesh: &Mesh, z: f32) -> Vec<EdgeKey> {
    let topology = mesh.topology();
    topology
        .border_edges()
        .into_iter()
        .filter(|e| {
            let (a, b) = e.ends();
            (mesh.position(a).unwrap().z - z).abs() < 1e-3
                && (mesh.position(b).unwrap().z - z).abs() < 1e-3
        })
        .collect()
}

// ------------------------------------------------------------- connect ---

#[test]
fn connecting_opposite_corners_cuts_a_diagonal() {
    let before = cube();
    let front = facing(&before, -Vec3::Y);
    let c = before.face(front).unwrap().to_vec();
    let mut mesh = before.clone();
    let out = connect_points(&mut mesh, &[c[0], c[2]]);
    assert_eq!(out.matched, 1);
    assert_eq!(mesh.face_count(), 7);
    assert_eq!(mesh.vertex_count(), 8);
    assert!(report(&mesh).is_closed());
    clean(&mesh);
    // Neighbours are already joined: nothing to do.
    let mut same = before.clone();
    assert_eq!(connect_points(&mut same, &[c[0], c[1]]).skipped, 1);
    assert_eq!(same, before);
    // Three corners: the one pair not already joined.
    let mut three = before.clone();
    assert_eq!(connect_points(&mut three, &[c[0], c[1], c[2]]).matched, 1);
    assert_eq!(three.face_count(), 7);
}

#[test]
fn points_round_a_many_sided_polygon_join_all_the_way_round() {
    let before = Primitive::Cylinder(CylinderParams {
        sides: 16,
        caps: true,
        cap_segments: 1,
        cap_centre: false,
        ..Default::default()
    })
    .mesh();
    let top = facing(&before, Vec3::Z);
    let c = before.face(top).unwrap().to_vec();
    let mut mesh = before.clone();
    connect_points(&mut mesh, &[c[0], c[4], c[8], c[12]]);
    // An inner square and four pieces round it.
    assert_eq!(mesh.face_count(), before.face_count() + 4);
    assert!(
        mesh.faces()
            .any(|(_, c)| c.len() == 4 && c.iter().all(|v| mesh.position(*v).unwrap().z > 99.0))
    );
    clean(&mesh);
}

#[test]
fn connecting_a_ring_of_edges_makes_a_loop() {
    let before = cube();
    let upright: Vec<EdgeKey> = before
        .edges()
        .iter()
        .copied()
        .filter(|e| {
            let (a, b) = e.ends();
            (before.position(a).unwrap().z - before.position(b).unwrap().z).abs() > 1.0
        })
        .collect();
    assert_eq!(upright.len(), 4);
    let mut mesh = before.clone();
    assert_eq!(connect_edges(&mut mesh, &upright, 1).matched, 4);
    assert_eq!(mesh.face_count(), 10);
    assert_eq!(mesh.vertex_count(), 12);
    assert!(report(&mesh).is_closed());
    clean(&mesh);
    assert!(
        mesh.vertices()
            .skip(8)
            .all(|(_, p)| (p.z - 50.0).abs() < 1e-3)
    );
    let mut two = before.clone();
    connect_edges(&mut two, &upright, 2);
    assert_eq!(two.face_count(), 14);
    clean(&two);
    // One edge alone joins nothing.
    let mut one = before.clone();
    assert_eq!(connect_edges(&mut one, &upright[..1], 1).skipped, 1);
    assert_eq!(one, before);
}

#[test]
fn connecting_two_edges_at_a_corner_cuts_the_corner_off() {
    let before = cube();
    let front = facing(&before, -Vec3::Y);
    let c = before.face(front).unwrap().to_vec();
    let edges = [EdgeKey::new(c[0], c[1]), EdgeKey::new(c[1], c[2])];
    let mut mesh = before.clone();
    assert_eq!(connect_edges(&mut mesh, &edges, 1).matched, 1);
    assert_eq!(mesh.face_count(), 7);
    assert!(mesh.faces().any(|(_, c)| c.len() == 3));
    assert!(report(&mesh).is_closed());
    clean(&mesh);
}

// -------------------------------------------------------------- bridge ---

#[test]
fn two_open_tubes_are_bridged_into_one() {
    let before = combine(&tube(), &tube(), Vec3::new(0.0, 0.0, 200.0));
    let mut edges = border_at(&before, 100.0);
    edges.extend(border_at(&before, 200.0));
    assert_eq!(edges.len(), 32);
    let mut mesh = before.clone();
    let out = bridge_edges(&mut mesh, &edges, 3, 0);
    assert_eq!(out.matched, 16 * 3);
    let r = report(&mesh);
    assert_eq!(r.pieces, 1, "{r:?}");
    assert_eq!(r.border_edges, 32, "only the two far ends stay open: {r:?}");
    assert_eq!(r.flipped_edges, 0, "{r:?}");
    clean(&mesh);
    // Twisting turns the band round.
    let mut twisted = before.clone();
    bridge_edges(&mut twisted, &edges, 1, 2);
    assert_eq!(report(&twisted).flipped_edges, 0);
    assert_ne!(twisted, {
        let mut plain = before.clone();
        bridge_edges(&mut plain, &edges, 1, 0);
        plain
    });
}

#[test]
fn borders_of_different_lengths_are_zipped() {
    let narrow = Primitive::Cylinder(CylinderParams {
        sides: 8,
        height_segments: 1,
        caps: false,
        ..Default::default()
    })
    .mesh();
    let before = combine(&tube(), &narrow, Vec3::new(0.0, 0.0, 200.0));
    let mut edges = border_at(&before, 100.0);
    edges.extend(border_at(&before, 200.0));
    let mut mesh = before.clone();
    let out = bridge_edges(&mut mesh, &edges, 1, 0);
    assert_eq!(out.matched, 16 + 8);
    let r = report(&mesh);
    assert_eq!(r.pieces, 1, "{r:?}");
    assert_eq!(r.flipped_edges, 0, "{r:?}");
    assert_eq!(r.non_manifold_edges, 0, "{r:?}");
}

#[test]
fn two_open_edges_get_a_polygon_between_them() {
    let mut before = Mesh::new();
    let p = |m: &mut Mesh, x: f32, y: f32| m.add_vertex(Vec3::new(x, y, 0.0)).unwrap();
    let a = [
        p(&mut before, 0.0, 0.0),
        p(&mut before, 10.0, 0.0),
        p(&mut before, 10.0, 10.0),
        p(&mut before, 0.0, 10.0),
    ];
    let b = [
        p(&mut before, 20.0, 0.0),
        p(&mut before, 30.0, 0.0),
        p(&mut before, 30.0, 10.0),
        p(&mut before, 20.0, 10.0),
    ];
    before.add_face(&a).unwrap();
    before.add_face(&b).unwrap();
    let mut mesh = before.clone();
    let out = bridge_edges(
        &mut mesh,
        &[EdgeKey::new(a[1], a[2]), EdgeKey::new(b[0], b[3])],
        1,
        0,
    );
    assert_eq!(out.matched, 1);
    let r = report(&mesh);
    assert_eq!((r.pieces, r.flipped_edges), (1, 0), "{r:?}");
    // Not border edges, or only one chain: nothing.
    let mut none = before.clone();
    assert_eq!(
        bridge_edges(&mut none, &[EdgeKey::new(a[1], a[2])], 1, 0).skipped,
        1
    );
    assert_eq!(none, before);
}

#[test]
fn two_polygons_are_bridged_by_a_tube() {
    let before = combine(&cube(), &cube(), Vec3::new(200.0, 0.0, 0.0));
    let right = before
        .faces()
        .map(|(f, _)| f)
        .find(|f| {
            before.face_normal(*f).x > 0.9
                && before
                    .face(*f)
                    .unwrap()
                    .iter()
                    .all(|v| before.position(*v).unwrap().x < 60.0)
        })
        .unwrap();
    let left = before
        .faces()
        .map(|(f, _)| f)
        .find(|f| {
            before.face_normal(*f).x < -0.9
                && before
                    .face(*f)
                    .unwrap()
                    .iter()
                    .all(|v| before.position(*v).unwrap().x > 100.0)
        })
        .unwrap();
    let mut mesh = before.clone();
    let out = bridge_faces(&mut mesh, &[right, left], 2, 0);
    assert_eq!(out.matched, 8);
    assert!(mesh.face(right).is_none() && mesh.face(left).is_none());
    let r = report(&mesh);
    assert!(r.is_closed(), "{r:?}");
    assert_eq!(r.pieces, 1, "{r:?}");
    clean(&mesh);
    // Anything but two polygons: nothing.
    let mut one = before.clone();
    assert_eq!(bridge_faces(&mut one, &[right], 1, 0).skipped, 1);
    assert_eq!(one, before);
}

// ---------------------------------------------------------------- weld ---

#[test]
fn welding_an_edges_ends_collapses_it() {
    let before = cube();
    let top = facing(&before, Vec3::Z);
    let c = before.face(top).unwrap().to_vec();
    let (pa, pb) = (
        before.position(c[0]).unwrap(),
        before.position(c[1]).unwrap(),
    );
    let mut mesh = before.clone();
    assert_eq!(weld(&mut mesh, &[c[0], c[1]], None).matched, 2);
    assert_eq!(mesh.vertex_count(), 7);
    let keep = c[0].min(c[1]);
    assert!(mesh.position(keep).unwrap().distance(pa.lerp(pb, 0.5)) < 1e-4);
    assert!(report(&mesh).is_closed());
    clean(&mesh);
    assert_eq!(mesh.faces().filter(|(_, c)| c.len() == 3).count(), 2);
}

#[test]
fn welding_a_whole_polygon_makes_a_point() {
    let before = cube();
    let top = facing(&before, Vec3::Z);
    let c = before.face(top).unwrap().to_vec();
    let mut mesh = before.clone();
    weld(&mut mesh, &c, None);
    assert!(mesh.face(top).is_none(), "the top collapsed away");
    assert_eq!(mesh.vertex_count(), 5);
    assert_eq!(mesh.face_count(), 5);
    assert!(report(&mesh).is_closed());
    clean(&mesh);
}

#[test]
fn welding_onto_a_target_keeps_it_where_it_is() {
    let before = cube();
    let top = facing(&before, Vec3::Z);
    let c = before.face(top).unwrap().to_vec();
    let at = before.position(c[2]).unwrap();
    let mut mesh = before.clone();
    weld(&mut mesh, &[c[1]], Some(c[2]));
    assert_eq!(mesh.position(c[2]), Some(at));
    assert!(mesh.position(c[1]).is_none());
    clean(&mesh);
    // One point and no target, or points not there: nothing.
    let mut alone = before.clone();
    assert_eq!(weld(&mut alone, &[c[1]], None).skipped, 1);
    let out = weld(&mut alone, &[VertexId::from_raw(99), c[1]], None);
    assert_eq!((out.skipped, out.missing), (1, 1));
    assert_eq!(alone, before);
}

#[test]
fn welding_carries_edge_values_to_the_kept_point() {
    let mut before = cube();
    let top = facing(&before, Vec3::Z);
    let c = before.face(top).unwrap().to_vec();
    // Hard edges everywhere on the cube already; weld two far corners and
    // the edges from the gone point now start at the kept one.
    let mut mesh = before.clone();
    weld(&mut mesh, &[c[0], c[2]], Some(c[0]));
    for e in mesh.edges() {
        let (a, b) = e.ends();
        assert!(mesh.position(a).is_some() && mesh.position(b).is_some());
    }
    assert!(mesh.hard_edge_count() > 0);
    before.set_hard(c[0], c[1], false);
}

#[test]
fn every_shape_takes_every_join() {
    for primitive in Primitive::all() {
        let before = primitive.mesh();
        let points: Vec<VertexId> = before.vertices().map(|(v, _)| v).take(6).collect();
        let edges: Vec<EdgeKey> = before.edges().iter().copied().step_by(3).take(8).collect();
        let faces: Vec<FaceId> = before.faces().map(|(f, _)| f).take(2).collect();
        type Check<'a> = (&'a str, Box<dyn Fn(&mut Mesh) + 'a>);
        let checks: [Check; 5] = [
            (
                "connect points",
                Box::new(|m| {
                    connect_points(m, &points);
                }),
            ),
            (
                "connect edges",
                Box::new(|m| {
                    connect_edges(m, &edges, 2);
                }),
            ),
            (
                "bridge edges",
                Box::new(|m| {
                    let border = m.topology().border_edges();
                    bridge_edges(m, &border, 2, 1);
                }),
            ),
            (
                "bridge faces",
                Box::new(|m| {
                    bridge_faces(m, &faces, 2, 0);
                }),
            ),
            (
                "weld",
                Box::new(|m| {
                    weld(m, &points[..3.min(points.len())], None);
                }),
            ),
        ];
        for (name, check) in &checks {
            let mut mesh = before.clone();
            check(&mut mesh);
            let r = report(&mesh);
            assert_eq!(r.loose_points, 0, "{} {name}: {r:?}", primitive.name());
            for (v, p) in mesh.vertices() {
                assert!(p.is_finite(), "{} {name}: {v:?}", primitive.name());
            }
        }
    }
}
