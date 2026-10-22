//! Tests for Slit in the window: the key, clicking and dragging the line's
//! points, the cut as one undo step, editing the line, finishing it, and
//! the Cut through switch.

use super::modelling_tests::{H, button, cube, note, on_screen, press, shape};
use super::tests::fresh;
use super::*;
use crate::transforming::Tool;
use egui::{Event, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;

/// Where line point `i` is on screen.
fn point_on_screen(harness: &H, i: usize) -> Pos2 {
    let p = harness
        .state()
        .workbench
        .viewport()
        .tool_path()
        .unwrap()
        .points[i];
    on_screen(harness, p)
}
use scene::{Element, ElementSelection, Operation};

/// The fresh window's cube selected, in the Edges mode, with an edge
/// selected and Slit in use.
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
    let edge = shape(&harness).edges()[0];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Edge(edge)]))],
            "Select",
            None,
        );
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::X);
    harness
}

/// Left of the cube's front and right of it, at half height.
fn ends(harness: &H) -> (Pos2, Pos2) {
    (
        on_screen(harness, glam::Vec3::new(-75.0, -50.0, 50.0)),
        on_screen(harness, glam::Vec3::new(40.0, -50.0, 50.0)),
    )
}

fn click(harness: &mut H, at: Pos2, keys: Modifiers) {
    harness.event(Event::PointerMoved(at));
    harness.run();
    harness.event(Event::ModifiersChanged(keys));
    button(harness, at, true, PointerButton::Primary, keys);
    button(harness, at, false, PointerButton::Primary, keys);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

fn drag(harness: &mut H, from: Pos2, to: Pos2) {
    harness.event(Event::PointerMoved(from));
    harness.run();
    button(harness, from, true, PointerButton::Primary, Modifiers::NONE);
    for k in 1..=5 {
        harness.event(Event::PointerMoved(from + (to - from) * (k as f32 / 5.0)));
        harness.run();
    }
    button(harness, to, false, PointerButton::Primary, Modifiers::NONE);
}

fn line_points(harness: &H) -> usize {
    harness
        .state()
        .workbench
        .viewport()
        .tool_path()
        .map_or(0, |p| p.points.len())
}

fn slits(harness: &H) -> usize {
    let doc = harness.state().workbench.document();
    doc.scene().object(cube(harness)).unwrap().stack.layers()[0]
        .entries
        .iter()
        .filter(|e| matches!(e.operation, Operation::Slit { .. }))
        .count()
}

#[test]
fn x_picks_slit_while_editing() {
    let mut harness = ready();
    assert_eq!(harness.state().workbench.tool(), Tool::Slit);
    assert!(
        note(&harness).starts_with("Slit: click points"),
        "{}",
        note(&harness)
    );
    press(&mut harness, Modifiers::NONE, Key::Q);
    press(&mut harness, Modifiers::NONE, Key::Num2);
    press(&mut harness, Modifiers::NONE, Key::X);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    assert!(
        note(&harness).starts_with("Slit works in"),
        "{}",
        note(&harness)
    );
}

#[test]
fn clicking_points_cuts_as_one_undo_step() {
    let mut harness = ready();
    let steps = harness.state().workbench.document().undo_steps();
    let (left, right) = ends(&harness);
    click(&mut harness, left, Modifiers::NONE);
    assert_eq!(line_points(&harness), 1);
    assert_eq!(shape(&harness).face_count(), 6, "one point cuts nothing");
    click(&mut harness, right, Modifiers::NONE);
    assert_eq!(line_points(&harness), 2);
    assert!(shape(&harness).face_count() > 6);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Slit"));
    // A third point: still one step, one slit.
    let below = on_screen(&harness, glam::Vec3::new(60.0, -50.0, 10.0));
    click(&mut harness, below, Modifiers::NONE);
    assert_eq!(line_points(&harness), 3);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    assert_eq!(slits(&harness), 1);
    let report = mesh::MeshReport::new(&shape(&harness));
    assert!(report.is_closed() && !report.has_problems(), "{report:?}");
}

#[test]
fn points_can_be_dragged_added_and_removed() {
    let mut harness = ready();
    let (left, right) = ends(&harness);
    click(&mut harness, left, Modifiers::NONE);
    click(&mut harness, right, Modifiers::NONE);
    let steps = harness.state().workbench.document().undo_steps();
    let before = shape(&harness);
    // Drag the right point up: the cut moves.
    let higher = on_screen(&harness, glam::Vec3::new(40.0, -50.0, 80.0));
    drag(&mut harness, right, higher);
    assert_eq!(line_points(&harness), 2);
    assert_ne!(*shape(&harness), *before);
    // Click the line's middle: a third point.
    let (a, b) = (point_on_screen(&harness, 0), point_on_screen(&harness, 1));
    let middle = a + (b - a) * 0.5;
    click(&mut harness, middle, Modifiers::NONE);
    assert_eq!(line_points(&harness), 3);
    // Ctrl + click it: gone again.
    click(&mut harness, middle, Modifiers::COMMAND);
    assert_eq!(line_points(&harness), 2);
    // Ctrl + click the end (where it snapped to): one point left, nothing
    // cut.
    let end = point_on_screen(&harness, 1);
    click(&mut harness, end, Modifiers::COMMAND);
    assert_eq!(line_points(&harness), 1);
    assert_eq!(shape(&harness).face_count(), 6);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps - 1);
}

