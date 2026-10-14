//! Tests for the basics in the window: hide, isolate and show everything
//! (objects and polygons), and copying, pasting, duplicating and
//! instancing objects.

use super::modelling_tests::{H, cube, note, press, shape};
use super::tests::fresh;
use super::*;
use crate::commands::CommandId;
use egui::Event;
use egui_kittest::kittest::Queryable;
use scene::{Element, ElementSelection, ObjectKind};

fn keys(harness: &mut H, modifiers: Modifiers, key: Key) {
    harness.event(Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    });
    harness.run();
}

fn selected_cube() -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness
}

fn visible(harness: &H, id: scene::ObjectId) -> bool {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .visible
}

#[test]
fn shift_h_hides_objects_and_alt_h_shows_them() {
    let mut harness = selected_cube();
    let id = cube(&harness);
    keys(&mut harness, Modifiers::SHIFT, Key::H);
    assert!(!visible(&harness, id));
    assert!(harness.state().workbench.document().selection().is_empty());
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Hide object")
    );
    keys(&mut harness, Modifiers::ALT, Key::H);
    assert!(visible(&harness, id));
    assert!(note(&harness).starts_with("Showing 1 hidden object"));
}

#[test]
fn ctrl_h_isolates_the_selected_object() {
    let mut harness = selected_cube();
    let id = cube(&harness);
    // A second object, then the cube selected again.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Create(ObjectKind::Group))
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    keys(&mut harness, Modifiers::COMMAND, Key::H);
    let doc = harness.state().workbench.document();
    let hidden = doc
        .scene()
        .walk()
        .into_iter()
        .filter(|(o, _)| !doc.scene().object(*o).unwrap().visible)
        .count();
    assert_eq!(hidden, 1, "the group");
    assert!(visible(&harness, id));
    assert!(note(&harness).starts_with("Isolated"));
}

#[test]
fn polygons_hide_and_isolate_and_come_back() {
    let mut harness = selected_cube();
    let id = cube(&harness);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let top: Vec<Element> = {
        let mesh = shape(&harness);
        mesh.faces()
            .map(|(f, _)| f)
            .filter(|f| mesh.face_normal(*f).z > 0.9)
            .map(Element::Polygon)
            .collect()
    };
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(top))], "Select", None);
    harness.run();
    keys(&mut harness, Modifiers::SHIFT, Key::H);
    let hidden = |h: &H| {
        h.state()
            .workbench
            .hidden
            .faces
            .get(&id)
            .map_or(0, |f| f.len())
    };
    assert_eq!(hidden(&harness), 1);
    // Not deleted: only out of view, and unselected.
    assert_eq!(shape(&harness).face_count(), 6);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.elements(id).map_or(0, |e| e.polygons().count()), 0);
    assert!(
        harness
            .query_by_label_contains("1 polygon hidden")
            .is_some()
    );
    // Select all now selects only what is shown.
    keys(&mut harness, Modifiers::COMMAND, Key::A);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.elements(id).unwrap().polygons().count(), 5);
    // Isolating those leaves nothing more hidden; isolating one side
    // hides the other four.
    let side = {
        let mesh = shape(&harness);
        mesh.faces()
            .map(|(f, _)| f)
            .find(|f| mesh.face_normal(*f).x > 0.9)
            .unwrap()
    };
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Polygon(side)]))],
            "Select",
            None,
        );
    harness.run();
    keys(&mut harness, Modifiers::COMMAND, Key::H);
    assert_eq!(hidden(&harness), 5);
    keys(&mut harness, Modifiers::ALT, Key::H);
    assert_eq!(hidden(&harness), 0);
    assert!(note(&harness).starts_with("Showing 5 hidden polygons"));
}

#[test]
fn ctrl_c_and_ctrl_v_paste_copies_and_alt_v_instances() {
    let mut harness = selected_cube();
    let id = cube(&harness);
    keys(&mut harness, Modifiers::COMMAND, Key::C);
    assert!(note(&harness).starts_with("Copied 1 object"));
    keys(&mut harness, Modifiers::COMMAND, Key::V);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().children(None).len(), 2);
    assert_eq!(doc.undo_label(), Some("Duplicate object"));
    keys(&mut harness, Modifiers::ALT, Key::V);
    let doc = harness.state().workbench.document();
    let instance = doc.selection()[0];
    assert_eq!(
        doc.scene().object(instance).unwrap().kind,
        ObjectKind::Instance(id)
    );
    // The system's Paste with the app's note pastes too; other text is
    // left alone.
    harness.event(Event::Paste(copying::CLIPBOARD_NOTE.into()));
    harness.run();
    assert_eq!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .children(None)
            .len(),
        4
    );
    harness.event(Event::Paste("some other text".into()));
    harness.run();
    assert_eq!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .children(None)
            .len(),
        4
    );
}

#[test]
fn duplicate_and_instance_from_the_tool_menu_and_copying_needs_objects() {
    let mut harness = selected_cube();
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(CommandId::Instance, &ctx, &mut Vec::new());
    // Run through the document as the workbench would.
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_command(CommandId::Duplicate, &ctx, &mut requests);
    assert!(matches!(
        requests.as_slice(),
        [crate::objects::DocRequest::Run(scene::Command::Duplicate {
            instances: false,
            ..
        })]
    ));
    // In the Points mode, copying explains where it works.
    press(&mut harness, Modifiers::NONE, Key::Num1);
    keys(&mut harness, Modifiers::COMMAND, Key::C);
    assert!(note(&harness).starts_with("Copy works on whole objects"));
}

#[test]
fn split_off_makes_the_selected_polygons_an_object_and_join_puts_them_back() {
    let mut harness = selected_cube();
    let id = cube(&harness);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let top: Vec<Element> = {
        let mesh = shape(&harness);
        mesh.faces()
            .map(|(f, _)| f)
            .filter(|f| mesh.face_normal(*f).z > 0.9)
            .map(Element::Polygon)
            .collect()
    };
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(top))], "Select", None);
    harness.run();
    harness.state_mut().workbench.run_split_off();
    harness.run();
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().children(None).len(), 2);
    let piece = doc.selection()[0];
    assert_ne!(piece, id);
    assert!(note(&harness).starts_with("Split off 1 polygon"));
    // Join them again from the Model mode's tool menu command.
    press(&mut harness, Modifiers::NONE, Key::Num3);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id, piece]);
    let ctx = harness.ctx.clone();
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_command(CommandId::Join, &ctx, &mut requests);
    harness.state_mut().workbench.carry_out(requests);
    harness.run();
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().children(None).len(), 1);
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    assert_eq!(mesh.face_count(), 6);
    // One shape alone can't be joined.
    harness
        .state_mut()
        .workbench
        .run_command(CommandId::Join, &ctx, &mut Vec::new());
    assert!(note(&harness).starts_with("Join: select two or more shapes"));
}
