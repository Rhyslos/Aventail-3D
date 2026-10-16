//! Tests for edit layers in the window (decision 0091): the modifiers
//! palette's Edit layer button, the Mesh menu's entry and the Edit layers
//! panel.

use super::modelling_tests::{H, cube};
use super::tests::{fresh, main_menu, quick_harness};
use super::*;
use egui::{Event, PointerButton};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{Command, Operation, VertexId};

/// Each layer's name and how many edits it has, bottom first.
fn layers(harness: &H) -> Vec<(String, usize)> {
    let doc = harness.state().workbench.document();
    doc.scene()
        .object(cube(harness))
        .unwrap()
        .stack
        .layers()
        .iter()
        .map(|l| (l.name.clone(), l.entries.len()))
        .collect()
}

fn run(harness: &mut H, command: Command) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(command)
        .unwrap();
    harness.run();
}

/// Lifts the cube's first point.
fn lift(harness: &mut H) {
    let id = cube(harness);
    run(
        harness,
        Command::Record {
            id,
            operation: Operation::MovePoints {
                points: vec![VertexId::from_raw(0)],
                offset: glam::Vec3::Z * 10.0,
                anchors: None,
            },
        },
    );
}

/// The window with the cube selected, an edit in "Edits" and an empty
/// "Edits 2" on top, and the Edit layers panel in front. Frames are close
/// enough together for double-clicks.
fn with_two_layers() -> H {
    let mut harness = quick_harness();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    lift(&mut harness);
    run(&mut harness, Command::AddLayer(vec![id]));
    harness.get_by_label("Edit layers").click();
    harness.run();
    harness
}

/// A layer's row in the Edit layers panel (the Attributes panel has an
/// "Edits" heading too, higher up).
fn row<'h>(harness: &'h H, name: &'h str) -> egui_kittest::Node<'h> {
    harness
        .get_all_by_label(name)
        .max_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        .unwrap_or_else(|| panic!("no layer {name}"))
}

/// Where a layer's row is, scrolled into view first.
fn row_at(harness: &mut H, name: &str) -> egui::Pos2 {
    row(harness, name).scroll_to_me();
    harness.run();
    row(harness, name).rect().center()
}

fn click_at(harness: &mut H, at: egui::Pos2, button: PointerButton) {
    harness.event(Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
}

#[test]
fn the_edit_layer_button_needs_a_shape_selected() {
    let harness = fresh();
    let button = harness.get_by_label(crate::layers::ADD_LAYER_LABEL);
    assert!(button.accesskit_node().is_disabled());
}

#[test]
fn the_edit_layer_button_adds_a_layer_and_shows_the_panel() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    let button = harness.get_by_label(crate::layers::ADD_LAYER_LABEL);
    // It sits on the right of the viewport, left of the panels.
    let viewport = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let objects = harness.get_by_label("Objects").rect();
    assert!(button.rect().min.x > viewport.center().x);
    assert!(button.rect().max.x <= objects.min.x + 1.0);
    button.click();
    harness.run();
    assert_eq!(layers(&harness), vec![("Edits".to_string(), 0)]);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Add edit layer")
    );
    // The Edit layers panel came to the front and lists the layer.
    assert!(row(&harness, "Edits").rect().min.y > objects.min.y);
    assert!(harness.query_by_label_contains("No edits").is_some());
}

#[test]
fn the_mesh_menu_adds_a_layer_too() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    main_menu(&harness, "Mesh").click();
    harness.run();
    harness
        .get_all_by_label(crate::layers::ADD_LAYER_LABEL)
        .max_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        .unwrap()
        .click();
    harness.run();
    assert_eq!(layers(&harness).len(), 1);
}

#[test]
fn the_panel_lists_layers_top_first_with_their_edits() {
    let harness = with_two_layers();
    let top = row(&harness, "Edits 2").rect();
    let bottom = row(&harness, "Edits").rect();
    assert!(top.min.y < bottom.min.y, "the top layer is listed first");
    assert!(harness.query_by_label_contains("Move 1 point").is_some());
    assert!(harness.query_by_label_contains("new edits").is_some());
}

#[test]
fn the_eye_switches_a_layer_off() {
    let mut harness = with_two_layers();
    harness.get_by_label("Switch off Edits").click();
    harness.run();
    let doc = harness.state().workbench.document();
    let stack = &doc.scene().object(cube(&harness)).unwrap().stack;
    assert!(!stack.layers()[0].enabled);
    assert_eq!(doc.undo_label(), Some("Switch layer off"));
}

#[test]
fn the_right_click_menu_deletes_a_layer() {
    let mut harness = with_two_layers();
    let at = row_at(&mut harness, "Edits");
    click_at(&mut harness, at, PointerButton::Secondary);
    harness.get_by_label("Delete layer").click();
    harness.run();
    assert_eq!(layers(&harness), vec![("Edits 2".to_string(), 0)]);
}

#[test]
fn the_layer_menu_moves_the_layer_clicked() {
    let mut harness = with_two_layers();
    let at = row_at(&mut harness, "Edits");
    click_at(&mut harness, at, PointerButton::Primary);
    harness.get_by_label(crate::layers::LAYER_MENU).click();
    harness.run();
    harness.get_by_label("Move up").click();
    harness.run();
    assert_eq!(
        layers(&harness),
        vec![("Edits 2".to_string(), 0), ("Edits".to_string(), 1)]
    );
}

#[test]
fn double_clicking_a_layer_renames_it() {
    let mut harness = with_two_layers();
    let at = row_at(&mut harness, "Edits 2");
    // Let a second pass after the window opened (egui would take clicks
    // so soon after its start for a triple-click).
    for _ in 0..60 {
        harness.step();
    }
    harness.event(Event::PointerMoved(at));
    harness.step();
    for _ in 0..2 {
        for pressed in [true, false] {
            harness.event(Event::PointerButton {
                pos: at,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
            harness.step();
        }
    }
    harness.run();
    assert!(harness.state().workbench.layers_panel().is_renaming());
    harness.event(Event::Text("Details".into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(layers(&harness)[1].0, "Details");
    assert!(!harness.state().workbench.layers_panel().is_renaming());
}

#[test]
fn the_panel_explains_when_it_has_nothing_to_show() {
    let mut harness = fresh();
    harness.get_by_label("Edit layers").click();
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Select an object to see its edit layers")
            .is_some()
    );
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    assert!(
        harness
            .query_by_label_contains("No edit layers yet")
            .is_some()
    );
}
