//! Texture lock (Phase 8 milestone 5): moving points with the texture
//! kept in place lays the UVs round them again for their new shape.

use crate::*;
use glam::{Vec2, Vec3};

/// A flat 4 × 4 plane and its id.
fn plane() -> (Document, ObjectId) {
    let mut doc = Document::new();
    let params = mesh::PlaneParams {
        segments: [4, 4],
        ..mesh::PlaneParams::default()
    };
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Plane(
        params,
    ))))
    .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

fn mesh_of(doc: &Document, id: ObjectId) -> Mesh {
    doc.scene().evaluated(id).unwrap().mesh.as_ref().clone()
}

/// The UV at each corner of point `v`.
fn uvs_at(mesh: &Mesh, v: VertexId) -> Vec<Vec2> {
    let uvs = mesh::uvs::Uvs::of(mesh).unwrap();
    let mut out = Vec::new();
    for (f, list) in mesh.faces() {
        for (k, p) in list.iter().enumerate() {
            if *p == v {
                out.push(uvs.corner(mesh, f, k).unwrap());
            }
        }
    }
    out
}

/// The point nearest the middle.
fn middle(mesh: &Mesh) -> VertexId {
    mesh.vertices()
        .min_by(|a, b| a.1.length().total_cmp(&b.1.length()))
        .unwrap()
        .0
}

fn moved(keep_texture: bool) -> (Mesh, Mesh, VertexId) {
    let (mut doc, id) = plane();
    let before = mesh_of(&doc, id);
    let point = middle(&before);
    doc.execute(Command::Record {
        id,
        operation: Operation::MovePoints {
            points: vec![point],
            offset: Vec3::new(20.0, 10.0, 0.0),
            anchors: None,
            keep_texture,
        },
    })
    .unwrap();
    (before, mesh_of(&doc, id), point)
}

#[test]
fn without_the_lock_the_uvs_stay_and_the_texture_stretches() {
    let (before, after, point) = moved(false);
    assert_eq!(uvs_at(&before, point), uvs_at(&after, point));
}

#[test]
fn with_the_lock_the_moved_points_uv_follows_it() {
    let (before, after, point) = moved(true);
    let was = uvs_at(&before, point)[0];
    let now = uvs_at(&after, point)[0];
    // Moved right and forward on the surface: its UV moved the same way.
    assert!(now.x > was.x + 1e-3, "{was} → {now}");
    // Points beyond the polygons round it keep their UVs.
    let far = before
        .vertices()
        .max_by(|a, b| a.1.length().total_cmp(&b.1.length()))
        .unwrap()
        .0;
    assert_eq!(uvs_at(&before, far), uvs_at(&after, far));
}

#[test]
fn the_lock_with_nothing_to_hold_does_nothing() {
    let mut empty = Mesh::default();
    crate::stack::keep_texture_round(&mut empty, &[]);
    assert_eq!(empty.face_count(), 0);
    let (doc, id) = plane();
    let mut shape = mesh_of(&doc, id);
    let before = shape.clone();
    crate::stack::keep_texture_round(&mut shape, &[]);
    assert_eq!(shape, before, "no points, nothing laid again");
}