#[test]
fn esc_cancels_the_line_and_takes_its_cut_back() {
    // Esc cancels everywhere, a slit too (decision 0126).
    let mut harness = ready();
    let id = cube(&harness);
    let selected = harness.state().workbench.document().elements(id).cloned();
    let steps = harness.state().workbench.document().undo_steps();
    let faces = shape(&harness).face_count();
    let (left, right) = ends(&harness);
    click(&mut harness, left, Modifiers::NONE);
    click(&mut harness, right, Modifiers::NONE);
    assert!(shape(&harness).face_count() > faces, "cut while drawn");
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert_eq!(line_points(&harness), 0);
    assert!(note(&harness).starts_with("Slit cancelled"));
    assert_eq!(shape(&harness).face_count(), faces, "the cut is gone");
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert_eq!(
        harness.state().workbench.document().elements(id).cloned(),
        selected,
        "Esc on the line doesn't deselect"
    );
}

#[test]
fn enter_finishes_the_line_and_keeps_the_selection() {
    let mut harness = ready();
    let id = cube(&harness);
    let selected = harness.state().workbench.document().elements(id).cloned();
    let (left, right) = ends(&harness);
    click(&mut harness, left, Modifiers::NONE);
    click(&mut harness, right, Modifiers::NONE);
    let faces = shape(&harness).face_count();
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(line_points(&harness), 0);
    assert!(note(&harness).starts_with("Slit finished"));
    assert_eq!(shape(&harness).face_count(), faces, "the cut stays");
    assert_eq!(
        harness.state().workbench.document().elements(id).cloned(),
        selected
    );
    // The next click starts a new line, and a new step.
    let steps = harness.state().workbench.document().undo_steps();
    let low = ends(&harness);
    let (a, b) = (low.0 + egui::vec2(0.0, 25.0), low.1 + egui::vec2(0.0, 25.0));
    click(&mut harness, a, Modifiers::NONE);
    click(&mut harness, b, Modifiers::NONE);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    assert_eq!(slits(&harness), 2);
    // Ctrl + Z takes the line back and finishes it.
    harness.event(Event::Key {
        key: Key::Z,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    });
    harness.run();
    harness.run();
    assert_eq!(slits(&harness), 1);
    assert_eq!(line_points(&harness), 0);
}

#[test]
fn another_tool_finishes_the_line() {
    let mut harness = ready();
    let (left, right) = ends(&harness);
    click(&mut harness, left, Modifiers::NONE);
    click(&mut harness, right, Modifiers::NONE);
    press(&mut harness, Modifiers::NONE, Key::Q);
    assert!(harness.state().workbench.viewport().tool_path().is_none());
    assert_eq!(slits(&harness), 1);
}

