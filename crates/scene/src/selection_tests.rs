//! Tests for the selected points, edges and polygons kept by the document,
//! and their undo.

use crate::*;

fn cube_document() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

fn points(ids: &[u32]) -> ElementSelection {
    ElementSelection::of(ids.iter().map(|n| Element::Point(VertexId::from_raw(*n))))
}

#[test]
fn selecting_elements_is_an_undo_step() {
    let (mut doc, id) = cube_document();
    let steps = doc.undo_steps();
    let revision = doc.element_revision();
    assert!(doc.elements(id).is_none());

    assert!(doc.select_elements([(id, points(&[1, 2]))], "Select 2 points", None));
    assert_eq!(doc.elements(id), Some(&points(&[1, 2])));
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Select 2 points"));
    assert!(doc.element_revision() > revision);

    assert_eq!(doc.undo().as_deref(), Some("Select 2 points"));
    assert!(doc.elements(id).is_none());
    assert_eq!(doc.redo().as_deref(), Some("Select 2 points"));
    assert_eq!(doc.elements(id), Some(&points(&[1, 2])));
}

#[test]
fn the_same_selection_again_records_nothing() {
    let (mut doc, id) = cube_document();
    doc.select_elements([(id, points(&[3]))], "Select 1 point", None);
    let steps = doc.undo_steps();
    assert!(!doc.select_elements([(id, points(&[3]))], "Select 1 point", None));
    assert_eq!(doc.undo_steps(), steps);
    // Unknown objects are skipped.
    assert!(!doc.select_elements([(ObjectId::from_raw(999), points(&[1]))], "x", None));
}

#[test]
fn a_brush_stroke_merges_into_one_step() {
    let (mut doc, id) = cube_document();
    let steps = doc.undo_steps();
    for n in 1..=5 {
        let ids: Vec<u32> = (0..n).collect();
        doc.select_elements([(id, points(&ids))], format!("Select {n} points"), Some(7));
    }
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(
        doc.undo_label(),
        Some("Select 5 points"),
        "the latest label"
    );
    doc.undo();
    assert!(doc.elements(id).is_none(), "back to before the stroke");

    // Painting back to where it started leaves no step.
    doc.select_elements([(id, points(&[1]))], "a", Some(8));
    doc.select_elements([(id, ElementSelection::new())], "b", Some(8));
    assert_eq!(doc.undo_steps(), steps);
}

#[test]
fn one_step_can_change_several_objects() {
    let (mut doc, first) = cube_document();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Plane.standard(),
    )))
    .unwrap();
    let second = doc.selection()[0];
    doc.select_elements(
        [(first, points(&[0])), (second, points(&[1, 2]))],
        "Select 3 points",
        None,
    );
    doc.undo();
    assert!(doc.elements(first).is_none() && doc.elements(second).is_none());
    doc.redo();
    assert_eq!(doc.elements(second).map(|s| s.len()), Some(2));
}

#[test]
fn deleted_objects_lose_their_selected_elements() {
    let (mut doc, id) = cube_document();
    doc.select_elements([(id, points(&[0, 1]))], "Select 2 points", None);
    doc.execute(Command::Delete(vec![id])).unwrap();
    assert!(doc.elements(id).is_none());
    // Undoing the delete brings the object back; undoing once more goes
    // back past the selection step without trouble.
    doc.undo();
    assert!(doc.scene().contains(id));
    doc.undo();
    assert!(doc.elements(id).is_none());
    doc.redo();
    assert_eq!(doc.elements(id), Some(&points(&[0, 1])));
}

#[test]
fn selection_steps_count_towards_undo_memory() {
    let (mut doc, id) = cube_document();
    let before = doc.history_memory();
    let many: Vec<u32> = (0..1000).collect();
    doc.select_elements([(id, points(&many))], "Select 1000 points", None);
    assert!(doc.history_memory() > before + 1000 * 4);
    assert_eq!(doc.history_memory(), doc.history().measured());
}

#[test]
fn a_new_command_after_undo_forgets_the_selection_redo() {
    let (mut doc, id) = cube_document();
    doc.select_elements([(id, points(&[4]))], "Select 1 point", None);
    doc.undo();
    doc.execute(Command::Rename {
        id,
        name: "Box".into(),
    })
    .unwrap();
    assert_eq!(doc.redo_steps(), 0);
    assert!(doc.elements(id).is_none());
}
