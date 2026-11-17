//! Tests for Sculpt mode (decision 0304): its layout, strokes with Draw,
//! Ctrl and Shift, undo, Esc, its keys, the question about booleans and
//! modifiers, and the matcap.

use super::brushing_tests::stroke;
use super::heavy_check::{H, on_screen};
use super::tests::{State, harness_for, info};
use super::*;
use egui::{Event, Modifiers, PointerButton};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use scene::{ObjectId, ObjectKind, PrimitiveKind};

/// A window with a sphere, selected, in Sculpt mode.
fn sculpting() -> H {
    let mut workbench = Workbench::new(info(), None);
    workbench
        .document_mut()
        .execute(scene::Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Sphere.standard(),
        )))
        .unwrap();
    let mut harness = harness_for(workbench);
    harness.run();
    harness.state_mut().workbench.mode = Mode::Sculpt;
    harness.run();
    harness
}

fn sphere(harness: &H) -> ObjectId {
    harness.state().workbench.document().selection()[0]
}

fn shape(harness: &H) -> scene::Mesh {
    let doc = harness.state().workbench.document();
    (*doc.scene().evaluated(sphere(harness)).unwrap().mesh).clone()
}

/// The sphere's front middle, in the world, and on screen.
fn front(harness: &H) -> (Vec3, egui::Pos2) {
    let b = shape(harness).bounds().unwrap();
    let world = Vec3::new((b.min.x + b.max.x) / 2.0, b.min.y, (b.min.z + b.max.z) / 2.0);
    (world, on_screen(harness, world))
}

/// How far the sphere's front reaches out (towards -Y).
fn front_reach(harness: &H) -> f32 {
    -shape(harness).bounds().unwrap().min.y
}

/// A short stroke across the sphere's front, holding `modifiers`.
fn sculpt_stroke(harness: &mut H, modifiers: Modifiers) {
    let (_, at) = front(harness);
    stroke(
        harness,
        at - egui::vec2(10.0, 0.0),
        at + egui::vec2(10.0, 0.0),
        modifiers,
        PointerButton::Primary,
    );
}

fn layer_names(harness: &H) -> Vec<String> {
    let doc = harness.state().workbench.document();
    doc.scene()
        .object(sphere(harness))
        .unwrap()
        .stack
        .layers()
        .iter()
        .map(|l| l.name.clone())
        .collect()
}

#[test]
fn sculpt_mode_has_its_layout_and_leaving_puts_the_standard_one_back() {
    let mut harness = sculpting();
    assert_eq!(harness.state().workbench.layout_choice(), LayoutChoice::Sculpt);
    assert!(harness.query_by_label("Sculpt brush bar").is_some());
    // In the brush bar and in the Sculpt tab.
    assert_eq!(harness.query_all_by_label("Sculpt brush size").count(), 2);
    harness.key_press(egui::Key::Escape);
    harness.run();
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.mode, Mode::Model);
    assert_eq!(workbench.layout_choice(), LayoutChoice::Standard);
    assert!(harness.query_by_label("Sculpt brush bar").is_none());
}

#[test]
fn the_sculpt_layout_button_enters_sculpt_mode() {
    let mut harness = harness_for(Workbench::new(info(), None));
    harness.run();
    harness.state_mut().workbench.switch_layout(LayoutChoice::Sculpt);
    harness.run();
    assert_eq!(harness.state().workbench.mode, Mode::Sculpt);
    harness.state_mut().workbench.switch_layout(LayoutChoice::Standard);
    harness.run();
    assert_eq!(harness.state().workbench.mode, Mode::Model);
}

#[test]
fn a_stroke_builds_up_in_one_undo_step_on_a_sculpt_layer() {
    let mut harness = sculpting();
    let before = shape(&harness);
    let reach = front_reach(&harness);
    sculpt_stroke(&mut harness, Modifiers::NONE);
    assert_eq!(layer_names(&harness), vec!["Sculpt".to_string()]);
    assert!(front_reach(&harness) > reach + 0.1, "built up");
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.document().undo_label(), Some("Sculpt"));
    // What is drawn is what the document holds.
    let drawn = workbench.drawing.summary().drawing.items[0].drawn.clone();
    assert_eq!(*drawn, shape(&harness));
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(shape(&harness), before);
    let drawn = harness.state().workbench.drawing.summary().drawing.items[0]
        .drawn
        .clone();
    assert_eq!(*drawn, before, "the live sculpt followed the undo");
    // Redo, then another stroke goes into the same layer.
    harness.state_mut().workbench.document_mut().redo();
    harness.run();
    sculpt_stroke(&mut harness, Modifiers::NONE);
    assert_eq!(layer_names(&harness).len(), 1);
    assert_eq!(harness.state().workbench.document().undo_steps(), 3);
}

#[test]
fn ctrl_digs_in_and_shift_smooths() {
    let mut harness = sculpting();
    let reach = front_reach(&harness);
    sculpt_stroke(&mut harness, Modifiers::COMMAND);
    assert!(front_reach(&harness) < reach - 0.1, "dug in");
    let dug = shape(&harness);
    sculpt_stroke(&mut harness, Modifiers::SHIFT);
    assert_ne!(shape(&harness), dug, "smoothed");
    assert!(!harness.state().workbench.sculpting.stroking());
}

