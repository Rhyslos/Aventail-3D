//! Tests for managing edit layers (decision 0091): adding, switching,
//! renaming, moving, merging and deleting them, and which layer new edits
//! go into.

use crate::*;
use glam::Vec3;

/// A document with one standard cube (selected), and its id.
fn with_a_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

/// Lifts point 0 by `by`.
fn lift(by: f32) -> Operation {
    Operation::MovePoints {
        points: vec![VertexId::from_raw(0)],
        offset: Vec3::Z * by,
        anchors: None,
    }
}

fn record(doc: &mut Document, id: ObjectId, by: f32) {
    doc.execute(Command::Record {
        id,
        operation: lift(by),
    })
    .unwrap();
}

fn height(doc: &Document, id: ObjectId) -> f32 {
    doc.scene()
        .evaluated(id)
        .unwrap()
        .mesh
        .position(VertexId::from_raw(0))
        .unwrap()
        .z
}

/// Each layer's name and edit count, bottom first.
fn layers(doc: &Document, id: ObjectId) -> Vec<(String, usize)> {
    doc.scene()
        .object(id)
        .unwrap()
        .stack
        .layers()
        .iter()
        .map(|l| (l.name.clone(), l.entries.len()))
        .collect()
}

fn change(doc: &mut Document, id: ObjectId, index: usize, change: LayerChange) {
    doc.execute(Command::ChangeLayer { id, index, change })
        .unwrap();
}

fn named(list: &[(&str, usize)]) -> Vec<(String, usize)> {
    list.iter().map(|(n, c)| (n.to_string(), *c)).collect()
}

/// A cube with an edit in "Edits" (lifting by 10) and one in "Edits 2"
/// (lifting by 5).
fn two_layers() -> (Document, ObjectId, f32) {
    let (mut doc, id) = with_a_cube();
    let start = height(&doc, id);
    record(&mut doc, id, 10.0);
    doc.execute(Command::AddLayer(vec![id])).unwrap();
    record(&mut doc, id, 5.0);
    (doc, id, start)
}

#[test]
fn an_added_layer_is_named_after_the_others_and_takes_new_edits() {
    let (mut doc, id) = with_a_cube();
    doc.execute(Command::AddLayer(vec![id])).unwrap();
    assert_eq!(doc.undo_label(), Some("Add edit layer"));
    assert_eq!(layers(&doc, id), named(&[("Edits", 0)]));
    doc.execute(Command::AddLayer(vec![id])).unwrap();
    record(&mut doc, id, 10.0);
    assert_eq!(layers(&doc, id), named(&[("Edits", 0), ("Edits 2", 1)]));
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(layers(&doc, id), named(&[("Edits", 0)]));
}

