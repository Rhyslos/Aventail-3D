//! Tests for clean-up.

use super::*;
use crate::{CylinderParams, MeshReport, PlaneParams, Primitive};

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

fn report(mesh: &Mesh) -> MeshReport {
    MeshReport::new(mesh)
}

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

#[test]
fn deleting_leaves_a_hole() {
    let before = cube();
    let top = facing(&before, Vec3::Z);
    let mut mesh = before.clone();
    assert_eq!(delete_elements(&mut mesh, &[], &[], &[top]).matched, 1);
    assert_eq!(mesh.face_count(), 5);
    assert_eq!(report(&mesh).border_edges, 4);
    assert_eq!(mesh.vertex_count(), 8);
    // A point takes its three polygons, and itself.
    let mut point = before.clone();
    delete_elements(&mut point, &[VertexId::from_raw(0)], &[], &[]);
    assert_eq!(point.face_count(), 3);
    assert_eq!(point.vertex_count(), 7);
    assert_eq!(report(&point).loose_points, 0);
    // An edge takes its two.
    let mut edge = before.clone();
    let e = before.edges()[0];
    delete_elements(&mut edge, &[], &[e], &[]);
    assert_eq!(edge.face_count(), 4);
    // Nothing there: nothing done.
    let mut none = before.clone();
    let out = delete_elements(&mut none, &[], &[], &[FaceId::from_raw(99)]);
    assert_eq!((out.missing, out.skipped), (1, 1));
    assert_eq!(none, before);
}

#[test]
fn dissolving_a_loop_takes_it_away_cleanly() {
    let before = cube();
    let mut cut = before.clone();
    let upright = *before
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            (before.position(a).unwrap().z - before.position(b).unwrap().z).abs() > 1.0
        })
        .unwrap();
    crate::loopcut::loop_cut(&mut cut, upright, 1, 0.5);
    assert_eq!(cut.face_count(), 10);
    let new_points: BTreeSet<VertexId> = cut.vertices().skip(8).map(|(v, _)| v).collect();
    let loop_edges: Vec<EdgeKey> = cut
        .edges()
        .iter()
        .copied()
        .filter(|e| {
            let (a, b) = e.ends();
            new_points.contains(&a) && new_points.contains(&b)
        })
        .collect();
    assert_eq!(loop_edges.len(), 4);
    let mut mesh = cut.clone();
    assert_eq!(dissolve_edges(&mut mesh, &loop_edges).matched, 4);
    assert_eq!(mesh.face_count(), 6);
    assert_eq!(mesh.vertex_count(), 8, "the loop's points go too");
    let r = report(&mesh);
    assert!(r.is_closed() && !r.has_problems(), "{r:?}");
}

#[test]
fn dissolving_polygons_and_points_merges_them() {
    let plane = Primitive::Plane(PlaneParams {
        segments: [2, 2],
        ..Default::default()
    })
    .mesh();
    let all: Vec<FaceId> = plane.faces().map(|(f, _)| f).collect();
    let mut faces = plane.clone();
    assert_eq!(dissolve_faces(&mut faces, &all).matched, 1);
    assert_eq!(faces.face_count(), 1);
    assert_eq!(faces.vertex_count(), 4, "straight-run points go");
    assert!(!report(&faces).has_problems());
    // The middle point: its four polygons become one with eight corners.
    let middle = plane
        .vertices()
        .find(|(_, p)| p.truncate().length() < 1e-3)
        .unwrap()
        .0;
    let mut point = plane.clone();
    assert_eq!(dissolve_points(&mut point, &[middle]).matched, 1);
    assert_eq!(point.face_count(), 1);
    assert_eq!(point.vertex_count(), 8);
    // A corner between two border edges just goes from its polygon.
    let mut corner = plane.clone();
    dissolve_points(&mut corner, &[VertexId::from_raw(0)]);
    assert_eq!(corner.vertex_count(), 8);
    assert!(corner.faces().any(|(_, c)| c.len() == 3));
    // A point on a border between three edges stays.
    let side = plane
        .vertices()
        .find(|(v, _)| plane.topology().vertex_edges(*v).len() == 3)
        .unwrap()
        .0;
    let mut stays = plane.clone();
    assert_eq!(dissolve_points(&mut stays, &[side]).skipped, 1);
}

