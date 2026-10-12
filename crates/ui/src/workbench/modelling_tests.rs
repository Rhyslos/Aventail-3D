//! Tests for Extrude and Inset in the window: the keys, dragging anywhere
//! in the view as one undo step, fine steps, cancelling, the Attributes
//! panel's Keep together switch and last edit, and extruding border edges.

use super::tests::{State, fresh};
use super::*;
use crate::transforming::Tool;
use egui::{Event, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{Element, ElementSelection, FaceId, Grouping, ObjectId, Operation};

type H = Harness<'static, State>;

fn press(harness: &mut H, modifiers: Modifiers, key: Key) {
    harness.event(Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    });
    harness.event(Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers,
    });
    harness.run();
}

fn cube(harness: &H) -> ObjectId {
    harness.state().workbench.document().scene().children(None)[0]
}

fn shape(harness: &H) -> std::sync::Arc<scene::Mesh> {
    let doc = harness.state().workbench.document();
    doc.scene().evaluated(cube(harness)).unwrap().mesh
}

fn top_face(mesh: &scene::Mesh) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .max_by(|a, b| mesh.face_normal(*a).z.total_cmp(&mesh.face_normal(*b).z))
        .unwrap()
}

/// The fresh window's cube selected, in the Polygons mode, its top
/// polygon selected, and `key`'s tool in use.
fn ready(key: Key) -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let top = top_face(&shape(&harness));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Polygon(top)]))],
            "Select",
            None,
        );
    harness.run();
    press(&mut harness, Modifiers::NONE, key);
    harness
}

fn button(harness: &mut H, at: Pos2, pressed: bool, button: PointerButton, modifiers: Modifiers) {
    harness.event(Event::PointerButton {
        pos: at,
        button,
        pressed,
        modifiers,
    });
    harness.run();
}

/// A spot in the view away from the cube.
fn empty_spot(harness: &H) -> Pos2 {
    harness
        .get_by_label(crate::VIEWPORT_LABEL)
        .rect()
        .left_top()
        + egui::vec2(60.0, 80.0)
}

/// Presses at `from` and moves `dx` points sideways in steps, without
/// letting go.
fn drag_without_letting_go(harness: &mut H, from: Pos2, dx: f32) -> Pos2 {
    harness.event(Event::PointerMoved(from));
    harness.run();
    button(harness, from, true, PointerButton::Primary, Modifiers::NONE);
    let mut at = from;
    for _ in 0..5 {
        at += egui::vec2(dx / 5.0, 0.0);
        harness.event(Event::PointerMoved(at));
        harness.run();
    }
    at
}

fn drag(harness: &mut H, dx: f32) {
    let from = empty_spot(harness);
    let at = drag_without_letting_go(harness, from, dx);
    button(harness, at, false, PointerButton::Primary, Modifiers::NONE);
}

fn top_height(harness: &H) -> f32 {
    shape(harness).bounds().unwrap().max.z
}

/// How high the (first selected) top polygon is.
fn top_polygon_height(harness: &H) -> f32 {
    let mesh = shape(harness);
    let doc = harness.state().workbench.document();
    let face = doc
        .elements(cube(harness))
        .unwrap()
        .polygons()
        .next()
        .unwrap();
    let corners = mesh.face(face).unwrap();
    corners
        .iter()
        .map(|v| mesh.position(*v).unwrap().z)
        .sum::<f32>()
        / corners.len() as f32
}

