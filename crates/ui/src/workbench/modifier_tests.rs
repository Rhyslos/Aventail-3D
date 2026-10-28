//! Tests for modifiers in the window (decision 0142): the modifiers
//! palette, the Mesh menu and command search add them to the selected
//! shapes; the Attributes panel lists them with their settings; the
//! viewport draws their result, heavy ones worked out in the background.

use super::modelling_tests::{H, cube, note};
use super::tests::{fresh, mesh_group};
use crate::attributes::modifier_rows::ON_LABEL;
use crate::commands::CommandId;
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{Command, Modifier, ModifierKind, ObjectId, ObjectKind, RemeshOptions};

fn modifiers(harness: &H, id: ObjectId) -> Vec<Modifier> {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .modifiers(id)
        .to_vec()
}

/// The fresh window's cube, selected.
fn selected_cube() -> (H, ObjectId) {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    (harness, id)
}

/// The cube's item as last drawn: its edited shape's and drawn shape's
/// polygon counts.
fn drawn(harness: &H, id: ObjectId) -> (usize, usize) {
    let view = harness.state().renderer.drawn.last().unwrap().clone();
    let item = view.scene.items.iter().find(|i| i.id == id).unwrap();
    (item.shown.face_count(), item.drawn.face_count())
}

/// Gives `id` a remesh at `voxel_size`.
fn add_remesh(harness: &mut H, id: ObjectId, voxel_size: f32) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::AddModifier {
            ids: vec![id],
            kind: ModifierKind::Remesh(RemeshOptions {
                voxel_size,
                smooth: 1,
            }),
        })
        .unwrap();
    harness.run();
}

#[test]
fn the_palette_button_adds_a_remesh_and_the_viewport_draws_it() {
    let (mut harness, id) = selected_cube();
    harness.get_by_label("Remesh").click();
    harness.run();
    let list = modifiers(&harness, id);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].kind, ModifierKind::Remesh(RemeshOptions::default()));
    // At the standard 2 cm a 1 m cube is heavy: worked out in the
    // background, then drawn.
    for _ in 0..600 {
        if !harness.state().workbench.drawing.busy() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        harness.run_steps(1);
    }
    harness.run_steps(1);
    let (shown, now) = drawn(&harness, id);
    assert_eq!(shown, 6, "the edited shape stays the cube");
    assert!(now > 1000, "drawn remeshed: {now}");
}

#[test]
fn the_palette_button_says_why_it_cant_be_used() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    let button = harness.get_by_label("Remesh");
    assert!(button.accesskit_node().is_disabled());
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_modifier(CommandId::Remesh, &mut requests);
    assert!(requests.is_empty());
    assert!(note(&harness).contains("Select"), "{}", note(&harness));
}

#[test]
fn the_mesh_menu_and_command_search_add_it_too() {
    let (mut harness, id) = selected_cube();
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    assert!(harness.query_by_label("Clean up").is_some(), "family name");
    harness
        .get_all_by_label("Remesh")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Remesh can be chosen")
        .click();
    harness.run();
    assert_eq!(modifiers(&harness, id).len(), 1);
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries.iter().find(|e| e.name == "Remesh").unwrap();
    assert_eq!(entry.place, "Mesh");
    assert!(entry.not_now.is_none());
    harness.key_press(egui::Key::Space);
    harness.run();
    harness
        .get_by_label(crate::command_search::SEARCH_LABEL)
        .type_text("remesh");
    harness.run();
    harness.key_press(egui::Key::Enter);
    // Remeshing the remeshed cube again is heavy: it is worked out on
    // another thread while the window keeps drawing.
    harness.run_steps(4);
    assert_eq!(modifiers(&harness, id).len(), 2);
}

#[test]
fn the_attributes_panel_shows_the_modifier_picked() {
    let (mut harness, id) = selected_cube();
    // Tall enough for the object's rows and a modifier's settings.
    harness.set_size(egui::vec2(1440.0, 1300.0));
    add_remesh(&mut harness, id, 10.0);
    add_remesh(&mut harness, id, 20.0);
    // With the object picked, only links to them: no settings.
    assert!(harness.query_by_label(ON_LABEL).is_none());
    assert!(harness.query_by_label("Voxel size").is_none());
    assert_eq!(
        harness
            .query_all_by_label("Remesh")
            .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::Label)
            .count(),
        2
    );
    // One picked: its settings alone.
    super::tests::open_modifier(&mut harness, id, 1);
    assert_eq!(harness.get_all_by_label(ON_LABEL).count(), 1);
    // One modifier's Voxel size: its name and its field.
    assert_eq!(harness.get_all_by_label("Voxel size").count(), 2);
    assert!(
        harness.query_by_label("Up").is_none(),
        "ordering is in the lists"
    );
    // Switch it off there.
    harness.get_by_label(ON_LABEL).click();
    harness.run();
    assert!(!modifiers(&harness, id)[1].on);
    assert!(modifiers(&harness, id)[0].on);
    // Back to the cube's own settings.
    harness
        .get_by_label_contains(crate::attributes::modifier_rows::BACK_PREFIX)
        .click();
    harness.run();
    assert!(harness.query_by_label(ON_LABEL).is_none());
    assert!(harness.query_by_label("Voxel size").is_none());
    // A link opens one too.
    harness
        .get_all_by_label("Remesh")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Label)
        .unwrap()
        .click();
    harness.run();
    // One modifier's Voxel size: its name and its field.
    assert_eq!(harness.get_all_by_label("Voxel size").count(), 2);
}

#[test]
fn an_open_shape_says_why_it_isnt_remeshed() {
    let mut harness = fresh();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::Create(ObjectKind::Primitive(
        scene::PrimitiveKind::Plane.standard(),
    )))
    .unwrap();
    let plane = doc.selection()[0];
    harness.run();
    add_remesh(&mut harness, plane, 10.0);
    super::tests::open_modifier(&mut harness, plane, 0);
    assert!(
        harness
            .query_by_label_contains("Not worked out: the shape isn't closed")
            .is_some()
    );
    let (shown, now) = drawn(&harness, plane);
    assert_eq!(shown, now, "drawn unchanged");
}

#[test]
fn bake_into_shape_makes_it_part_of_the_shape() {
    let (mut harness, id) = selected_cube();
    add_remesh(&mut harness, id, 10.0);
    let expected = harness
        .state()
        .workbench
        .document()
        .scene()
        .exported_shape(id)
        .unwrap();
    assert_eq!(
        harness.state().workbench.readiness(CommandId::MakeEditable),
        crate::tool_menu::Ready::Yes
    );
    let mut requests = Vec::new();
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(CommandId::MakeEditable, &ctx, &mut requests);
    harness.state_mut().workbench.carry_out(requests);
    harness.run();
    let scene = harness.state().workbench.document().scene();
    assert!(scene.modifiers(id).is_empty());
    assert_eq!(
        scene.object(id).unwrap().kind,
        ObjectKind::Mesh(expected),
        "the remeshed shape"
    );
}
