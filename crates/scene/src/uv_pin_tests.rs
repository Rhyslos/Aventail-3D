//! Pinned UVs and unwrap methods as recorded edits (decision 0109): pins
//! are held by a later unwrap, survive undo and redo, and every method
//! replays to the same UVs.

use crate::*;
use glam::Vec2;

fn with_a_plane() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Plane.standard(),
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

fn point_uv(mesh: &Mesh, v: VertexId) -> Vec2 {
    let uvs = mesh::uvs::Uvs::of(mesh).unwrap();
    let (starts, _) = mesh.face_layout();
    let (f, list) = mesh.faces().find(|(_, l)| l.contains(&v)).unwrap();
    let k = list.iter().position(|x| *x == v).unwrap();
    uvs.at(starts[f.get() as usize] as usize + k)
}

fn unwrap(faces: Vec<FaceId>, method: UnwrapMethod, only: Option<UnfoldAxis>) -> Operation {
    Operation::Unwrap {
        faces,
        options: UnwrapOptions {
            method,
            only,
            ..UnwrapOptions::plain(false)
        },
        anchors: None,
    }
}

#[test]
fn pins_hold_through_a_later_unwrap_and_come_off_with_undo() {
    let (mut doc, id) = with_a_plane();
    let mesh = mesh_of(&doc, id);
    let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
    let points: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).collect();
    let pinned = vec![points[0], *points.last().unwrap()];
    // Moved off their unwrapped spots first, so a plain unwrap would
    // differ.
    record(
        &mut doc,
        id,
        Operation::TransformUvs {
            faces: Vec::new(),
            points: pinned.clone(),
            transform: glam::Affine2::from_translation(Vec2::new(0.05, 0.0)),
            anchors: None,
        },
    );
    record(
        &mut doc,
        id,
        Operation::SetUvPins {
            faces: Vec::new(),
            points: pinned.clone(),
            pin: true,
            anchors: None,
        },
    );
    let mesh = mesh_of(&doc, id);
    assert!(mesh::uvs::pins(&mesh).is_some());
    let before: Vec<Vec2> = pinned.iter().map(|v| point_uv(&mesh, *v)).collect();
    record(
        &mut doc,
        id,
        unwrap(faces.clone(), UnwrapMethod::SizeKeeping, None),
    );
    let mesh = mesh_of(&doc, id);
    for (v, was) in pinned.iter().zip(&before) {
        assert!(point_uv(&mesh, *v).distance(*was) < 1e-5);
    }
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert!(mesh::uvs::pins(&mesh_of(&doc, id)).is_none());
    doc.redo().unwrap();
    assert!(mesh::uvs::pins(&mesh_of(&doc, id)).is_some());
    // Unpinning everything takes the pins off again.
    record(
        &mut doc,
        id,
        Operation::SetUvPins {
            faces,
            points: Vec::new(),
            pin: false,
            anchors: None,
        },
    );
    assert!(mesh::uvs::pins(&mesh_of(&doc, id)).is_none());
}

#[test]
fn every_method_and_one_way_unfold_replays_the_same() {
    for method in UnwrapMethod::ALL {
        for only in [None, Some(UnfoldAxis::Across), Some(UnfoldAxis::Up)] {
            let (mut doc, id) = with_a_plane();
            let faces: Vec<FaceId> = mesh_of(&doc, id).faces().map(|(f, _)| f).collect();
            record(&mut doc, id, unwrap(faces, method, only));
            let count = mesh_of(&doc, id).face_layout().1.len();
            let uvs = |doc: &Document| {
                mesh::uvs::Uvs::of(&mesh_of(doc, id))
                    .unwrap()
                    .into_values(count)
            };
            let first = uvs(&doc);
            assert!(first.iter().all(|v| v.is_finite()));
            doc.undo().unwrap();
            doc.redo().unwrap();
            assert_eq!(first, uvs(&doc), "{method:?} {only:?}");
        }
    }
}

#[test]
fn pin_labels_read_well() {
    let pin = Operation::SetUvPins {
        faces: Vec::new(),
        points: vec![VertexId::from_raw(0), VertexId::from_raw(1)],
        pin: true,
        anchors: None,
    };
    assert_eq!(pin.describe(), "Pin the UVs of 2 points");
    let unpin = Operation::SetUvPins {
        faces: vec![FaceId::from_raw(0)],
        points: Vec::new(),
        pin: false,
        anchors: None,
    };
    assert_eq!(unpin.describe(), "Unpin the UVs of 1 polygon");
    assert!(pin.numbers_are_real());
}

#[test]
fn seam_preferences_and_budgets_are_recorded_edits() {
    let (mut doc, id) = with_a_plane();
    let mesh = mesh_of(&doc, id);
    let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
    let topology = mesh.topology();
    let edges: Vec<EdgeKey> = topology.edges().take(2).collect();
    record(
        &mut doc,
        id,
        Operation::SetSeamPreference {
            edges: edges.clone(),
            preference: SeamPreference::Never,
            anchors: None,
        },
    );
    record(
        &mut doc,
        id,
        Operation::SetStretchBudget {
            faces: faces.clone(),
            budget: Some(1.5),
            anchors: None,
        },
    );
    let mesh = mesh_of(&doc, id);
    assert_eq!(mesh::uvs::budget_of(&mesh, &faces), Some(1.5));
    let open = edges
        .iter()
        .filter(|e| mesh.topology().edge_uses(**e).len() == 2)
        .count();
    assert_eq!(mesh::uvs::seam_preferences(&mesh).len(), open);
    doc.undo().unwrap();
    doc.undo().unwrap();
    let mesh = mesh_of(&doc, id);
    assert!(mesh::uvs::budgets(&mesh).is_none());
    assert!(mesh::uvs::seam_preferences(&mesh).is_empty());
    // Labels, and budgets out of range are refused.
    let op = Operation::SetStretchBudget {
        faces: faces.clone(),
        budget: Some(50.0),
        anchors: None,
    };
    assert!(!op.numbers_are_real());
    assert!(doc.execute(Command::Record { id, operation: op }).is_err());
    let clear = Operation::SetStretchBudget {
        faces,
        budget: None,
        anchors: None,
    };
    assert_eq!(clear.describe(), "Clear the stretch budget of 1 polygon");
    let never = Operation::SetSeamPreference {
        edges,
        preference: SeamPreference::Never,
        anchors: None,
    };
    assert_eq!(never.describe(), "Allow no seams on 2 edges");
}
