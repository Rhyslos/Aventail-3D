//! Tests for the Snapshots menu's workings (decision 0114).

use super::*;
use scene::{ObjectKind, Operation, PrimitiveKind};

fn with_a_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

fn layout(doc: &Document, id: ObjectId) -> scene::UvLayout {
    scene::UvLayout::of(&doc.scene().evaluated(id).unwrap().mesh).unwrap()
}

#[test]
fn snapshots_are_named_in_turn_and_never_clash() {
    let (mut doc, _) = with_a_cube();
    assert!(take(&mut doc, None).unwrap().contains("Snapshot 1"));
    assert!(take(&mut doc, None).unwrap().contains("Snapshot 2"));
    assert!(take(&mut doc, Some("Snapshot 1")).is_ok());
    let names: Vec<String> = doc
        .scene()
        .snapshots()
        .iter()
        .map(|s| s.name.clone())
        .collect();
    assert_eq!(names, ["Snapshot 1", "Snapshot 2", "Snapshot 1 2"]);
}

#[test]
fn nothing_shown_means_no_snapshot() {
    let (mut doc, _) = with_a_cube();
    doc.clear_selection();
    let said = take(&mut doc, None).unwrap_err();
    assert!(said.contains("Select objects"), "{said}");
    assert!(doc.scene().snapshots().is_empty());
}

#[test]
fn a_full_list_says_so() {
    let (mut doc, _) = with_a_cube();
    for _ in 0..scene::MAX_SNAPSHOTS {
        take(&mut doc, None).unwrap();
    }
    let said = take(&mut doc, None).unwrap_err();
    assert!(said.contains("at most"), "{said}");
}

#[test]
fn restoring_is_one_step_even_for_several_objects() {
    let (mut doc, a) = with_a_cube();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let b = doc.selection()[0];
    doc.set_selection([a, b]);
    take(&mut doc, Some("Start")).unwrap();
    let (la, lb) = (layout(&doc, a), layout(&doc, b));
    for id in [a, b] {
        let faces = doc
            .scene()
            .evaluated(id)
            .unwrap()
            .mesh
            .faces()
            .map(|(f, _)| f)
            .collect();
        doc.execute(Command::Record {
            id,
            operation: Operation::TransformUvs {
                faces,
                points: Vec::new(),
                transform: glam::Affine2::from_translation(glam::Vec2::splat(0.3)),
                anchors: None,
            },
        })
        .unwrap();
    }
    let mut editor = UvEditor::default();
    let steps = doc.undo_steps();
    assert!(editor.restore_snapshot(&mut doc, 0));
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Restore the snapshot “Start”"));
    assert_eq!((layout(&doc, a), layout(&doc, b)), (la, lb));
    // A second restore is a step of its own.
    assert!(editor.restore_snapshot(&mut doc, 0));
    assert_eq!(doc.undo_steps(), steps + 2);
    assert!(!editor.restore_snapshot(&mut doc, 5), "no such snapshot");
}

#[test]
fn nothing_is_compared_at_first() {
    assert_eq!(UvEditor::default().compared(), None);
}
