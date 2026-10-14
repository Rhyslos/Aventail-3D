//! Tests for the Points, Edges and Polygons modes: the mode keys, and the
//! elements of the selected objects shown and found under the mouse.

use super::tests::{State, camera, empty, fresh};
use super::*;
use egui::Event;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use glam::{Vec2, Vec3};
use std::sync::Arc;

fn press(harness: &mut Harness<'static, State>, key: Key) {
    harness.key_press(key);
    harness.run();
}

/// A window with the standard cube, selected.
fn selected_cube() -> Harness<'static, State> {
    let mut harness = fresh();
    let id = harness.state().workbench.document().scene().children(None)[0];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness
}

/// Moves the mouse to where `world` appears in the viewport.
fn hover(harness: &mut Harness<'static, State>, world: Vec3) {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).expect("in view");
    harness.event(Event::PointerMoved(rect.min + egui::vec2(at.x, at.y)));
    harness.run();
}

/// The overlay drawn in the last frame.
fn last_overlay(harness: &Harness<'static, State>) -> Arc<crate::ComponentOverlay> {
    let view = harness.state().renderer.drawn.last().expect("drawn");
    Arc::clone(&view.overlay)
}

#[test]
fn keys_one_two_three_switch_modes_and_again_go_back_to_model() {
    let mut harness = empty();
    assert_eq!(harness.state().workbench.mode(), Mode::Model);
    press(&mut harness, Key::Num1);
    assert_eq!(harness.state().workbench.mode(), Mode::Points);
    press(&mut harness, Key::Num2);
    assert_eq!(harness.state().workbench.mode(), Mode::Edges);
    press(&mut harness, Key::Num3);
    assert_eq!(harness.state().workbench.mode(), Mode::Polygons);
    press(&mut harness, Key::Num3);
    assert_eq!(harness.state().workbench.mode(), Mode::Model, "again: back");
}

#[test]
fn mode_keys_type_into_a_name_instead() {
    let mut harness = selected_cube();
    harness.get_by_label("Object name").click();
    harness.run();
    press(&mut harness, Key::Num1);
    assert_eq!(harness.state().workbench.mode(), Mode::Model);
}

#[test]
fn the_points_mode_shows_the_selected_objects_points() {
    let mut harness = selected_cube();
    let drawn = |h: &Harness<'static, State>| {
        let view = h.state().renderer.drawn.last().unwrap().clone();
        (view.scene.editing, view.scene.points)
    };
    assert_eq!(drawn(&harness), (false, false), "the Model mode shows none");
    press(&mut harness, Key::Num1);
    harness.run();
    assert_eq!(drawn(&harness), (true, true), "every point, with the edges");
    press(&mut harness, Key::Num2);
    harness.run();
    assert_eq!(drawn(&harness), (true, false), "edges only");
}

#[test]
fn nothing_selected_says_what_to_do() {
    let mut harness = fresh();
    press(&mut harness, Key::Num1);
    assert!(
        harness
            .query_by_label_contains("Select an object to see its points")
            .is_some()
    );
    assert!(last_overlay(&harness).points.is_empty());
}

#[test]
fn the_point_under_the_mouse_is_highlighted_and_named() {
    let mut harness = selected_cube();
    press(&mut harness, Key::Num1);
    // The cube's corner nearest the opening camera: front, right, top.
    let corner = Vec3::new(50.0, -50.0, 100.0);
    hover(&mut harness, corner);
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Under the mouse: point")
            .is_some(),
        "the status bar names it"
    );
    let overlay = last_overlay(&harness);
    let hovered: Vec<_> = overlay
        .points
        .iter()
        .filter(|(_, s)| *s == crate::OverlayStyle::Hovered)
        .collect();
    assert_eq!(hovered.len(), 1);
    assert!(hovered[0].0.distance(corner) < 1e-3);
}

#[test]
fn the_polygon_under_the_mouse_is_tinted() {
    let mut harness = selected_cube();
    press(&mut harness, Key::Num3);
    hover(&mut harness, Vec3::new(0.0, -50.0, 50.0));
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Under the mouse: polygon")
            .is_some()
    );
    assert_eq!(last_overlay(&harness).tints.len(), 2);
    // Back in the Model mode: nothing is drawn over the cube.
    press(&mut harness, Key::Num3);
    harness.run();
    let overlay = last_overlay(&harness);
    assert!(overlay.tints.is_empty() && overlay.lines.is_empty());
    assert!(harness.query_by_label_contains("Under the mouse").is_none());
}

