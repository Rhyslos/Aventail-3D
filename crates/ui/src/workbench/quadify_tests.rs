//! Tests for Quadify in the window (decision 0150): the palette button,
//! the Mesh menu and command search add the modifier, with its settings
//! in the Attributes panel; Quadify now, in the Mesh menu's Clean up
//! group, joins whole shapes in the Model mode and the selected polygons
//! in the Polygons mode, one undo step, its settings under Last edit.

use super::modelling_tests::{H, cube, last_operation, note, press, shape};
use super::tests::{empty, mesh_group};
use super::*;
use crate::commands::CommandId;
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{
    Command, Element, ElementSelection, ModifierKind, ObjectKind, Operation, QuadifyOptions,
};
use std::sync::Arc;

/// A window whose only object is a cube cut into triangles (a plain
/// mesh), selected, in the Model mode.
fn triangulated_cube() -> H {
    let mut harness = empty();
    let cube_mesh = scene::PrimitiveKind::Cube.standard().mesh();
    let tris = mesh::decimate::Decimator::new(
        &cube_mesh,
        mesh::decimate::Keep::default(),
        mesh::Hardness::Marked,
    )
    .unwrap()
    .mesh(false);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::Create(ObjectKind::Mesh(Arc::new(tris))))
        .unwrap();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness
}

fn run(harness: &mut H, command: CommandId) {
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut Vec::new());
    harness.run();
}

fn shown(harness: &H) -> usize {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .shown_shape(cube(harness))
        .unwrap()
        .face_count()
}

#[test]
fn the_palette_button_adds_the_modifier_with_its_settings_in_attributes() {
    let mut harness = triangulated_cube();
    harness.get_by_label("Quadify").click();
    harness.run();
    let doc = harness.state().workbench.document();
    let list = doc.scene().modifiers(cube(&harness)).to_vec();
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].kind,
        ModifierKind::Quadify(QuadifyOptions::default())
    );
    assert_eq!(shown(&harness), 6);
    for name in [
        "Max angle",
        "Max shape distortion",
        "Keep UV seams",
        "Keep hard edges",
        "Keep material borders",
    ] {
        assert!(harness.query_all_by_label(name).count() >= 1, "{name}");
    }
}

#[test]
fn the_mesh_menu_and_command_search_offer_both() {
    let mut harness = triangulated_cube();
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    harness
        .get_all_by_label("Quadify")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Quadify can be chosen")
        .click();
    harness.run();
    assert_eq!(shown(&harness), 6);
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    for name in ["Quadify", "Quadify now"] {
        let entry = entries.iter().find(|e| e.name == name).unwrap();
        assert_eq!(entry.place, "Mesh", "{name}");
        assert!(entry.not_now.is_none(), "{name}");
    }
}

#[test]
fn quadify_now_from_the_clean_up_group_joins_the_whole_shape() {
    let mut harness = triangulated_cube();
    let steps = harness.state().workbench.document().undo_steps();
    mesh_group(&mut harness, super::mesh_menu::MESH_CLEAN_UP_MENU);
    harness
        .get_all_by_label("Quadify now")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Quadify now can be chosen")
        .click();
    harness.run();
    assert_eq!(shape(&harness).face_count(), 6);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Quadify"));
    assert!(doc.scene().modifiers(cube(&harness)).is_empty());
    assert!(
        note(&harness).contains("Joined 6 pairs"),
        "{}",
        note(&harness)
    );
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::Quadify { ref faces, .. }) if faces.is_empty()
    ));
    // Its settings show under Last edit.
    assert!(harness.query_all_by_label("Max shape distortion").count() >= 1);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(shape(&harness).face_count(), 12);
}

#[test]
fn quadify_now_in_the_polygons_mode_joins_the_selected_polygons() {
    let mut harness = triangulated_cube();
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let mesh = shape(&harness);
    let pairs = mesh::quadify::pairs(
        &mesh,
        None,
        &QuadifyOptions::default(),
        mesh::Hardness::Marked,
    );
    let chosen: Vec<Element> = pairs[..2]
        .iter()
        .flatten()
        .map(|f| Element::Polygon(*f))
        .collect();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(chosen))], "Select", None);
    harness.run();
    run(&mut harness, CommandId::QuadifyNow);
    assert_eq!(shape(&harness).face_count(), 10);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Quadify 4 polygons")
    );
}

#[test]
fn quadify_now_says_why_when_it_cant() {
    // The Edges mode: greyed out with the reason.
    let mut harness = triangulated_cube();
    press(&mut harness, Modifiers::NONE, Key::Num2);
    assert!(matches!(
        harness.state().workbench.readiness(CommandId::QuadifyNow),
        crate::tool_menu::Ready::NotNow(_)
    ));
    // The Polygons mode with nothing selected.
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let steps = harness.state().workbench.document().undo_steps();
    run(&mut harness, CommandId::QuadifyNow);
    assert!(
        note(&harness).contains("select some polygons"),
        "{}",
        note(&harness)
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    // A cube of quads has nothing to join: nothing is recorded.
    let mut harness = super::tests::fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    let steps = harness.state().workbench.document().undo_steps();
    run(&mut harness, CommandId::QuadifyNow);
    assert!(
        note(&harness).contains("no two neighbouring"),
        "{}",
        note(&harness)
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
}

#[test]
fn settings_too_strict_still_record_so_they_can_be_loosened() {
    // An icosphere's triangles pair into slanted quads only.
    let mut harness = empty();
    let ball = scene::PrimitiveKind::Icosphere.standard();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::Create(ObjectKind::Primitive(ball)))
        .unwrap();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    let before = shape(&harness).face_count();
    let strict = QuadifyOptions {
        max_distortion: 0.0,
        ..QuadifyOptions::default()
    };
    harness.state_mut().workbench.attributes.quadify = strict;
    run(&mut harness, CommandId::QuadifyNow);
    assert_eq!(shape(&harness).face_count(), before);
    assert!(
        note(&harness).contains("joined nothing"),
        "{}",
        note(&harness)
    );
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::Quadify { options, .. }) if options == strict
    ));
    // The standard settings join most of them.
    harness.state_mut().workbench.attributes.quadify = QuadifyOptions::default();
    run(&mut harness, CommandId::QuadifyNow);
    assert!(shape(&harness).face_count() < before * 2 / 3);
}
