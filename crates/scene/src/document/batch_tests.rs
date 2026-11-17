//! Batches of changes as one undo step, and holding the document
//! (decision 0302).

use crate::*;

fn cube() -> Command {
    Command::Create(ObjectKind::Primitive(PrimitiveKind::Cube.standard()))
}

fn points(ids: &[u32]) -> ElementSelection {
    ElementSelection::of(ids.iter().map(|n| Element::Point(VertexId::from_raw(*n))))
}

/// The objects of `scene` in order, with their parents: the scene as a
/// person sees it (undo never takes back the next id, which is never
/// used again).
fn objects(scene: &Scene) -> Vec<(ObjectId, Option<ObjectId>, Object)> {
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| (id, scene.parent(id), scene.object(id).unwrap().clone()))
        .collect()
}

/// A document with one cube, selected, and its id.
fn one_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(cube()).unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

#[test]
fn a_batch_is_one_undo_step_and_comes_back_on_redo() {
    let (mut doc, first) = one_cube();
    let before = doc.clone();
    let steps = doc.undo_steps();
    doc.execute_batch(
        [
            Change::Command(cube()),
            Change::Command(Command::Rename {
                id: first,
                name: "Box".into(),
            }),
            Change::SelectObjects(vec![first]),
            Change::SelectElements(first, points(&[0, 1])),
        ],
        "Tidy",
    )
    .unwrap();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Tidy"));
    assert_eq!(doc.scene().walk().len(), 2);
    assert_eq!(doc.scene().object(first).unwrap().name, "Box");
    assert_eq!(doc.selection(), vec![first]);
    assert_eq!(doc.elements(first), Some(&points(&[0, 1])));
    let after = doc.clone();

    assert_eq!(doc.undo().as_deref(), Some("Tidy"));
    assert_eq!(objects(doc.scene()), objects(before.scene()));
    assert_eq!(doc.selection(), before.selection());
    assert_eq!(doc.elements(first), None);
    assert_eq!(doc.redo().as_deref(), Some("Tidy"));
    assert_eq!(doc.scene(), after.scene());
    assert_eq!(doc.selection(), after.selection());
    assert_eq!(doc.elements(first), Some(&points(&[0, 1])));
}

#[test]
fn a_refused_change_takes_back_everything_before_it() {
    let (mut doc, first) = one_cube();
    doc.select_elements([(first, points(&[2]))], "Select", None);
    let before = doc.clone();
    let refused = doc
        .execute_batch(
            [
                Change::Command(cube()),
                Change::Command(Command::Delete(vec![first])),
                Change::SelectElements(first, points(&[3])),
                Change::Command(Command::Rename {
                    id: first,
                    name: "Gone".into(),
                }),
            ],
            "Breaks",
        )
        .unwrap_err();
    assert_eq!(refused.at, 3);
    assert_eq!(refused.why, SceneError::UnknownObject(first));
    assert_eq!(objects(doc.scene()), objects(before.scene()));
    assert_eq!(doc.selection(), before.selection());
    assert_eq!(doc.selection_order(), before.selection_order());
    assert_eq!(doc.elements(first), Some(&points(&[2])));
    assert_eq!(doc.undo_steps(), before.undo_steps());
    assert_eq!(doc.undo_label(), before.undo_label());
}

#[test]
fn a_batch_that_changes_nothing_records_nothing() {
    let (mut doc, first) = one_cube();
    let steps = doc.undo_steps();
    doc.execute_batch(std::iter::empty(), "Nothing").unwrap();
    doc.execute_batch(
        [
            Change::SelectObjects(vec![first]),
            Change::SelectElements(first, points(&[1])),
            Change::SelectElements(first, ElementSelection::new()),
            // Unknown objects are skipped.
            Change::SelectElements(ObjectId::from_raw(999), points(&[1])),
        ],
        "Back where it was",
    )
    .unwrap();
    assert_eq!(doc.undo_steps(), steps);
}

#[test]
fn a_batch_ends_as_the_same_changes_made_one_by_one() {
    let (mut whole, first) = one_cube();
    let mut one_by_one = whole.clone();
    let changes = vec![
        Change::Command(cube()),
        Change::Command(Command::Group(vec![first])),
        Change::Command(cube()),
        Change::SelectObjects(vec![first]),
        Change::Command(Command::SetVisible {
            ids: vec![first],
            visible: false,
        }),
    ];
    for change in changes.clone() {
        one_by_one.execute_batch([change], "One").unwrap();
    }
    whole.execute_batch(changes, "All").unwrap();
    assert_eq!(whole.scene(), one_by_one.scene());
    assert_eq!(whole.selection(), one_by_one.selection());
    assert_eq!(whole.undo_steps() + 4, one_by_one.undo_steps());
}

#[test]
fn a_held_document_turns_every_change_away() {
    let (mut doc, first) = one_cube();
    doc.execute(cube()).unwrap();
    doc.undo();
    let before = doc.clone();
    doc.hold();
    assert!(doc.is_held());
    assert_eq!(doc.execute(cube()), Err(SceneError::Held));
    assert_eq!(doc.execute_merging(cube(), 7), Err(SceneError::Held));
    assert_eq!(doc.execute_with_last(cube()), Err(SceneError::Held));
    assert_eq!(
        doc.execute_all_labelled([cube()], "All", 8),
        Err(SceneError::Held)
    );
    assert_eq!(doc.undo(), None);
    assert_eq!(doc.redo(), None);
    assert!(!doc.abandon(7));
    doc.clear_selection();
    doc.toggle_selected(first);
    assert!(!doc.select_elements([(first, points(&[1]))], "Select", None));
    assert!(!doc.set_bake_settings("x"));
    assert_eq!(
        doc.execute_batch([Change::Command(cube())], "Script")
            .unwrap_err()
            .why,
        SceneError::Held
    );
    assert!(!SceneError::Held.to_string().is_empty());
    doc.release();
    assert!(!doc.is_held());
    assert_eq!(doc.scene(), before.scene());
    assert_eq!(doc.selection(), before.selection());
    assert_eq!(doc.undo_steps(), before.undo_steps());
    assert_eq!(doc.redo_steps(), before.redo_steps());
    // Released, it changes again.
    doc.execute(cube()).unwrap();
    assert_eq!(doc.scene().walk().len(), 2);
}

#[test]
fn many_small_changes_land_quickly() {
    let (mut doc, first) = one_cube();
    let changes: Vec<Change> = (0..10_000)
        .map(|n| {
            Change::Command(Command::Rename {
                id: first,
                name: format!("Box {n}"),
            })
        })
        .collect();
    let start = std::time::Instant::now();
    doc.execute_batch(changes, "Renames").unwrap();
    let took = start.elapsed();
    assert_eq!(doc.scene().object(first).unwrap().name, "Box 9999");
    assert!(took.as_secs() < 5, "10,000 changes took {took:?}");
    doc.undo();
    assert_eq!(doc.scene().object(first).unwrap().name, "Cube");
}