#[test]
fn groups_and_instances_get_no_layer() {
    let (mut doc, cube) = with_a_cube();
    doc.execute(Command::Duplicate {
        ids: vec![cube],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    doc.execute(Command::AddLayer(vec![cube, instance, group]))
        .unwrap();
    assert_eq!(layers(&doc, cube).len(), 1);
    assert!(layers(&doc, instance).is_empty());
    assert!(layers(&doc, group).is_empty());
}

#[test]
fn a_switched_off_layer_hides_its_edits_and_new_edits_go_below_it() {
    let (mut doc, id, start) = two_layers();
    assert!((height(&doc, id) - (start + 15.0)).abs() < 1e-4);
    change(&mut doc, id, 1, LayerChange::SetEnabled(false));
    assert_eq!(doc.undo_label(), Some("Switch layer off"));
    assert!((height(&doc, id) - (start + 10.0)).abs() < 1e-4);
    // The top layer is off: the next edit goes into the one below.
    record(&mut doc, id, 1.0);
    assert_eq!(layers(&doc, id), named(&[("Edits", 2), ("Edits 2", 1)]));
    // Every layer off: a new one is added for it.
    change(&mut doc, id, 0, LayerChange::SetEnabled(false));
    assert!((height(&doc, id) - start).abs() < 1e-4);
    record(&mut doc, id, 2.0);
    assert_eq!(
        layers(&doc, id),
        named(&[("Edits", 2), ("Edits 2", 1), ("Edits 3", 1)])
    );
    assert!((height(&doc, id) - (start + 2.0)).abs() < 1e-4);
}

#[test]
fn switching_a_layer_to_what_it_is_records_nothing() {
    let (mut doc, id, _) = two_layers();
    change(&mut doc, id, 1, LayerChange::SetEnabled(true));
    assert_eq!(doc.undo_label(), Some("Move 1 point"));
}

#[test]
fn a_layer_is_renamed_trimmed_and_never_empty() {
    let (mut doc, id, _) = two_layers();
    change(&mut doc, id, 1, LayerChange::Rename("  Details ".into()));
    assert_eq!(doc.undo_label(), Some("Rename layer"));
    assert_eq!(layers(&doc, id)[1].0, "Details");
    let refused = doc.execute(Command::ChangeLayer {
        id,
        index: 1,
        change: LayerChange::Rename("   ".into()),
    });
    assert_eq!(refused, Err(SceneError::EmptyName));
    assert_eq!(layers(&doc, id)[1].0, "Details");
    // The same name again records nothing.
    change(&mut doc, id, 1, LayerChange::Rename("Details".into()));
    doc.undo().unwrap();
    assert_eq!(layers(&doc, id)[1].0, "Edits 2");
}

#[test]
fn layers_move_up_and_down_but_not_past_the_ends() {
    let (mut doc, id, start) = two_layers();
    change(&mut doc, id, 0, LayerChange::Move { up: true });
    assert_eq!(doc.undo_label(), Some("Move layer up"));
    assert_eq!(layers(&doc, id), named(&[("Edits 2", 1), ("Edits", 1)]));
    // Moving these two lifts doesn't change what they add up to.
    assert!((height(&doc, id) - (start + 15.0)).abs() < 1e-4);
    change(&mut doc, id, 1, LayerChange::Move { up: false });
    assert_eq!(layers(&doc, id), named(&[("Edits", 1), ("Edits 2", 1)]));
    for (index, up) in [(1, true), (0, false)] {
        let refused = doc.execute(Command::ChangeLayer {
            id,
            index,
            change: LayerChange::Move { up },
        });
        assert!(matches!(refused, Err(SceneError::History(_))));
    }
    assert_eq!(layers(&doc, id), named(&[("Edits", 1), ("Edits 2", 1)]));
}

#[test]
fn merging_down_keeps_the_edits_in_order_and_the_shape() {
    let (mut doc, id, start) = two_layers();
    let before = doc.scene().object(id).unwrap().stack.clone();
    change(&mut doc, id, 1, LayerChange::MergeDown);
    assert_eq!(doc.undo_label(), Some("Merge layer down"));
    assert_eq!(layers(&doc, id), named(&[("Edits", 2)]));
    let merged = &doc.scene().object(id).unwrap().stack.layers()[0];
    let ids: Vec<u64> = merged.entries.iter().map(|e| e.id.get()).collect();
    assert_eq!(ids, vec![1, 2]);
    assert!((height(&doc, id) - (start + 15.0)).abs() < 1e-4);
    doc.undo().unwrap();
    assert_eq!(doc.scene().object(id).unwrap().stack, before);
}

#[test]
fn merging_is_refused_at_the_bottom_and_with_a_layer_off() {
    let (mut doc, id, _) = two_layers();
    let bottom = doc.execute(Command::ChangeLayer {
        id,
        index: 0,
        change: LayerChange::MergeDown,
    });
    assert!(matches!(bottom, Err(SceneError::History(_))));
    change(&mut doc, id, 0, LayerChange::SetEnabled(false));
    let off = doc.execute(Command::ChangeLayer {
        id,
        index: 1,
        change: LayerChange::MergeDown,
    });
    assert!(matches!(off, Err(SceneError::History(_))));
    assert_eq!(layers(&doc, id), named(&[("Edits", 1), ("Edits 2", 1)]));
}

#[test]
fn deleting_a_layer_takes_its_edits_and_undo_brings_them_back() {
    let (mut doc, id, start) = two_layers();
    change(&mut doc, id, 0, LayerChange::Delete);
    assert_eq!(doc.undo_label(), Some("Delete layer"));
    assert_eq!(layers(&doc, id), named(&[("Edits 2", 1)]));
    assert!((height(&doc, id) - (start + 5.0)).abs() < 1e-4);
    // The last layer can go too; the next edit makes a new one.
    change(&mut doc, id, 0, LayerChange::Delete);
    assert!(layers(&doc, id).is_empty());
    record(&mut doc, id, 1.0);
    assert_eq!(layers(&doc, id), named(&[("Edits", 1)]));
    doc.undo().unwrap();
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(layers(&doc, id), named(&[("Edits", 1), ("Edits 2", 1)]));
    assert!((height(&doc, id) - (start + 15.0)).abs() < 1e-4);
}

#[test]
fn a_layer_that_isnt_there_is_refused() {
    let (mut doc, id, _) = two_layers();
    for change in [
        LayerChange::Delete,
        LayerChange::SetEnabled(false),
        LayerChange::Rename("x".into()),
    ] {
        let refused = doc.execute(Command::ChangeLayer {
            id,
            index: 2,
            change,
        });
        assert!(matches!(refused, Err(SceneError::History(_))));
    }
    let unknown = doc.execute(Command::AddLayer(vec![ObjectId::from_raw(999)]));
    assert!(matches!(unknown, Err(SceneError::UnknownObject(_))));
    assert_eq!(layers(&doc, id), named(&[("Edits", 1), ("Edits 2", 1)]));
}

#[test]
fn new_edit_ids_stay_unique_across_layers() {
    let (mut doc, id, _) = two_layers();
    change(&mut doc, id, 0, LayerChange::Delete);
    record(&mut doc, id, 1.0);
    let stack = &doc.scene().object(id).unwrap().stack;
    assert!(stack.repeated_edit_id().is_none());
    let ids: Vec<u64> = stack.layers()[0]
        .entries
        .iter()
        .map(|e| e.id.get())
        .collect();
    assert_eq!(ids, vec![2, 3]);
}
