//! Tests for editing UVs on the model, and the viewport and UV Editor
//! lighting up what is under the mouse in the other (decision 0103).

use super::modelling_tests::{H, button, cube, drag_by, on_screen, press, shape};
use super::tests::quick_harness;
use super::*;
use egui::{Event, PointerButton};
use glam::{Vec2, Vec3};
use scene::{Element, ElementKind, FaceId};

/// The cube selected, in the UV Edit layout (the UV mode).
fn in_uv_mode() -> H {
    let mut harness = quick_harness();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    harness.run();
    harness
}

/// The cube's polygon facing `normal`, and the middle of its UVs.
fn side(harness: &H, normal: Vec3) -> (FaceId, Vec2) {
    let mesh = shape(harness);
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let (face, corners) = mesh
        .faces()
        .find(|(f, _)| mesh.face_normal(*f).dot(normal) > 0.99)
        .unwrap();
    let sum: Vec2 = (0..corners.len())
        .map(|k| uvs.corner(&mesh, face, k).unwrap())
        .sum();
    (face, sum / corners.len() as f32)
}

/// A spot on the cube's top, where it shows in the viewport.
fn on_top(harness: &H) -> egui::Pos2 {
    on_screen(harness, Vec3::new(0.0, 0.0, 100.0))
}

#[test]
fn w_and_a_drag_on_the_model_slide_the_islands_uvs_as_one_step() {
    let mut harness = in_uv_mode();
    let top = on_top(&harness);
    // Select the top's island with a click on the model.
    harness.event(Event::PointerMoved(top));
    harness.run();
    for pressed in [true, false] {
        button(
            &mut harness,
            top,
            pressed,
            PointerButton::Primary,
            Modifiers::NONE,
        );
    }
    harness.run();
    let (face, before) = side(&harness, Vec3::Z);
    let doc = harness.state().workbench.document();
    assert!(
        doc.elements(cube(&harness))
            .unwrap()
            .contains(Element::Polygon(face))
    );
    press(&mut harness, Modifiers::NONE, Key::W);
    let steps = harness.state().workbench.document().undo_steps();
    drag_by(&mut harness, top, egui::vec2(40.0, 0.0));
    let (_, after) = side(&harness, Vec3::Z);
    assert!((after - before).length() > 0.01, "{before} → {after}");
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    // Only the top moved.
    let (_, front) = side(&harness, -Vec3::Y);
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert!((side(&harness, Vec3::Z).1 - before).length() < 1e-5);
    assert!((side(&harness, -Vec3::Y).1 - front).length() < 1e-5);
}

#[test]
fn a_drag_from_an_island_not_selected_selects_and_turns_it() {
    let mut harness = in_uv_mode();
    press(&mut harness, Modifiers::NONE, Key::R);
    let top = on_top(&harness);
    let (face, _) = side(&harness, Vec3::Z);
    let mesh = shape(&harness);
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let corner = uvs.corner(&mesh, face, 0).unwrap();
    drag_by(&mut harness, top, egui::vec2(60.0, 0.0));
    let doc = harness.state().workbench.document();
    let selected = doc.elements(cube(&harness)).unwrap();
    assert!(selected.contains(Element::Polygon(face)));
    assert_eq!(selected.count(ElementKind::Polygons), 1);
    let mesh = shape(&harness);
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    assert!(
        uvs.corner(&mesh, face, 0).unwrap().distance(corner) > 0.01,
        "turned"
    );
}

#[test]
fn esc_cancels_a_drag_on_the_model() {
    let mut harness = in_uv_mode();
    press(&mut harness, Modifiers::NONE, Key::W);
    let top = on_top(&harness);
    let (_, before) = side(&harness, Vec3::Z);
    let steps = harness.state().workbench.document().undo_steps();
    harness.event(Event::PointerMoved(top));
    harness.run();
    button(
        &mut harness,
        top,
        true,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    for i in 1..=5 {
        harness.event(Event::PointerMoved(top + egui::vec2(i as f32 * 8.0, 0.0)));
        harness.run();
    }
    assert!((side(&harness, Vec3::Z).1 - before).length() > 0.01);
    harness.key_press(Key::Escape);
    harness.run();
    button(
        &mut harness,
        top + egui::vec2(40.0, 0.0),
        false,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    harness.run();
    assert!((side(&harness, Vec3::Z).1 - before).length() < 1e-5);
    // Selecting the island is the only step left.
    assert!(harness.state().workbench.document().undo_steps() <= steps + 1);
}

#[test]
fn the_polygon_under_the_mouse_in_the_uv_editor_lights_up_on_the_model() {
    let mut harness = in_uv_mode();
    let (face, uv) = side(&harness, Vec3::Z);
    let at = harness.state().workbench.uv_editor.screen_of(uv).unwrap();
    harness.event(Event::PointerMoved(at));
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().workbench.components.hovered(),
        Some((cube(&harness), Element::Polygon(face)))
    );
}