#[test]
fn the_edge_under_the_mouse_is_found() {
    let mut harness = selected_cube();
    press(&mut harness, Key::Num2);
    hover(&mut harness, Vec3::new(0.0, -50.0, 100.0));
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Under the mouse: edge")
            .is_some()
    );
    assert_eq!(
        last_overlay(&harness)
            .lines
            .iter()
            .filter(|(_, s)| *s == crate::OverlayStyle::Hovered)
            .count(),
        1
    );
}

#[test]
fn every_mode_key_is_shown_on_its_mode() {
    for (key, mode) in MODE_KEYS {
        assert_eq!(mode.key(), Some(key), "{mode:?}");
    }
    let with_keys = Mode::ALL.iter().filter(|m| m.key().is_some()).count();
    assert_eq!(with_keys, MODE_KEYS.len());
}

#[test]
fn tab_steps_straight_back_to_the_model_mode() {
    let mut harness = selected_cube();
    // From the Edges mode with Extrude in use.
    press(&mut harness, Key::Num2);
    press(&mut harness, Key::D);
    assert_eq!(harness.state().workbench.tool(), Tool::Extrude);
    press(&mut harness, Key::Tab);
    assert_eq!(harness.state().workbench.mode(), Mode::Model);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    // Nothing took the keyboard: the keys still work afterwards, and Tab
    // again just says where it is.
    press(&mut harness, Key::Num1);
    assert_eq!(harness.state().workbench.mode(), Mode::Points);
    press(&mut harness, Key::Tab);
    press(&mut harness, Key::Tab);
    assert_eq!(harness.state().workbench.mode(), Mode::Model);
    assert!(
        harness
            .state()
            .workbench
            .note
            .as_deref()
            .unwrap()
            .starts_with("Already")
    );
    press(&mut harness, Key::Num3);
    assert_eq!(harness.state().workbench.mode(), Mode::Polygons);
    // Move, Rotate and Scale stay: they work on objects too.
    press(&mut harness, Key::W);
    press(&mut harness, Key::Tab);
    assert_eq!(harness.state().workbench.mode(), Mode::Model);
    assert_eq!(harness.state().workbench.tool(), Tool::Move);
}

#[test]
fn tab_moves_between_fields_while_typing() {
    let mut harness = selected_cube();
    press(&mut harness, Key::Num1);
    harness.get_by_label("Object name").click();
    harness.run();
    press(&mut harness, Key::Tab);
    assert_eq!(harness.state().workbench.mode(), Mode::Points);
}

#[test]
fn tab_finishes_a_slit_line() {
    let mut harness = selected_cube();
    press(&mut harness, Key::Num2);
    press(&mut harness, Key::X);
    assert_eq!(harness.state().workbench.tool(), Tool::Slit);
    press(&mut harness, Key::Tab);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    assert_eq!(harness.state().workbench.mode(), Mode::Model);
}

#[test]
fn a_mode_key_puts_a_tool_away_and_selects_in_that_mode() {
    let mut harness = selected_cube();
    // In the Edges mode with Slit: 2 keeps the Edges mode and selects.
    press(&mut harness, Key::Num2);
    press(&mut harness, Key::X);
    assert_eq!(harness.state().workbench.tool(), Tool::Slit);
    press(&mut harness, Key::Num2);
    assert_eq!(harness.state().workbench.mode(), Mode::Edges);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    assert!(
        harness
            .state()
            .workbench
            .note
            .as_deref()
            .unwrap()
            .contains("Slit put away")
    );
    // With Extrude: 3 goes to the Polygons mode, selecting.
    press(&mut harness, Key::D);
    assert_eq!(harness.state().workbench.tool(), Tool::Extrude);
    press(&mut harness, Key::Num3);
    assert_eq!(harness.state().workbench.mode(), Mode::Polygons);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    // With no tool (or Move), the same key goes back to the Model mode,
    // as before.
    press(&mut harness, Key::Num3);
    assert_eq!(harness.state().workbench.mode(), Mode::Model);
    press(&mut harness, Key::Num1);
    press(&mut harness, Key::W);
    press(&mut harness, Key::Num1);
    assert_eq!(harness.state().workbench.mode(), Mode::Model);
    assert_eq!(harness.state().workbench.tool(), Tool::Move);
}