#[test]
fn cut_through_cuts_the_far_side_too() {
    let mut seen = ready();
    let (left, right) = ends(&seen);
    click(&mut seen, left, Modifiers::NONE);
    click(&mut seen, right, Modifiers::NONE);
    let mut through = ready();
    // The tool's switch, before cutting.
    assert!(through.query_all_by_label("Cut through").count() >= 1);
    through.state_mut().workbench.attributes.slit_through = true;
    through.run();
    click(&mut through, left, Modifiers::NONE);
    click(&mut through, right, Modifiers::NONE);
    assert!(
        shape(&through).face_count() > shape(&seen).face_count(),
        "{} > {}",
        shape(&through).face_count(),
        shape(&seen).face_count()
    );
    assert_eq!(
        through.state().workbench.document().undo_label(),
        Some("Slit through")
    );
}

#[test]
fn shift_keeps_the_stretch_at_steps_of_15_degrees() {
    let mut harness = ready();
    let (left, _) = ends(&harness);
    click(&mut harness, left, Modifiers::NONE);
    // 40 right and 13 down: about 18°, kept at 15°.
    let at = left + egui::vec2(40.0, 13.0);
    click(&mut harness, at, Modifiers::SHIFT);
    let path = harness
        .state()
        .workbench
        .viewport()
        .tool_path()
        .unwrap()
        .clone();
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let camera = super::tests::camera(&harness);
    let a = camera.project(path.points[0], size).unwrap();
    let b = camera.project(path.points[1], size).unwrap();
    let angle = (b - a).y.atan2((b - a).x).to_degrees();
    assert!((angle - 15.0).abs() < 0.5, "{angle}");
}

/// The last recorded slit's line.
fn last_line(harness: &H) -> scene::SlitLine {
    match super::modelling_tests::last_operation(harness) {
        Some(Operation::Slit { line }) => line,
        other => panic!("not a slit: {other:?}"),
    }
}

#[test]
fn clicking_the_first_point_closes_the_line_into_a_loop() {
    let mut harness = ready();
    // Four points well inside the cube's front polygon.
    let corners = [
        glam::Vec3::new(-25.0, -50.0, 25.0),
        glam::Vec3::new(25.0, -50.0, 25.0),
        glam::Vec3::new(25.0, -50.0, 75.0),
        glam::Vec3::new(-25.0, -50.0, 75.0),
    ];
    for c in corners {
        let at = on_screen(&harness, c);
        click(&mut harness, at, Modifiers::NONE);
    }
    assert!(!last_line(&harness).is_closed());
    let faces = shape(&harness).face_count();
    assert_eq!(faces, 6, "an open line inside one polygon cuts nothing");
    let steps = harness.state().workbench.document().undo_steps();
    // Back to the first point: hovering it shows the closing stretch.
    let first = point_on_screen(&harness, 0);
    harness.event(Event::PointerMoved(first));
    harness.run();
    let path = harness.state().workbench.viewport().tool_path().unwrap();
    assert_eq!(path.cursor, Some(path.points[0]));
    click(&mut harness, first, Modifiers::NONE);
    // Closed and finished: the square is cut into the front, in the same
    // undo step as the line.
    assert!(last_line(&harness).is_closed());
    assert_eq!(shape(&harness).face_count(), 6 + 2);
    // Finished: the next click starts a new line.
    assert!(
        harness
            .state()
            .workbench
            .viewport()
            .tool_path()
            .is_none_or(|p| p.points.is_empty())
    );
    assert!(note(&harness).starts_with("Slit closed"));
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    let report = mesh::MeshReport::new(&shape(&harness));
    assert!(!report.has_problems(), "{report:?}");
    // Ctrl + Z takes the whole slit back.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(shape(&harness).face_count(), 6);
}

#[test]
fn clicking_a_point_leaves_it_where_it_is_and_two_points_never_close() {
    let mut harness = ready();
    let (a, b) = ends(&harness);
    click(&mut harness, a, Modifiers::NONE);
    click(&mut harness, b, Modifiers::NONE);
    let before = last_line(&harness);
    // A click a little off the first point: a click, not a drag, so the
    // point stays and (with only two points) nothing closes.
    let first = point_on_screen(&harness, 0);
    click(&mut harness, first + egui::vec2(2.0, 1.0), Modifiers::NONE);
    let after = last_line(&harness);
    assert_eq!(after, before);
    assert!(harness.state().workbench.viewport().tool_path().is_some());
    // Dragging it still moves it.
    drag(&mut harness, first, first + egui::vec2(0.0, -30.0));
    assert_ne!(last_line(&harness).points[0], before.points[0]);
}
