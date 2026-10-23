//! Tests for the edit list's commands (decision 0092): deleting any edit,
//! and freezing the edits up to one into the base.

use crate::*;
use glam::Vec3;

fn with_a_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

fn lift(point: u32, by: f32) -> Operation {
    Operation::MovePoints {
        points: vec![VertexId::from_raw(point)],
        offset: Vec3::Z * by,
        anchors: None,
    }
}

fn record(doc: &mut Document, id: ObjectId, operation: Operation) -> EditId {
    doc.execute(Command::Record { id, operation }).unwrap();
    let stack = &doc.scene().object(id).unwrap().stack;
    let layer = stack.layers().iter().rev().find(|l| !l.entries.is_empty());
    layer.unwrap().entries.last().unwrap().id
}

fn points(doc: &Document, id: ObjectId) -> Vec<Vec3> {
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    mesh.vertices().map(|(_, p)| p).collect()
}

fn same(a: &[Vec3], b: &[Vec3]) {
    assert_eq!(a.len(), b.len());
    for (p, q) in a.iter().zip(b) {
        assert!(p.distance(*q) < 1e-4, "{p} {q}");
    }
}

fn counts(doc: &Document, id: ObjectId) -> Vec<usize> {
    let stack = &doc.scene().object(id).unwrap().stack;
    stack.layers().iter().map(|l| l.entries.len()).collect()
}

/// The top polygon of the shape.
fn top_face(mesh: &Mesh) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .max_by(|a, b| mesh.face_normal(*a).z.total_cmp(&mesh.face_normal(*b).z))
        .unwrap()
}

fn extrude_top(doc: &Document, id: ObjectId, distance: f32) -> Operation {
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let top = top_face(&mesh);
    let corners = vec![mesh.face(top).unwrap().to_vec()];
    Operation::ExtrudePolygons {
        faces: vec![top],
        distance,
        grouping: Grouping::Together,
        shape: ExtrudeShape::EARLIER,
        anchors: mesh::Anchors::record(&mesh, ElementKind::Polygons, &corners),
        stretch: Vec3::ONE,
    }
}

#[test]
fn deleting_an_earlier_edit_keeps_the_later_ones() {
    let (mut doc, id) = with_a_cube();
    let start = points(&doc, id);
    let first = record(&mut doc, id, lift(0, 10.0));
    record(&mut doc, id, lift(2, 20.0));
    doc.execute(Command::DeleteEdit { id, edit: first })
        .unwrap();
    assert_eq!(doc.undo_label(), Some("Delete move 1 point"));
    let now = points(&doc, id);
    assert!(
        (now[0].z - start[0].z).abs() < 1e-4,
        "the first edit is gone"
    );
    assert!(
        (now[2].z - (start[2].z + 20.0)).abs() < 1e-4,
        "the second stays"
    );
    doc.undo().unwrap();
    assert_eq!(counts(&doc, id), vec![2]);
    let gone = doc.execute(Command::DeleteEdit {
        id,
        edit: EditId::from_raw(99),
    });
    assert!(matches!(gone, Err(SceneError::History(_))));
}

#[test]
fn freezing_keeps_the_shape_and_the_edits_after() {
    let (mut doc, id) = with_a_cube();
    let op = extrude_top(&doc, id, 30.0);
    let first = record(&mut doc, id, op);
    let op = extrude_top(&doc, id, 20.0);
    record(&mut doc, id, op);
    let before = points(&doc, id);
    let stack_before = doc.scene().object(id).unwrap().stack.clone();
    doc.execute(Command::Freeze { id, edit: first }).unwrap();
    assert_eq!(doc.undo_label(), Some("Freeze 1 edit"));
    assert!(matches!(
        doc.scene().object(id).unwrap().kind,
        ObjectKind::Mesh(_)
    ));
    assert_eq!(counts(&doc, id), vec![1]);
    same(&points(&doc, id), &before);
    doc.undo().unwrap();
    assert_eq!(doc.scene().object(id).unwrap().stack, stack_before);
    assert!(matches!(
        doc.scene().object(id).unwrap().kind,
        ObjectKind::Primitive(_)
    ));
    same(&points(&doc, id), &before);
}