#[test]
fn esc_takes_back_a_stroke_under_way_and_keeps_the_mode() {
    let mut harness = sculpting();
    let before = shape(&harness);
    let (_, at) = front(&harness);
    harness.event(Event::PointerMoved(at));
    harness.step();
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for i in 1..4 {
        harness.event(Event::PointerMoved(at + egui::vec2(i as f32 * 8.0, 0.0)));
        harness.step();
    }
    assert!(harness.state().workbench.sculpting.stroking());
    harness.key_press(egui::Key::Escape);
    harness.step();
    assert_eq!(harness.state().workbench.mode, Mode::Sculpt);
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert_eq!(shape(&harness), before);
    assert_eq!(harness.state().workbench.document().undo_steps(), 1);
    let drawn = harness.state().workbench.drawing.summary().drawing.items[0]
        .drawn
        .clone();
    assert_eq!(*drawn, before);
}

#[test]
fn number_keys_choose_brushes_instead_of_modes_and_brackets_size_the_brush() {
    let mut harness = sculpting();
    harness.state_mut().workbench.attributes.sculpt.brush = sculpt::BrushKind::Smooth;
    harness.key_press(egui::Key::Num1);
    harness.run();
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.mode, Mode::Sculpt);
    assert_eq!(workbench.attributes.sculpt.brush, sculpt::BrushKind::Draw);
    harness.key_press(egui::Key::Num2);
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert_eq!(harness.state().workbench.mode, Mode::Sculpt);
    let size = harness.state().workbench.attributes.sculpt.draw.radius;
    harness.key_press(egui::Key::CloseBracket);
    harness.run();
    assert!(harness.state().workbench.attributes.sculpt.draw.radius > size);
    harness.key_press(egui::Key::OpenBracket);
    harness.key_press(egui::Key::OpenBracket);
    harness.run();
    assert!(harness.state().workbench.attributes.sculpt.draw.radius < size);
}

#[test]
fn the_views_show_a_matcap_with_cavity_and_no_edges() {
    let mut harness = sculpting();
    harness.state_mut().renderer.drawn.clear();
    harness.state_mut().workbench.viewport.options_mut().show_edges = true;
    harness.run();
    let view = harness.state().renderer.drawn.last().unwrap().clone();
    assert_eq!(view.matcap, Some(0));
    assert!(view.cavity);
    assert!(!view.show_edges);
    harness.state_mut().workbench.attributes.sculpt.look = crate::sculpt_tab::SculptLook::Lit;
    harness.run();
    let view = harness.state().renderer.drawn.last().unwrap().clone();
    assert_eq!(view.matcap, None);
    // Out of Sculpt mode, no matcap and the edges as they were.
    harness.state_mut().workbench.mode = Mode::Model;
    harness.run();
    let view = harness.state().renderer.drawn.last().unwrap().clone();
    assert_eq!(view.matcap, None);
    assert!(view.show_edges);
}

#[test]
fn a_shape_with_a_modifier_asks_first_and_can_be_sculpted_under_it() {
    let mut harness = sculpting();
    let id = sphere(&harness);
    harness.state_mut().workbench.mode = Mode::Model;
    harness.run();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::AddModifier {
            ids: vec![id],
            kind: scene::ModifierKind::Subdivide(scene::SubdivideOptions::default()),
        })
        .unwrap();
    harness.state_mut().workbench.mode = Mode::Sculpt;
    harness.run();
    assert!(harness.query_by_label("Bake them in").is_some());
    assert!(harness.state().workbench.sculpting.sculpted().is_none());
    harness.get_by_label("Sculpt under them").click();
    harness.run();
    assert_eq!(harness.state().workbench.sculpting.sculpted(), Some(id));
    // Drawn without the modifier while sculpting.
    let drawn = harness.state().workbench.drawing.summary().drawing.items[0]
        .drawn
        .clone();
    assert_eq!(drawn.face_count(), shape(&harness).face_count());
    sculpt_stroke(&mut harness, Modifiers::NONE);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().modifiers(id).len(), 1, "the modifier stays");
}

#[test]
fn baking_in_takes_the_modifiers_into_the_shape_as_one_step() {
    let mut harness = sculpting();
    let id = sphere(&harness);
    harness.state_mut().workbench.mode = Mode::Model;
    harness.run();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::AddModifier {
            ids: vec![id],
            kind: scene::ModifierKind::Subdivide(scene::SubdivideOptions::default()),
        })
        .unwrap();
    let steps = harness.state().workbench.document().undo_steps();
    harness.state_mut().workbench.mode = Mode::Sculpt;
    harness.run();
    harness.get_by_label("Bake them in").click();
    harness.run();
    let workbench = &harness.state().workbench;
    assert!(workbench.document().scene().modifiers(id).is_empty());
    assert_eq!(workbench.document().undo_steps(), steps + 1);
    assert_eq!(workbench.sculpting.sculpted(), Some(id));
}

#[test]
fn a_heavy_shape_is_made_ready_on_another_thread() {
    let mut workbench = Workbench::new(info(), None);
    workbench.add_test_shape(super::sculpting::READY_IN_BACKGROUND_FROM);
    let mut harness = harness_for(workbench);
    harness.run();
    harness.state_mut().workbench.mode = Mode::Sculpt;
    harness.step();
    let state: &State = harness.state();
    assert!(state.workbench.sculpting.getting_ready() || state.workbench.sculpting.sculpted().is_some());
    for _ in 0..200 {
        if harness.state().workbench.sculpting.sculpted().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        harness.step();
    }
    assert!(harness.state().workbench.sculpting.sculpted().is_some());
}

#[test]
fn nothing_selected_says_what_to_do() {
    let mut harness = sculpting();
    harness.state_mut().workbench.document_mut().clear_selection();
    harness.run();
    let workbench = &harness.state().workbench;
    assert!(workbench.sculpting.sculpted().is_none());
    assert!(workbench.attributes.sculpt.status.contains("Select a shape"));
}
