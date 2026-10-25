//! Tests for triangles in the window (decision 0161): the Triangulate
//! modifier's palette button, Mesh menu entry, command search and
//! Attributes rows, and Display → Show triangles.

use super::modelling_tests::H;
use super::tests::{empty, mesh_group};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{Command, ModifierKind, ObjectId, ObjectKind, TriangulateMethod, TriangulateOptions};

/// A window with a cube, selected, in the Model mode.
fn cube() -> (H, ObjectId) {
    let mut harness = empty();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::Create(ObjectKind::Primitive(
        scene::PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    harness.run();
    (harness, id)
}

fn options(harness: &H, id: ObjectId) -> TriangulateOptions {
    match &harness.state().workbench.document().scene().modifiers(id)[0].kind {
        ModifierKind::Triangulate(options) => *options,
        other => panic!("{other:?}"),
    }
}

/// How many triangle lines the overlay holds.
fn triangle_lines(harness: &H) -> usize {
    harness
        .state()
        .workbench
        .components
        .overlay()
        .lines
        .iter()
        .filter(|(_, s)| *s == crate::OverlayStyle::Triangle)
        .count()
}

/// The Quads only switch in the Attributes panel.
fn switch(harness: &H) -> egui_kittest::Node<'_> {
    harness
        .get_all_by_label("Quads only")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("the Quads only switch")
}

/// Clicks the palette's Triangulate button (scrolled to first: it is near
/// the end of a palette that scrolls when the window is short).
fn click_palette_button(harness: &mut H) {
    harness.get_by_label("Triangulate").scroll_to_me();
    harness.run_steps(4);
    harness.get_by_label("Triangulate").click();
    harness.run();
}

#[test]
fn the_palette_button_cuts_the_cube_into_triangles() {
    let (mut harness, id) = cube();
    click_palette_button(&mut harness);
    assert_eq!(options(&harness, id), TriangulateOptions::default());
    let doc = harness.state().workbench.document();
    let shown = doc.scene().shown_shape(id).unwrap();
    assert_eq!(shown.face_count(), 12);
    assert_eq!(doc.undo_label(), Some("Add Triangulate"));
    for name in ["Method", "Quads only"] {
        assert!(harness.query_all_by_label(name).count() >= 1, "{name}");
    }
}

#[test]
fn quads_only_and_the_method_are_one_undo_step_each() {
    let (mut harness, id) = cube();
    click_palette_button(&mut harness);
    switch(&harness).scroll_to_me();
    harness.run_steps(4);
    switch(&harness).click();
    harness.run();
    assert!(options(&harness, id).quads_only);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change Triangulate")
    );
    harness
        .get_all_by_label("Method")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::ComboBox)
        .expect("the Method choice")
        .click();
    harness.run();
    harness.get_by_label("Fixed").click();
    harness.run();
    assert_eq!(options(&harness, id).method, TriangulateMethod::Fixed);
    let doc = harness.state_mut().workbench.document_mut();
    doc.undo();
    assert_eq!(options(&harness, id).method, TriangulateMethod::Beauty);
    assert!(options(&harness, id).quads_only);
}

#[test]
fn the_mesh_menu_and_command_search_offer_it_under_game_ready() {
    let (mut harness, id) = cube();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries.iter().find(|e| e.name == "Triangulate").unwrap();
    assert_eq!(entry.place, "Mesh");
    assert!(entry.not_now.is_none());
    let show = entries.iter().find(|e| e.name == "Show triangles").unwrap();
    assert_eq!(show.place, "Display");
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    harness
        .get_all_by_label("Triangulate")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Triangulate can be chosen")
        .click();
    harness.run();
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().modifiers(id).len(), 1);
    assert_eq!(doc.undo_label(), Some("Add Triangulate"));
}

#[test]
fn show_triangles_draws_the_exports_cut_for_every_shape() {
    let (mut harness, _) = cube();
    assert_eq!(triangle_lines(&harness), 0, "off as standard");
    harness.get_by_label("Display").click();
    harness.run();
    harness.get_by_label("Show triangles").click();
    harness.run();
    assert!(harness.state().workbench.viewport.options().show_triangles);
    // One line across each of the cube's six sides, selected or not.
    assert_eq!(triangle_lines(&harness), 6);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    assert_eq!(triangle_lines(&harness), 6);
    // A shape that is all triangles has nothing more to show.
    let doc = harness.state_mut().workbench.document_mut();
    let id = doc.scene().walk()[0].0;
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: ModifierKind::Triangulate(TriangulateOptions::default()),
    })
    .unwrap();
    harness.run();
    assert_eq!(triangle_lines(&harness), 0);
    // Switched off again from command search's action.
    harness
        .state_mut()
        .workbench
        .viewport
        .options_mut()
        .show_triangles = false;
    harness.run();
    assert_eq!(triangle_lines(&harness), 0);
}
