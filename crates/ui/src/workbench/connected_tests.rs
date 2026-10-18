//! Tests for Select connected in the window (decision 0118): Alt + A, the
//! Select menu and UV → Pieces select the whole part the selection is on.

use super::modelling_tests::{H, note, press};
use super::tests::fresh;
use super::*;
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{Element, ElementKind, ElementSelection};

/// A shape of two cubes apart from each other, selected, in the
/// Polygons mode, with one polygon of the first selected.
fn two_cubes() -> (H, scene::ObjectId) {
    let mut harness = fresh();
    let cube = scene::Primitive::Cube(mesh::CubeParams::default()).mesh();
    let both = mesh::parts::combine(&[
        (&cube, glam::Mat4::IDENTITY),
        (&cube, glam::Mat4::from_translation(glam::Vec3::X * 300.0)),
    ]);
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Mesh(
        std::sync::Arc::new(both),
    )))
    .unwrap();
    let id = doc.selection()[0];
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let first = shape(&harness, id).faces().next().unwrap().0;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Polygon(first)]))],
            "Select",
            None,
        );
    harness.run();
    (harness, id)
}

fn shape(harness: &H, id: scene::ObjectId) -> std::sync::Arc<scene::Mesh> {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh
}

fn selected(harness: &H, id: scene::ObjectId) -> usize {
    harness
        .state()
        .workbench
        .document()
        .elements(id)
        .map_or(0, |s| s.count(ElementKind::Polygons))
}

#[test]
fn alt_a_selects_the_whole_part_and_only_it() {
    let (mut harness, id) = two_cubes();
    assert_eq!(selected(&harness, id), 1);
    harness.key_press_modifiers(Modifiers::ALT, Key::A);
    harness.run();
    assert_eq!(selected(&harness, id), 6, "one cube, not both");
    assert!(
        harness
            .state()
            .workbench
            .document()
            .undo_label()
            .is_some_and(|l| l.starts_with("Select connected"))
    );
    // Undo puts the one polygon back.
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(selected(&harness, id), 1);
}

#[test]
fn the_select_menu_and_the_uv_menu_have_it_too() {
    let (mut harness, id) = two_cubes();
    super::tests::main_menu(&harness, "Select").click();
    harness.run();
    harness
        .get_by_label_contains(selection::CONNECTED_LABEL)
        .click();
    harness.run();
    assert_eq!(selected(&harness, id), 6);
    let (mut harness, id) = two_cubes();
    // (A two-part shape's notes say "pieces" too, so the group is found
    // as the button it is.)
    super::tests::main_menu(&harness, "UV").click();
    harness.run();
    harness
        .get_all_by_label_contains(UV_PIECES_MENU)
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .unwrap()
        .hover();
    harness.run_steps(8);
    harness
        .get_by_label_contains(selection::CONNECTED_LABEL)
        .click();
    harness.run();
    assert_eq!(selected(&harness, id), 6);
}

#[test]
fn with_nothing_selected_it_says_what_to_do() {
    let (mut harness, id) = two_cubes();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::new())], "Deselect", None);
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::A);
    harness.run();
    assert_eq!(selected(&harness, id), 0);
    assert!(
        note(&harness).contains("select a polygon"),
        "{}",
        note(&harness)
    );
    // In the Model mode there are no elements to select.
    press(&mut harness, Modifiers::NONE, Key::Num3);
    harness.key_press_modifiers(Modifiers::ALT, Key::A);
    harness.run();
    assert!(
        note(&harness).contains("Select connected"),
        "{}",
        note(&harness)
    );
}
