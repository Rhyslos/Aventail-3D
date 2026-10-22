//! Tests for Select connected in the window (decision 0118): Alt + A and
//! the Select menu select the whole part the selection is on; over the UV
//! Editor, in the UV mode and in UV → Pieces, the whole UV pieces instead
//! (decision 0124).

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
    // The UV menu's means the pieces: a plain cube's side is a piece.
    assert_eq!(selected(&harness, id), 1);
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

/// The UV Edit layout with a cube of 3 × 3 polygons a side, edited in the
/// Polygons mode, with one polygon selected; and the polygons of its side.
fn uv_edit_with_a_segmented_cube() -> (H, scene::ObjectId, usize) {
    let mut harness = fresh();
    let cube = scene::Primitive::Cube(mesh::CubeParams {
        segments: [3, 3, 3],
        ..Default::default()
    })
    .mesh();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Mesh(
        std::sync::Arc::new(cube),
    )))
    .unwrap();
    let id = doc.selection()[0];
    harness
        .state_mut()
        .workbench
        .switch_layout(super::LayoutChoice::UvEdit);
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
    let mesh = shape(&harness, id);
    let side = mesh::uvs::islands(&mesh)
        .into_iter()
        .find(|i| i.contains(&first))
        .unwrap()
        .len();
    assert_eq!(side, 9);
    (harness, id, side)
}

fn hover_uv_editor(harness: &mut H) {
    let centre = harness
        .get_by_label(crate::uv_editor::UV_EDITOR_LABEL)
        .rect()
        .center();
    harness.event(egui::Event::PointerMoved(centre));
    harness.run();
}

#[test]
fn alt_a_over_the_uv_editor_selects_the_piece_not_the_part() {
    let (mut harness, id, side) = uv_edit_with_a_segmented_cube();
    hover_uv_editor(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, Key::A);
    harness.run();
    assert_eq!(selected(&harness, id), side, "one side of the cube");
    assert!(
        harness
            .state()
            .workbench
            .document()
            .undo_label()
            .is_some_and(|l| l.starts_with("Select whole pieces"))
    );
    // Away from the UV Editor, the same key takes the whole part.
    harness.event(egui::Event::PointerMoved(egui::pos2(2.0, 2.0)));
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::A);
    harness.run();
    assert_eq!(selected(&harness, id), 6 * side, "the whole cube");
}

#[test]
fn alt_a_over_the_uv_editor_works_from_points() {
    let (mut harness, id, _) = uv_edit_with_a_segmented_cube();
    let mesh = shape(&harness, id);
    let topology = mesh.topology();
    let (island_of, _) = mesh::uvs::island_of_each(&mesh);
    // A point in the middle of a side.
    let inner = mesh
        .vertices()
        .map(|(v, _)| v)
        .find(|v| {
            let faces = topology.vertex_faces(*v);
            faces.len() == 4
                && faces
                    .iter()
                    .all(|f| island_of[f.get() as usize] == island_of[faces[0].get() as usize])
        })
        .unwrap();
    press(&mut harness, Modifiers::NONE, Key::Num1);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Point(inner)]))],
            "Select",
            None,
        );
    harness.run();
    hover_uv_editor(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, Key::A);
    harness.run();
    let points = harness
        .state()
        .workbench
        .document()
        .elements(id)
        .map_or(0, |s| s.count(ElementKind::Points));
    assert_eq!(points, 16, "the 4 × 4 points of one side");
}

#[test]
fn in_the_uv_mode_alt_a_keeps_to_pieces_anywhere() {
    let (mut harness, id, side) = uv_edit_with_a_segmented_cube();
    harness.state_mut().workbench.mode = crate::palette::Mode::Uv;
    harness.run();
    harness.event(egui::Event::PointerMoved(egui::pos2(2.0, 2.0)));
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::A);
    harness.run();
    assert_eq!(selected(&harness, id), side);
}
