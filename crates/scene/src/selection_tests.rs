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

#[test]
fn selecting_objects_is_an_undo_step_too() {
    let (mut doc, cube) = cube_document();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    let steps = doc.undo_steps();

    doc.set_selection([cube]);
    assert_eq!(doc.undo_label(), Some("Select Cube"));
    doc.toggle_selected(group);
    assert_eq!(doc.undo_label(), Some("Select 2 objects"));
    doc.clear_selection();
    assert_eq!(doc.undo_label(), Some("Deselect all"));
    assert_eq!(doc.undo_steps(), steps + 3);
    let revision = doc.revision();

    doc.undo();
    assert_eq!(doc.selection(), vec![group, cube], "tree order");
    doc.undo();
    assert_eq!(doc.selection(), vec![cube]);
    doc.redo();
    assert_eq!(doc.selection(), vec![group, cube], "tree order");
    assert_eq!(doc.revision(), revision, "nothing to save");

    // Selecting what is already selected records nothing.
    doc.set_selection([group, cube]);
    assert_eq!(doc.undo_steps(), steps + 2);
}

#[test]
fn object_selections_merge_with_a_key() {
    let (mut doc, cube) = cube_document();
    doc.clear_selection();
    let steps = doc.undo_steps();
    doc.select_objects([cube], Some(3));
    doc.select_objects([], Some(3));
    assert_eq!(doc.undo_steps(), steps, "back where it started: no step");
    doc.select_objects([cube], Some(4));
    doc.select_objects([cube], Some(4));
    assert_eq!(doc.undo_steps(), steps + 1);
    doc.undo();
    assert!(doc.selection().is_empty());
}

#[test]
fn opening_a_scene_with_a_selection_records_no_step() {
    let (doc, cube) = cube_document();
    let opened = Document::from_scene(doc.scene().clone(), [cube]);
    assert_eq!(opened.undo_steps(), 0);
    assert_eq!(opened.selection(), vec![cube]);
}

// ------------------------------------------------- transforms (0030) ---

#[test]
fn objects_move_turn_and_scale_as_one_undo_step() {
    let (mut doc, cube) = cube_document();
    let moved = Transform {
        position: glam::Vec3::new(10.0, 0.0, 5.0),
        rotation: glam::Vec3::new(0.5, 0.0, 0.0),
        scale: glam::Vec3::splat(2.0),
    };
    doc.execute_labelled(
        Command::SetTransforms(vec![(cube, moved)]),
        "Move Cube",
        None,
    )
    .unwrap();
    assert_eq!(doc.scene().object(cube).unwrap().transform, moved);
    assert_eq!(doc.undo_label(), Some("Move Cube"));
    let revision = doc.revision();
    doc.undo();
    assert_eq!(
        doc.scene().object(cube).unwrap().transform,
        Transform::default()
    );
    assert!(doc.revision() > revision, "moving is a change to save");
    // A value that isn't a number is refused, and nothing changes.
    let broken = Transform {
        position: glam::Vec3::NAN,
        ..Transform::default()
    };
    assert_eq!(
        doc.execute(Command::SetTransforms(vec![(cube, broken)])),
        Err(SceneError::NotANumber)
    );
    assert_eq!(
        Command::SetTransforms(vec![(cube, moved)]).describe(doc.scene()),
        "Transform object"
    );
}

#[test]
fn a_drag_replaces_its_own_step_and_can_be_cancelled() {
    let (mut doc, cube) = cube_document();
    let steps = doc.undo_steps();
    let at = |x: f32| Transform {
        position: glam::Vec3::new(x, 0.0, 0.0),
        ..Transform::default()
    };
    // Each frame of a drag takes back the frame before and does the new
    // total: one step in the end.
    for x in [1.0, 2.0, 3.0] {
        doc.abandon(42);
        doc.execute_labelled(
            Command::SetTransforms(vec![(cube, at(x))]),
            "Move Cube",
            Some(42),
        )
        .unwrap();
    }
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.scene().object(cube).unwrap().transform, at(3.0));
    assert_eq!(doc.redo_steps(), 0, "nothing left to redo");
    // Cancelling: everything back as it was.
    assert!(doc.abandon(42));
    assert_eq!(
        doc.scene().object(cube).unwrap().transform,
        Transform::default()
    );
    assert_eq!(doc.undo_steps(), steps);
    assert_eq!(doc.redo_steps(), 0);
    // Only a step made with that key is taken back.
    assert!(!doc.abandon(42));
    assert!(!doc.abandon(7));
    assert_eq!(doc.undo_steps(), steps);
}

#[test]
fn points_turn_with_a_recorded_matrix() {
    let (mut doc, cube) = cube_document();
    let points: Vec<VertexId> = (0..4).map(VertexId::from_raw).collect();
    let matrix = glam::Mat4::from_scale(glam::Vec3::splat(2.0));
    doc.execute(Command::Record {
        id: cube,
        operation: Operation::TransformPoints {
            points: points.clone(),
            matrix,
            anchors: None,
        },
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Transform 4 points"));
    let shape = doc.scene().evaluated(cube).unwrap();
    let base = PrimitiveKind::Cube.standard().mesh();
    for v in &points {
        let expected = base.position(*v).unwrap() * 2.0;
        assert!(shape.mesh.position(*v).unwrap().distance(expected) < 1e-3);
    }
}

#[test]
fn a_transform_round_trips_through_its_matrix() {
    let t = Transform {
        position: glam::Vec3::new(10.0, -20.0, 30.0),
        rotation: glam::Vec3::new(0.3, -0.4, 0.5),
        scale: glam::Vec3::new(1.0, 2.0, 0.5),
    };
    let back = Transform::from_matrix(t.matrix()).unwrap();
    assert!(back.matrix().abs_diff_eq(t.matrix(), 1e-4), "{back:?}");
    assert!(back.position.abs_diff_eq(t.position, 1e-4));
    assert!(back.rotation.abs_diff_eq(t.rotation, 1e-4));
    assert!(back.scale.abs_diff_eq(t.scale, 1e-4));
    assert_eq!(Transform::from_matrix(glam::Mat4::ZERO), None, "flat");
    assert_eq!(
        Transform::from_matrix(glam::Mat4::from_scale(glam::Vec3::NAN)),
        None
    );
}

#[test]
fn undo_and_redo_of_a_create_or_delete_put_the_selection_back() {
    // Regression: steps that choose the selection themselves didn't
    // record it, so undo and redo left it wrong.
    let (mut doc, id) = cube_document();
    doc.clear_selection();
    doc.undo();
    assert_eq!(doc.selection(), vec![id]);
    doc.undo();
    assert!(doc.selection().is_empty());
    doc.redo();
    assert_eq!(doc.selection(), vec![id], "redo of Create selects it again");

    doc.execute(Command::Delete(vec![id])).unwrap();
    assert!(doc.selection().is_empty());
    doc.undo();
    assert_eq!(doc.selection(), vec![id], "undo of Delete selects it again");
    doc.redo();
    assert!(doc.selection().is_empty());
}
