//! Tests for when the Last edit can be changed (decision 0120): right
//! after an edit, the panel and Ctrl + wheel change it; once something
//! else is selected, they set the tool for its next use, and the old edit
//! is changed only when opened on purpose.

use super::modelling_tests::{H, ctrl_wheel, cube, drag, last_operation, on_screen, ready, shape};
use super::*;
use egui_kittest::kittest::Queryable;
use scene::{Element, ElementSelection, Operation};

fn steps_of_last(harness: &H) -> u32 {
    match last_operation(harness) {
        Some(Operation::ExtrudePolygons { shape, .. }) => shape.steps,
        other => panic!("not a straight extrude: {other:?}"),
    }
}

/// Selects the cube's bottom polygon instead of what is selected.
fn select_the_bottom(harness: &mut H) {
    let id = cube(harness);
    let mesh = shape(harness);
    let bottom = mesh
        .faces()
        .map(|(f, _)| f)
        .min_by(|a, b| mesh.face_normal(*a).z.total_cmp(&mesh.face_normal(*b).z))
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Polygon(bottom)]))],
            "Select",
            None,
        );
    harness.run();
}

#[test]
fn after_selecting_something_else_the_wheel_sets_the_next_extrude_only() {
    let mut harness = ready(Key::D);
    let at = on_screen(&harness, glam::Vec3::new(0.0, 0.0, 100.0));
    drag(&mut harness, 60.0);
    assert_eq!(steps_of_last(&harness), 1);
    // Right after: the last extrude follows.
    ctrl_wheel(&mut harness, at, true);
    assert_eq!(steps_of_last(&harness), 2);
    // Something else selected: the last extrude stays as it is, and the
    // tool's steps change for the next one.
    select_the_bottom(&mut harness);
    let tool_steps = harness.state().workbench.attributes.extrude.steps;
    ctrl_wheel(&mut harness, at, true);
    assert_eq!(steps_of_last(&harness), 2, "the old extrude is left alone");
    assert_eq!(
        harness.state().workbench.attributes.extrude.steps,
        tool_steps + 1
    );
    // The next extrude uses them, and is then the one that follows.
    drag(&mut harness, 60.0);
    assert_eq!(steps_of_last(&harness), tool_steps + 1);
    ctrl_wheel(&mut harness, at, true);
    assert_eq!(steps_of_last(&harness), tool_steps + 2);
}

#[test]
fn the_panel_shows_the_tools_settings_and_a_way_back_to_the_old_edit() {
    let mut harness = ready(Key::D);
    drag(&mut harness, 60.0);
    assert!(
        harness
            .query_by_label_contains(crate::attributes::CHANGE_LAST_EDIT_LABEL)
            .is_none()
    );
    let id = cube(&harness);
    assert!(
        harness
            .state()
            .workbench
            .attributes
            .last_edit_is_live(harness.state().workbench.document(), id)
    );
    select_the_bottom(&mut harness);
    assert!(
        !harness
            .state()
            .workbench
            .attributes
            .last_edit_is_live(harness.state().workbench.document(), id)
    );
    // The tool's own rows are shown, for the next extrude.
    assert!(harness.query_all_by_label("Extrude steps").count() >= 1);
    // Change it opens the old extrude, to change it on purpose.
    harness
        .get_by_label(crate::attributes::CHANGE_LAST_EDIT_LABEL)
        .click();
    harness.run();
    assert!(harness.state().workbench.attributes.opened.is_some());
    assert!(
        harness
            .query_by_label_contains("Edit: Extrude 1 polygon")
            .is_some()
    );
}

#[test]
fn a_tool_moving_the_selection_on_keeps_its_edit_live() {
    // Extruding edges moves the selection to the new outer edges: that's
    // the tool's own doing, so the extrude can still be changed.
    let mut harness = ready(Key::D);
    drag(&mut harness, 60.0);
    let id = cube(&harness);
    select_the_bottom(&mut harness);
    let bench = &mut harness.state_mut().workbench;
    assert!(!bench.attributes.last_edit_is_live(&bench.document, id));
    bench.attributes.keep_last_edit_live(&bench.document);
    assert!(bench.attributes.last_edit_is_live(&bench.document, id));
}
