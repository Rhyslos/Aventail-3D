//! UVs that survive modelling (decision 0104): edits after UV work get
//! their new polygons laid out around the kept ones, and UV work replays
//! on a shape changed under it.

use crate::*;
use glam::{Affine2, Vec2, Vec3};

fn with_a_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

fn mesh_of(doc: &Document, id: ObjectId) -> Mesh {
    doc.scene().evaluated(id).unwrap().mesh.as_ref().clone()
}

fn record(doc: &mut Document, id: ObjectId, operation: Operation) {
    doc.execute(Command::Record { id, operation }).unwrap();
}

/// The polygons facing up.
fn top(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces()
        .map(|(f, _)| f)
        .filter(|f| mesh.face_normal(*f).z > 0.99)
        .collect()
}

fn anchors(mesh: &Mesh, faces: &[FaceId]) -> Option<mesh::Anchors> {
    let corners: Vec<Vec<VertexId>> = faces
        .iter()
        .map(|f| mesh.face(*f).unwrap().to_vec())
        .collect();
    mesh::Anchors::record(mesh, ElementKind::Polygons, &corners)
}

fn uvs_of(mesh: &Mesh, faces: &[FaceId]) -> Vec<Vec2> {
    let uvs = mesh::uvs::Uvs::of(mesh).unwrap();
    faces
        .iter()
        .flat_map(|f| {
            let n = mesh.face(*f).unwrap().len();
            (0..n)
                .map(|k| uvs.corner(mesh, *f, k).unwrap())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Every polygon has room on the texture.
fn all_have_room(mesh: &Mesh) {
    for (f, s) in uv::stretch::stretch_of(mesh).unwrap() {
        assert!(!s.empty, "polygon {} has no room", f.get());
    }
}

/// A turn of the top's UVs by hand, about the square's middle.
fn turn_top(mesh: &Mesh) -> Operation {
    let faces = top(mesh);
    let turn = Affine2::from_translation(Vec2::splat(0.5))
        * Affine2::from_angle(0.4)
        * Affine2::from_translation(Vec2::splat(-0.5));
    Operation::TransformUvs {
        anchors: anchors(mesh, &faces),
        faces,
        points: Vec::new(),
        transform: turn,
    }
}

fn extrude_top(mesh: &Mesh, distance: f32) -> Operation {
    let faces = top(mesh);
    Operation::ExtrudePolygons {
        anchors: anchors(mesh, &faces),
        faces,
        distance,
        grouping: Grouping::Together,
        shape: ExtrudeShape::EARLIER,
        stretch: Vec3::ONE,
    }
}

#[test]
fn an_extrude_after_uv_work_gets_room_and_keeps_the_hand_work() {
    let (mut doc, id) = with_a_cube();
    let cube = mesh_of(&doc, id);
    record(&mut doc, id, turn_top(&cube));
    let turned = mesh_of(&doc, id);
    let up = top(&turned);
    let kept = uvs_of(&turned, &up);
    record(&mut doc, id, extrude_top(&turned, 50.0));
    let extruded = mesh_of(&doc, id);
    all_have_room(&extruded);
    assert_eq!(uvs_of(&extruded, &up), kept, "the turned top keeps its UVs");
    // Changing the extrude afterwards mends again.
    doc.execute(Command::ReplaceOperation {
        id,
        layer: 0,
        index: 1,
        operation: extrude_top(&turned, 120.0),
    })
    .unwrap();
    let longer = mesh_of(&doc, id);
    all_have_room(&longer);
    assert_eq!(uvs_of(&longer, &up), kept);
    // Undo takes the extrude back, and the UVs with it.
    doc.undo();
    doc.undo();
    assert_eq!(mesh_of(&doc, id), turned);
}

#[test]
fn uv_work_replays_when_the_shape_changes_under_it() {
    let (mut doc, id) = with_a_cube();
    let cube = mesh_of(&doc, id);
    let turn = turn_top(&cube);
    let Operation::TransformUvs { transform, .. } = turn.clone() else {
        unreachable!()
    };
    record(&mut doc, id, turn);
    let turned = mesh_of(&doc, id);
    record(&mut doc, id, extrude_top(&turned, 50.0));
    // More segments: the base changes under both edits.
    let mut more = PrimitiveKind::Cube.standard();
    if let Primitive::Cube(c) = &mut more {
        c.segments = [2, 2, 2];
    }
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: more,
    })
    .unwrap();
    let after = mesh_of(&doc, id);
    all_have_room(&after);
    // The top's four polygons were found again and turned like the one
    // before: their UVs are the plain cube's, turned.
    let plain = more.mesh();
    let plain_top = top(&plain);
    let expected: Vec<Vec2> = uvs_of(&plain, &plain_top)
        .into_iter()
        .map(|v| transform.transform_point2(v))
        .collect();
    let got = uvs_of(&after, &plain_top);
    assert_eq!(expected.len(), got.len());
    for (e, g) in expected.iter().zip(&got) {
        assert!(e.distance(*g) < 1e-5, "{e} {g}");
    }
}

#[test]
fn moving_points_leaves_the_uvs_alone() {
    let (mut doc, id) = with_a_cube();
    let cube = mesh_of(&doc, id);
    let up = top(&cube);
    let points = cube.face(up[0]).unwrap().to_vec();
    record(
        &mut doc,
        id,
        Operation::MovePoints {
            points,
            offset: Vec3::Z * 30.0,
            anchors: None,
        },
    );
    let moved = mesh_of(&doc, id);
    assert_eq!(uvs_of(&moved, &up), uvs_of(&cube, &up));
}
