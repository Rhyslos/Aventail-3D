//! Tests for Connect (J), Bridge (C) and Weld (M, Ctrl + M) in the window.

use super::modelling_tests::{H, button, cube, last_operation, note, on_screen, press, shape};
use super::tests::fresh;
use super::*;
use crate::transforming::Tool;
use egui::{Event, PointerButton};
use egui_kittest::kittest::Queryable;
use scene::{Element, ElementSelection, Operation};

/// The fresh window's cube selected, in `mode_key`'s mode, with
/// `elements` selected.
fn ready(mode_key: Key, elements: impl Fn(&scene::Mesh) -> Vec<Element>) -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    press(&mut harness, Modifiers::NONE, mode_key);
    let chosen = elements(&shape(&harness));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(chosen))], "Select", None);
    harness.run();
    harness
}

fn uprights(mesh: &scene::Mesh) -> Vec<Element> {
    mesh.edges()
        .iter()
        .filter(|e| {
            let (a, b) = e.ends();
            (mesh.position(a).unwrap().z - mesh.position(b).unwrap().z).abs() > 1.0
        })
        .map(|e| Element::Edge(*e))
        .collect()
}

fn top_points(mesh: &scene::Mesh) -> Vec<Element> {
    mesh.vertices()
        .filter(|(_, p)| p.z > 99.0)
        .map(|(v, _)| Element::Point(v))
        .collect()
}

#[test]
fn j_connects_the_selected_edges() {
    let mut harness = ready(Key::Num2, uprights);
    let steps = harness.state().workbench.document().undo_steps();
    press(&mut harness, Modifiers::NONE, Key::J);
    assert_eq!(shape(&harness).face_count(), 10);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Connect 4 edges"));
    // Its cuts are the Last edit.
    assert!(
        harness
            .query_by_label_contains("Last edit: Connect 4 edges")
            .is_some()
    );
    // In the Polygons mode it says where it works.
    press(&mut harness, Modifiers::NONE, Key::Num3);
    press(&mut harness, Modifiers::NONE, Key::J);
    assert!(note(&harness).starts_with("Connect works on points or edges"));
}

#[test]
fn m_welds_the_selected_points_at_their_middle() {
    let mut harness = ready(Key::Num1, top_points);
    press(&mut harness, Modifiers::NONE, Key::M);
    let mesh = shape(&harness);
    assert_eq!(mesh.vertex_count(), 5);
    assert!(
        mesh.vertices()
            .any(|(_, p)| p.distance(glam::Vec3::new(0.0, 0.0, 100.0)) < 1e-3)
    );
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Weld 4 points")
    );
    assert!(mesh::MeshReport::new(&mesh).is_closed());
}

#[test]
fn c_bridges_two_polygons_or_picks_the_bridge_tool() {
    // Nothing that can be bridged: C picks the tool.
    let mut harness = ready(Key::Num2, |_| Vec::new());
    press(&mut harness, Modifiers::NONE, Key::C);
    assert_eq!(harness.state().workbench.tool(), Tool::Bridge);
    assert!(note(&harness).starts_with("Bridge: drag"));
    // Two polygons: bridged at once.
    let mut harness = ready(Key::Num3, |mesh| {
        mesh.faces()
            .map(|(f, _)| f)
            .filter(|f| mesh.face_normal(*f).z.abs() > 0.9)
            .map(Element::Polygon)
            .collect()
    });
    press(&mut harness, Modifiers::NONE, Key::C);
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::Bridge { ref faces, .. }) if faces.len() == 2
    ));
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    assert!(harness.query_all_by_label("Twist").count() >= 1);
}

#[test]
fn ctrl_m_drags_a_point_onto_another() {
    let mut harness = ready(Key::Num1, |_| Vec::new());
    harness.event(Event::Key {
        key: Key::M,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    });
    harness.run();
    assert_eq!(harness.state().workbench.tool(), Tool::Weld);
    // The two front top corners, seen from the opening view.
    let from = on_screen(&harness, glam::Vec3::new(-50.0, -50.0, 100.0));
    let to = on_screen(&harness, glam::Vec3::new(50.0, -50.0, 100.0));
    harness.event(Event::PointerMoved(from));
    harness.run();
    button(
        &mut harness,
        from,
        true,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    for k in 1..=5 {
        harness.event(Event::PointerMoved(from + (to - from) * (k as f32 / 5.0)));
        harness.run();
    }
    assert!(harness.state().workbench.viewport().tool_path().is_some());
    button(
        &mut harness,
        to,
        false,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let mesh = shape(&harness);
    assert_eq!(mesh.vertex_count(), 7);
    // Melted onto the target, which stays put.
    assert!(
        mesh.vertices()
            .any(|(_, p)| p.distance(glam::Vec3::new(50.0, -50.0, 100.0)) < 1e-3)
    );
    assert!(
        !mesh
            .vertices()
            .any(|(_, p)| p.distance(glam::Vec3::new(-50.0, -50.0, 100.0)) < 1e-3)
    );
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::Weld {
            target: Some(_),
            ..
        })
    ));
}