/// Clicks the `n`th Keep together switch; returns how many there are.
fn click_keep_together(harness: &mut H, n: usize) -> usize {
    let switches: Vec<egui::Rect> = harness
        .get_all_by_label("Keep together")
        .filter(|node| node.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .map(|node| node.rect())
        .collect();
    let count = switches.len();
    let at = switches[n].center();
    harness.event(Event::PointerMoved(at));
    harness.run();
    button(harness, at, true, PointerButton::Primary, Modifiers::NONE);
    button(harness, at, false, PointerButton::Primary, Modifiers::NONE);
    count
}

fn note(harness: &H) -> String {
    harness.state().workbench.note.clone().unwrap_or_default()
}

fn last_operation(harness: &H) -> Option<Operation> {
    let doc = harness.state().workbench.document();
    let object = doc.scene().object(cube(harness))?;
    object.stack.layers().last()?.operations.last().cloned()
}

#[test]
fn d_and_i_pick_extrude_and_inset() {
    let mut harness = ready(Key::D);
    assert_eq!(harness.state().workbench.tool(), Tool::Extrude);
    assert!(note(&harness).starts_with("Extrude: drag left or right"));
    press(&mut harness, Modifiers::NONE, Key::I);
    assert_eq!(harness.state().workbench.tool(), Tool::Inset);
    // Ctrl + I still inverts the selection: it doesn't pick Inset.
    press(&mut harness, Modifiers::NONE, Key::Q);
    press(&mut harness, Modifiers::COMMAND, Key::I);
    assert_ne!(harness.state().workbench.tool(), Tool::Inset);
}

#[test]
fn extrude_and_inset_need_the_right_mode() {
    let mut harness = fresh();
    press(&mut harness, Modifiers::NONE, Key::D);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    assert!(note(&harness).contains("press 3 for the Polygons mode"));
    press(&mut harness, Modifiers::NONE, Key::I);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    assert!(note(&harness).starts_with("Inset works on polygons"));
}

#[test]
fn a_drag_anywhere_extrudes_as_one_undo_step() {
    let mut harness = ready(Key::D);
    let steps = harness.state().workbench.document().undo_steps();
    drag(&mut harness, 120.0);
    assert_eq!(shape(&harness).face_count(), 10);
    assert!(top_height(&harness) > 101.0, "pushed up");
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1, "one step for the whole drag");
    assert_eq!(doc.undo_label(), Some("Extrude 1 polygon"));
    assert!(note(&harness).starts_with("Extrude 1 polygon: "));
    // Dragging left pushes in.
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    drag(&mut harness, -120.0);
    assert!(top_polygon_height(&harness) < 99.9, "pushed in");
    // The polygon is still selected, so another drag carries on.
    drag(&mut harness, -40.0);
    assert_eq!(shape(&harness).face_count(), 14);
}

#[test]
fn shift_makes_the_drag_ten_times_finer() {
    let mut harness = ready(Key::D);
    drag(&mut harness, 100.0);
    let coarse = top_height(&harness) - 100.0;
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    harness.event(Event::ModifiersChanged(Modifiers::SHIFT));
    harness.run();
    let from = empty_spot(&harness);
    let at = drag_without_letting_go(&mut harness, from, 100.0);
    button(
        &mut harness,
        at,
        false,
        PointerButton::Primary,
        Modifiers::SHIFT,
    );
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
    let fine = top_height(&harness) - 100.0;
    assert!(fine > 0.0 && fine < coarse * 0.2, "{fine} vs {coarse}");
}

#[test]
fn esc_or_a_right_click_cancels_and_letting_go_at_nothing_leaves_nothing() {
    for cancel in ["esc", "right"] {
        let mut harness = ready(Key::D);
        let steps = harness.state().workbench.document().undo_steps();
        let from = empty_spot(&harness);
        let at = drag_without_letting_go(&mut harness, from, 150.0);
        assert_eq!(shape(&harness).face_count(), 10, "shown while dragging");
        if cancel == "esc" {
            press(&mut harness, Modifiers::NONE, Key::Escape);
        } else {
            button(
                &mut harness,
                at,
                true,
                PointerButton::Secondary,
                Modifiers::NONE,
            );
            button(
                &mut harness,
                at,
                false,
                PointerButton::Secondary,
                Modifiers::NONE,
            );
        }
        button(
            &mut harness,
            at,
            false,
            PointerButton::Primary,
            Modifiers::NONE,
        );
        assert_eq!(shape(&harness).face_count(), 6, "{cancel}");
        assert_eq!(harness.state().workbench.document().undo_steps(), steps);
        assert!(note(&harness).starts_with("Cancelled"));
        // The selection and tool stay.
        assert_eq!(harness.state().workbench.tool(), Tool::Extrude);
    }
    // There and back again: nothing.
    let mut harness = ready(Key::D);
    let steps = harness.state().workbench.document().undo_steps();
    let from = empty_spot(&harness);
    let at = drag_without_letting_go(&mut harness, from, 100.0);
    harness.event(Event::PointerMoved(from));
    harness.run();
    button(
        &mut harness,
        from,
        false,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let _ = at;
    assert_eq!(shape(&harness).face_count(), 6);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert!(note(&harness).contains("nothing changed"));
}

#[test]
fn inset_shrinks_and_never_grows() {
    let mut harness = ready(Key::I);
    drag(&mut harness, 60.0);
    let mesh = shape(&harness);
    assert_eq!(mesh.face_count(), 10);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Inset 1 polygon")
    );
    // The cube's outline is unchanged: the inset stays on the top.
    assert!((top_height(&harness) - 100.0).abs() < 1e-3);
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    let steps = harness.state().workbench.document().undo_steps();
    drag(&mut harness, -60.0);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
}