#[test]
fn freezing_takes_the_layers_below_and_keeps_the_rest_of_its_own() {
    let (mut doc, id) = with_a_cube();
    record(&mut doc, id, lift(0, 10.0));
    record(&mut doc, id, lift(1, 10.0));
    doc.execute(Command::AddLayer(vec![id])).unwrap();
    let middle = record(&mut doc, id, lift(2, 10.0));
    record(&mut doc, id, lift(3, 10.0));
    doc.execute(Command::AddLayer(vec![id])).unwrap();
    record(&mut doc, id, lift(4, 10.0));
    let before = points(&doc, id);
    assert_eq!(
        frozen_count(&doc.scene().object(id).unwrap().stack, middle),
        3
    );
    doc.execute(Command::Freeze { id, edit: middle }).unwrap();
    assert_eq!(doc.undo_label(), Some("Freeze 3 edits"));
    assert_eq!(counts(&doc, id), vec![1, 1]);
    let names: Vec<String> = doc
        .scene()
        .object(id)
        .unwrap()
        .stack
        .layers()
        .iter()
        .map(|l| l.name.clone())
        .collect();
    assert_eq!(names, ["Edits 2", "Edits 3"]);
    same(&points(&doc, id), &before);
}

#[test]
fn freezing_the_last_edit_of_a_layer_leaves_it_empty() {
    let (mut doc, id) = with_a_cube();
    let only = record(&mut doc, id, lift(0, 10.0));
    let before = points(&doc, id);
    doc.execute(Command::Freeze { id, edit: only }).unwrap();
    assert_eq!(counts(&doc, id), vec![0]);
    same(&points(&doc, id), &before);
    // New edits still have somewhere to go.
    record(&mut doc, id, lift(1, 5.0));
    assert_eq!(counts(&doc, id), vec![1]);
}

#[test]
fn switched_off_edits_are_never_frozen_and_kept_ones_no_longer_hold_it_back() {
    let (mut doc, id) = with_a_cube();
    let first = record(&mut doc, id, lift(0, 10.0));
    let second = record(&mut doc, id, lift(1, 10.0));
    let refused = |doc: &mut Document| {
        let before = doc.scene().object(id).unwrap().clone();
        let result = doc.execute(Command::Freeze { id, edit: second });
        assert!(matches!(result, Err(SceneError::History(_))), "{result:?}");
        assert_eq!(*doc.scene().object(id).unwrap(), before);
    };
    // A kept (pinned) edit from an older project no longer stops it
    // (decision 0126: the switch is gone).
    doc.execute(Command::SetEditPinned {
        id,
        edit: first,
        pinned: true,
    })
    .unwrap();
    assert!(
        doc.scene()
            .object(id)
            .unwrap()
            .stack
            .freeze_problem(second)
            .is_none()
    );
    doc.undo().unwrap();
    doc.execute(Command::SetEditEnabled {
        id,
        edit: first,
        enabled: false,
    })
    .unwrap();
    refused(&mut doc);
    doc.undo().unwrap();
    doc.execute(Command::ChangeLayer {
        id,
        index: 0,
        change: LayerChange::SetEnabled(false),
    })
    .unwrap();
    refused(&mut doc);
}

#[test]
fn instances_and_groups_cant_be_frozen() {
    let (mut doc, cube) = with_a_cube();
    let edit = record(&mut doc, cube, lift(0, 10.0));
    doc.execute(Command::Duplicate {
        ids: vec![cube],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    let result = doc.execute(Command::Freeze { id: instance, edit });
    assert_eq!(result, Err(SceneError::Instance));
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    let result = doc.execute(Command::Freeze { id: group, edit });
    assert_eq!(result, Err(SceneError::NoShape));
}
