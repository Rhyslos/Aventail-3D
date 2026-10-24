//! Tests for Weld by distance in the window (decision 0152): the palette
//! button, the Mesh menu and command search add the modifier, with its
//! settings in the Attributes panel; Weld by distance now, in the Mesh
//! menu's Clean up group, welds whole shapes in the Model mode and the
//! selected elements' points in the other modes, one undo step, its
//! settings under Last edit.

use super::modelling_tests::{H, cube, last_operation, note, press, shape};
use super::tests::{empty, mesh_group};
use super::*;
use crate::commands::CommandId;
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{
    Command, Element, ElementSelection, Mesh, ModifierKind, ObjectKind, Operation, VertexId,
    WeldOptions,
};
use std::sync::Arc;

/// A cube 100 wide whose six sides each have their own four points.
fn split_cube_mesh() -> Mesh {
    let corners = [
        [0.0, 0.0, 0.0],
        [100.0, 0.0, 0.0],
        [100.0, 100.0, 0.0],
        [0.0, 100.0, 0.0],
        [0.0, 0.0, 100.0],
        [100.0, 0.0, 100.0],
        [100.0, 100.0, 100.0],
        [0.0, 100.0, 100.0],
    ];
    let sides: [[usize; 4]; 6] = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ];
    let mut mesh = Mesh::new();
    for side in sides {
        let ids: Vec<VertexId> = side
            .iter()
            .map(|c| mesh.add_vertex(glam::Vec3::from(corners[*c])).unwrap())
            .collect();
        mesh.add_face(&ids).unwrap();
    }
    mesh
}

/// A window whose only object is a split cube (a plain mesh), selected,
/// in the Model mode.
fn split_cube() -> H {
    let mut harness = empty();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::Create(ObjectKind::Mesh(Arc::new(
            split_cube_mesh(),
        ))))
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

fn shown_points(harness: &H) -> usize {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .shown_shape(cube(harness))
        .unwrap()
        .vertex_count()
}

#[test]
fn the_palette_button_adds_the_modifier_with_its_settings_in_attributes() {
    let mut harness = split_cube();
    harness.get_by_label("Weld by distance").click();
    harness.run();
    let doc = harness.state().workbench.document();
    let list = doc.scene().modifiers(cube(&harness)).to_vec();
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].kind,
        ModifierKind::WeldByDistance(WeldOptions::default())
    );
    assert_eq!(shown_points(&harness), 8);
    for name in ["Distance", "Only at open borders", "Keep UV seams"] {
        assert!(harness.query_all_by_label(name).count() >= 1, "{name}");
    }
}

#[test]
fn the_mesh_menu_and_command_search_offer_both() {
    let mut harness = split_cube();
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    harness
        .get_all_by_label("Weld by distance")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Weld by distance can be chosen")
        .click();
    harness.run();
    assert_eq!(shown_points(&harness), 8);
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    for name in ["Weld by distance", "Weld by distance now"] {
        let entry = entries.iter().find(|e| e.name == name).unwrap();
        assert_eq!(entry.place, "Mesh", "{name}");
        assert!(entry.not_now.is_none(), "{name}");
    }
}

#[test]
fn weld_now_from_the_clean_up_group_welds_the_whole_shape() {
    let mut harness = split_cube();
    let steps = harness.state().workbench.document().undo_steps();
    mesh_group(&mut harness, super::mesh_menu::MESH_CLEAN_UP_MENU);
    harness
        .get_all_by_label("Weld by distance now")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Weld by distance now can be chosen")
        .click();
    harness.run();
    assert_eq!(shape(&harness).vertex_count(), 8);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Weld by distance"));
    assert!(doc.scene().modifiers(cube(&harness)).is_empty());
    assert!(
        note(&harness).contains("Welded 16 points away"),
        "{}",
        note(&harness)
    );
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::WeldByDistance { ref points, .. }) if points.is_empty()
    ));
    // Its settings show under Last edit.
    assert!(harness.query_all_by_label("Only at open borders").count() >= 1);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(shape(&harness).vertex_count(), 24);
}

#[test]
fn weld_now_in_the_points_mode_welds_only_the_selected_points() {
    let mut harness = split_cube();
    press(&mut harness, Modifiers::NONE, Key::Num1);
    let mesh = shape(&harness);
    let chosen: Vec<Element> = mesh
        .vertices()
        .filter(|(_, p)| p.length() < 1e-3)
        .map(|(v, _)| Element::Point(v))
        .collect();
    assert_eq!(chosen.len(), 3);
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(chosen))], "Select", None);
    harness.run();
    run(&mut harness, CommandId::WeldByDistanceNow);
    assert_eq!(shape(&harness).vertex_count(), 22);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Weld 3 points by distance")
    );
}

#[test]
fn weld_now_in_the_polygons_mode_welds_the_selected_polygons_points() {
    let mut harness = split_cube();
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let chosen: Vec<Element> = shape(&harness)
        .faces()
        .take(3)
        .map(|(f, _)| Element::Polygon(f))
        .collect();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(chosen))], "Select", None);
    harness.run();
    run(&mut harness, CommandId::WeldByDistanceNow);
    // The bottom, top and front sides share four corners pairwise.
    assert!(shape(&harness).vertex_count() < 24);
    assert!(shape(&harness).vertex_count() > 8);
}

#[test]
fn weld_now_says_why_when_it_cant() {
    // The Object mode: greyed out with the reason.
    let mut harness = split_cube();
    harness.state_mut().workbench.mode = crate::palette::Mode::Object;
    assert!(matches!(
        harness
            .state()
            .workbench
            .readiness(CommandId::WeldByDistanceNow),
        crate::tool_menu::Ready::NotNow(_)
    ));
    // The Points mode with nothing selected.
    let mut harness = split_cube();
    press(&mut harness, Modifiers::NONE, Key::Num1);
    let steps = harness.state().workbench.document().undo_steps();
    run(&mut harness, CommandId::WeldByDistanceNow);
    assert!(
        note(&harness).contains("select two or more points"),
        "{}",
        note(&harness)
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
}

#[test]
fn too_small_a_distance_still_records_so_it_can_be_raised() {
    let mut harness = super::tests::fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    let before = shape(&harness).vertex_count();
    run(&mut harness, CommandId::WeldByDistanceNow);
    assert_eq!(shape(&harness).vertex_count(), before);
    assert!(
        note(&harness).contains("found no points"),
        "{}",
        note(&harness)
    );
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::WeldByDistance { .. })
    ));
    // The setting is remembered for the next one.
    let wide = WeldOptions {
        distance: 50.0,
        ..WeldOptions::default()
    };
    harness.state_mut().workbench.attributes.weld = wide;
    run(&mut harness, CommandId::WeldByDistanceNow);
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::WeldByDistance { options, .. }) if options == wide
    ));
}