#[test]
fn holes_are_filled_facing_out() {
    let before = cube();
    let mut mesh = before.clone();
    delete_elements(&mut mesh, &[], &[], &[facing(&before, Vec3::Z)]);
    let open = mesh.clone();
    assert_eq!(fill_holes(&mut mesh, &[]).matched, 1);
    let r = report(&mesh);
    assert!(r.is_closed() && !r.has_problems(), "{r:?}");
    assert!(mesh.face_normal(FaceId::from_raw(6)).z > 0.9);
    // Only holes touching the given points.
    let mut none = open.clone();
    assert_eq!(fill_holes(&mut none, &[VertexId::from_raw(999)]).skipped, 1);
    let tube = Primitive::Cylinder(CylinderParams {
        sides: 12,
        caps: false,
        ..Default::default()
    })
    .mesh();
    let mut capped = tube.clone();
    assert_eq!(fill_holes(&mut capped, &[]).matched, 2);
    let r = report(&capped);
    assert!(r.is_closed() && !r.has_problems(), "{r:?}");
}

#[test]
fn flipped_polygons_are_found_and_fixed() {
    let before = cube();
    let top = facing(&before, Vec3::Z);
    let mut mesh = before.clone();
    assert_eq!(flip_faces(&mut mesh, &[top]).matched, 1);
    assert_eq!(report(&mesh).flipped_edges, 4);
    assert_eq!(faces_facing_wrong(&mesh), vec![top]);
    assert_eq!(fix_facing(&mut mesh).matched, 1);
    assert_eq!(report(&mesh).flipped_edges, 0);
    assert!(mesh.face_normal(top).z > 0.9);
    // Nothing wrong: nothing to fix.
    assert_eq!(fix_facing(&mut mesh).skipped, 1);
    // A cube inside out: all six turn.
    let mut inside_out = before.clone();
    let all: Vec<FaceId> = before.faces().map(|(f, _)| f).collect();
    flip_faces(&mut inside_out, &all);
    assert_eq!(faces_facing_wrong(&inside_out).len(), 6);
    fix_facing(&mut inside_out);
    assert!(inside_out.face_normal(top).z > 0.9);
}

#[test]
fn polygons_are_found_by_facing() {
    let mesh = cube();
    assert_eq!(
        faces_facing(&mesh, Vec3::Z, 30.0),
        vec![facing(&mesh, Vec3::Z)]
    );
    assert_eq!(faces_facing(&mesh, Vec3::Z, 95.0).len(), 5);
    assert!(faces_facing(&mesh, Vec3::ZERO, 30.0).is_empty());
}

#[test]
fn every_shape_takes_every_clean_up() {
    for primitive in Primitive::all() {
        let before = primitive.mesh();
        let faces: Vec<FaceId> = before.faces().map(|(f, _)| f).take(3).collect();
        let edges: Vec<EdgeKey> = before.edges().iter().copied().step_by(5).take(6).collect();
        let points: Vec<VertexId> = before
            .vertices()
            .map(|(v, _)| v)
            .step_by(3)
            .take(4)
            .collect();
        let mut meshes = Vec::new();
        let mut m = before.clone();
        delete_elements(&mut m, &points[..1], &edges[..1], &faces[..1]);
        meshes.push(("delete", m));
        let mut m = before.clone();
        dissolve_edges(&mut m, &edges);
        meshes.push(("dissolve edges", m));
        let mut m = before.clone();
        dissolve_faces(&mut m, &faces);
        meshes.push(("dissolve polygons", m));
        let mut m = before.clone();
        dissolve_points(&mut m, &points);
        meshes.push(("dissolve points", m));
        let mut m = before.clone();
        delete_elements(&mut m, &[], &[], &faces[..1]);
        fill_holes(&mut m, &[]);
        meshes.push(("fill", m));
        let mut m = before.clone();
        flip_faces(&mut m, &faces);
        fix_facing(&mut m);
        meshes.push(("fix", m));
        for (name, mesh) in meshes {
            let r = report(&mesh);
            assert_eq!(r.loose_points, 0, "{} {name}: {r:?}", primitive.name());
            assert_eq!(
                r.non_manifold_edges,
                0,
                "{} {name}: {r:?}",
                primitive.name()
            );
            if name == "fix" || name.starts_with("dissolve") {
                assert_eq!(r.flipped_edges, 0, "{} {name}: {r:?}", primitive.name());
            }
        }
    }
}