#[test]
fn the_last_extrude_can_be_changed_in_the_attributes_panel() {
    let mut harness = ready(Key::D);
    drag(&mut harness, 100.0);
    assert!(
        harness
            .query_by_label_contains("Last edit: Extrude 1 polygon")
            .is_some()
    );
    assert!(harness.query_all_by_label("Distance").count() >= 1);
    // Switching Keep together off on the last edit changes the edit, as
    // an undo step. It is the only switch: the tool's own is hidden while
    // the last edit shows.
    let steps = harness.state().workbench.document().undo_steps();
    let count = click_keep_together(&mut harness, 0);
    assert_eq!(count, 1, "one switch");
    assert_eq!(
        last_operation(&harness).and_then(|op| op.grouping()),
        Some(Grouping::Separate)
    );
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Change Extrude 1 polygon"));
    // ...and the next drag uses it too.
    assert_eq!(harness.state().workbench.grouping(), Grouping::Separate);
}

#[test]
fn the_tools_keep_together_switch_sets_the_next_drag() {
    let mut harness = ready(Key::D);
    assert_eq!(click_keep_together(&mut harness, 0), 1);
    assert_eq!(harness.state().workbench.grouping(), Grouping::Separate);
    drag(&mut harness, 80.0);
    assert_eq!(
        last_operation(&harness).and_then(|op| op.grouping()),
        Some(Grouping::Separate)
    );
}

#[test]
fn the_mesh_menu_and_tool_menu_pick_extrude() {
    let mut harness = ready(Key::Q);
    super::tests::main_menu(&harness, "Mesh").click();
    harness.run();
    harness.get_by_label_contains("Extrude").click();
    harness.run();
    assert_eq!(harness.state().workbench.tool(), Tool::Extrude);
}

#[test]
fn border_edges_extrude_and_the_selection_follows() {
    let mut harness = fresh();
    // A plane instead of the cube.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::Primitive::Plane(Default::default()),
        )))
        .unwrap();
    harness.run();
    let plane = *harness
        .state()
        .workbench
        .document()
        .selection()
        .first()
        .unwrap();
    press(&mut harness, Modifiers::NONE, Key::Num2);
    let mesh = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(plane)
        .unwrap()
        .mesh;
    let edge = mesh.edges()[0];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(plane, ElementSelection::of([Element::Edge(edge)]))],
            "Select",
            None,
        );
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::D);
    drag(&mut harness, 80.0);
    let faces = |h: &H| {
        h.state()
            .workbench
            .document()
            .scene()
            .evaluated(plane)
            .unwrap()
            .mesh
            .face_count()
    };
    assert_eq!(faces(&harness), 2);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Extrude 1 edge")
    );
    // The new outer edge is selected now, so the next drag carries on.
    let selected = harness
        .state()
        .workbench
        .document()
        .elements(plane)
        .unwrap()
        .clone();
    assert_eq!(selected.count(scene::ElementKind::Edges), 1);
    assert!(!selected.contains(Element::Edge(edge)));
    drag(&mut harness, 80.0);
    assert_eq!(faces(&harness), 3);
    // One undo takes back the second strip and its selection together.
    harness.state_mut().workbench.document_mut().undo();
    assert_eq!(faces(&harness), 2);
}
