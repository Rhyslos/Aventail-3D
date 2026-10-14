//! Tests for Loop cut in the window: the key, the loop shown while
//! pointing at an edge, a click cutting it as one undo step, dragging to
//! slide it, cancelling, and the Attributes panel's Cuts and Position.

use super::modelling_tests::{H, button, cube, last_operation, note, on_screen, press, shape};
use super::tests::fresh;
use super::*;
use crate::transforming::Tool;
use egui::{Event, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use scene::Operation;

/// The fresh window's cube selected, in the Edges mode, with Loop cut in
/// use.
fn ready() -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num2);
    press(&mut harness, Modifiers::NONE, Key::F);
    harness
}

/// The middle of the cube's top +X edge on screen (it runs along Y).
fn edge_middle(harness: &H) -> Pos2 {
    on_screen(harness, glam::Vec3::new(50.0, 0.0, 100.0))
}

fn hover(harness: &mut H, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.run();
    harness.run();
}

fn click(harness: &mut H, at: Pos2) {
    hover(harness, at);
    button(harness, at, true, PointerButton::Primary, Modifiers::NONE);
    button(harness, at, false, PointerButton::Primary, Modifiers::NONE);
}

/// The lines Loop cut shows over the cube.
fn shown_lines(harness: &H) -> usize {
    harness.state().workbench.components.overlay().lines.len()
}

fn loop_cut(harness: &H) -> (u32, f32) {
    match last_operation(harness) {
        Some(Operation::LoopCut { cuts, position, .. }) => (cuts, position),
        other => panic!("not a loop cut: {other:?}"),
    }
}

#[test]
fn f_picks_loop_cut_while_editing() {
    let mut harness = ready();
    assert_eq!(harness.state().workbench.tool(), Tool::LoopCut);
    assert!(note(&harness).starts_with("Loop cut: point at an edge"));
    // Not in the Model mode: it says where it works.
    press(&mut harness, Modifiers::NONE, Key::Q);
    press(&mut harness, Modifiers::NONE, Key::Num2);
    press(&mut harness, Modifiers::NONE, Key::F);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    assert!(
        note(&harness).starts_with("Loop cut works in"),
        "{}",
        note(&harness)
    );
}

#[test]
fn pointing_at_an_edge_shows_the_loop() {
    let mut harness = ready();
    let away = harness
        .get_by_label(crate::VIEWPORT_LABEL)
        .rect()
        .left_top()
        + egui::vec2(60.0, 80.0);
    hover(&mut harness, away);
    assert_eq!(shown_lines(&harness), 0);
    let middle = edge_middle(&harness);
    hover(&mut harness, middle);
    // One line across each of the four polygons round the cube.
    assert_eq!(shown_lines(&harness), 4);
    // Nothing is cut until the click.
    assert_eq!(shape(&harness).face_count(), 6);
    // Three cuts: three lines across each.
    harness.state_mut().workbench.attributes.loop_cut.cuts = 3;
    hover(&mut harness, middle + egui::vec2(1.0, 0.0));
    assert_eq!(shown_lines(&harness), 12);
}

#[test]
fn a_click_cuts_the_loop_as_one_undo_step() {
    let mut harness = ready();
    let doc = harness.state().workbench.document();
    let (steps, selection) = (doc.undo_steps(), doc.elements(cube(&harness)).cloned());
    let at = edge_middle(&harness);
    click(&mut harness, at);
    assert_eq!(shape(&harness).face_count(), 10);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Loop cut"));
    // The click didn't select anything.
    assert_eq!(doc.elements(cube(&harness)).cloned(), selection);
    let (cuts, position) = loop_cut(&harness);
    assert_eq!(cuts, 1);
    assert!((position - 0.5).abs() < 0.1, "{position}");
    assert!(note(&harness).starts_with("Loop cut across 4 polygons"));
    // The loop isn't shown any more over the cut: it is there.
    harness.state_mut().workbench.document_mut().undo().unwrap();
    harness.run();
    assert_eq!(shape(&harness).face_count(), 6);
}

#[test]
fn dragging_along_the_edge_slides_the_cut() {
    let mut harness = ready();
    let steps = harness.state().workbench.document().undo_steps();
    let from = edge_middle(&harness);
    let towards = on_screen(&harness, glam::Vec3::new(50.0, 40.0, 100.0)) - from;
    hover(&mut harness, from);
    button(
        &mut harness,
        from,
        true,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let mut at = from;
    for _ in 0..5 {
        at += towards / 5.0;
        harness.event(Event::PointerMoved(at));
        harness.run();
    }
    // Shown while dragging, one step in all.
    assert_eq!(shape(&harness).face_count(), 10);
    let (_, sliding) = loop_cut(&harness);
    button(
        &mut harness,
        at,
        false,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let (_, position) = loop_cut(&harness);
    assert_eq!(position, sliding);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    // 40 of the edge's 100 towards +Y from the middle: 90% or 10% across,
    // depending on which end has the lower id.
    assert!(
        (position - 0.9).abs() < 0.05 || (position - 0.1).abs() < 0.05,
        "{position}"
    );
}

#[test]
fn esc_cancels_a_drag() {
    let mut harness = ready();
    let steps = harness.state().workbench.document().undo_steps();
    let from = edge_middle(&harness);
    hover(&mut harness, from);
    button(
        &mut harness,
        from,
        true,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    for k in 1..=4 {
        harness.event(Event::PointerMoved(from + egui::vec2(0.0, 5.0 * k as f32)));
        harness.run();
    }
    assert_eq!(shape(&harness).face_count(), 10);
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert_eq!(shape(&harness).face_count(), 6);
    let at = from + egui::vec2(0.0, 20.0);
    button(
        &mut harness,
        at,
        false,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert_eq!(harness.state().workbench.tool(), Tool::LoopCut);
}

#[test]
fn cuts_is_set_before_and_cuts_and_position_after() {
    let mut harness = ready();
    // The tool's setting: Cuts.
    assert!(harness.query_all_by_label("Cuts").count() >= 1);
    harness.state_mut().workbench.attributes.loop_cut.cuts = 3;
    harness.run();
    let at = edge_middle(&harness);
    click(&mut harness, at);
    assert_eq!(loop_cut(&harness), (3, 0.5));
    assert_eq!(shape(&harness).face_count(), 6 + 12);
    // Afterwards, as the Last edit.
    assert!(
        harness
            .query_by_label_contains("Last edit: Loop cut (3 cuts)")
            .is_some()
    );
    assert!(harness.query_all_by_label("Position").count() >= 1);
    assert!(harness.query_by_label("Distance").is_none());
}

#[test]
fn clicking_away_from_the_shape_cuts_nothing() {
    let mut harness = ready();
    let steps = harness.state().workbench.document().undo_steps();
    let away = harness
        .get_by_label(crate::VIEWPORT_LABEL)
        .rect()
        .left_top()
        + egui::vec2(60.0, 80.0);
    click(&mut harness, away);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert!(note(&harness).starts_with("Loop cut: point at an edge of a selected object"));
    // And the object stays selected.
    assert_eq!(harness.state().workbench.document().selection().len(), 1);
}