#[test]
fn dissolving_an_edge_forgets_its_hard_mark() {
    // A cube's twelve edges are hard. Dissolving an upright one merges
    // two sides; the edge is gone, and so is its mark.
    let mut mesh = cube();
    assert_eq!(mesh.hard_edge_count(), 12);
    let upright = mesh
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            (mesh.position(a).unwrap().z - mesh.position(b).unwrap().z).abs() > 1.0
        })
        .copied()
        .unwrap();
    assert_eq!(dissolve_edges(&mut mesh, &[upright]).matched, 1);
    assert_eq!(mesh.hard_edge_count(), 11);
    assert!(!mesh.is_hard(upright.ends().0, upright.ends().1));
    let topology = mesh.topology();
    assert!(mesh.hard_edges().all(|e| topology.has_edge(e)));
}

#[test]
fn dissolving_polygons_forgets_the_marks_inside() {
    let mut mesh = cube();
    let top = facing(&mesh, Vec3::Z);
    let side = facing(&mesh, Vec3::X);
    assert_eq!(dissolve_faces(&mut mesh, &[top, side]).matched, 1);
    let topology = mesh.topology();
    assert!(mesh.hard_edges().all(|e| topology.has_edge(e)));
    assert_eq!(mesh.hard_edge_count(), 11);
}

#[test]
fn a_point_between_two_hard_edges_leaves_one_hard_edge() {
    // A square with a point in the middle of its bottom side, both halves
    // of that side hard.
    let p = VertexId::from_raw;
    let points = vec![
        Some(Vec3::new(0.0, 0.0, 0.0)),
        Some(Vec3::new(50.0, 0.0, 0.0)),
        Some(Vec3::new(100.0, 0.0, 0.0)),
        Some(Vec3::new(100.0, 100.0, 0.0)),
        Some(Vec3::new(0.0, 100.0, 0.0)),
    ];
    let faces = vec![Some(vec![p(0), p(1), p(2), p(3), p(4)])];
    let hard = [EdgeKey::new(p(0), p(1)), EdgeKey::new(p(1), p(2))];
    let mut mesh = Mesh::from_slots(points, faces, hard).unwrap();
    assert!(dissolve_points(&mut mesh, &[p(1)]).matched > 0);
    assert_eq!(mesh.vertex_count(), 4);
    assert!(mesh.is_hard(p(0), p(2)));
    assert_eq!(mesh.hard_edge_count(), 1);
    // Only one half hard: the joined edge is soft.
    let points = vec![
        Some(Vec3::new(0.0, 0.0, 0.0)),
        Some(Vec3::new(50.0, 0.0, 0.0)),
        Some(Vec3::new(100.0, 0.0, 0.0)),
        Some(Vec3::new(100.0, 100.0, 0.0)),
        Some(Vec3::new(0.0, 100.0, 0.0)),
    ];
    let faces = vec![Some(vec![p(0), p(1), p(2), p(3), p(4)])];
    let mut mesh = Mesh::from_slots(points, faces, [EdgeKey::new(p(0), p(1))]).unwrap();
    dissolve_points(&mut mesh, &[p(1)]);
    assert_eq!(mesh.hard_edge_count(), 0);
}

#[test]
fn forgetting_unused_edge_values_keeps_used_ones() {
    let mut mesh = cube();
    assert_eq!(mesh.forget_unused_edge_values(), 0);
    assert_eq!(mesh.hard_edge_count(), 12);
    // A mesh without edge values has nothing to forget.
    let mut plain = Primitive::Plane(PlaneParams::default()).mesh();
    let _ = plain.forget_unused_edge_values();
    let topology = plain.topology();
    assert!(plain.hard_edges().all(|e| topology.has_edge(e)));
}
