//! Tests for the Weighted normals modifier in the window (decision
//! 0148): the palette button, the Mesh menu and command search add it;
//! the Attributes panel shows its settings and says when a modifier after
//! it drops its normals; the bake check follows its hard edges.

use super::modelling_tests::{H, cube};
use super::tests::{fresh, mesh_group};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{
    Command, Modifier, ModifierKind, ObjectId, Operation, SubdivideOptions, WeightedNormalsOptions,
};

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

fn run(harness: &mut H, command: Command) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(command)
        .unwrap();
    harness.run();
}

#[test]
fn the_palette_button_adds_it_with_its_settings_in_attributes() {
    let (mut harness, id) = selected_cube();
    harness.get_by_label("Weighted normals").click();
    harness.run();
    let list = modifiers(&harness, id);
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].kind,
        ModifierKind::WeightedNormals(WeightedNormalsOptions::default())
    );
    for name in ["Weight by", "Face influence", "Strength", "Keep hard edges"] {
        assert!(harness.query_all_by_label(name).count() >= 1, "{name}");
    }
    // The switch is one undo step.
    let switch = harness
        .get_all_by_label("Keep hard edges")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("the switch");
    switch.scroll_to_me();
    harness.run();
    harness
        .get_all_by_label("Keep hard edges")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("the switch")
        .click();
    harness.run();
    let ModifierKind::WeightedNormals(options) = modifiers(&harness, id)[0].kind else {
        panic!("weighted normals")
    };
    assert!(!options.keep_hard_edges);
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(
        modifiers(&harness, id)[0].kind,
        ModifierKind::WeightedNormals(WeightedNormalsOptions::default())
    );
}

#[test]
fn a_modifier_after_it_is_pointed_out() {
    let (mut harness, id) = selected_cube();
    let note = "move Weighted normals to the end";
    harness.get_by_label("Weighted normals").click();
    harness.run();
    let shown = |harness: &H| {
        harness
            .query_all(egui_kittest::kittest::by().label_contains(note))
            .count()
    };
    assert_eq!(shown(&harness), 0);
    run(
        &mut harness,
        Command::AddModifier {
            ids: vec![id],
            kind: ModifierKind::Subdivide(SubdivideOptions::default()),
        },
    );
    assert_eq!(shown(&harness), 1);
    run(&mut harness, Command::MoveModifier { id, from: 0, to: 1 });
    assert_eq!(shown(&harness), 0);
}

#[test]
fn the_mesh_menu_and_command_search_offer_it() {
    let (mut harness, id) = selected_cube();
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    assert!(harness.query_all_by_label("Game ready").count() >= 1);
    harness
        .get_all_by_label("Weighted normals")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Weighted normals can be chosen")
        .click();
    harness.run();
    assert_eq!(modifiers(&harness, id).len(), 1);
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries
        .iter()
        .find(|e| e.name == "Weighted normals")
        .unwrap();
    assert_eq!(entry.place, "Mesh");
    assert!(entry.not_now.is_none());
}

#[test]
fn the_bake_check_sees_no_hard_edges_when_they_are_smoothed_over() {
    let mut harness = fresh();
    run(
        &mut harness,
        Command::Create(scene::ObjectKind::Primitive(
            scene::PrimitiveKind::Sphere.standard(),
        )),
    );
    let id = harness.state().workbench.document().selection()[0];
    // Shaded by its marks, with one edge the UVs don't split marked hard:
    // the bake check lists it.
    run(
        &mut harness,
        Command::SetHardness {
            ids: vec![id],
            hardness: scene::Hardness::Marked,
        },
    );
    let mesh = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh;
    let splits = mesh::uvs::uv_splits(&mesh);
    let joined = *mesh
        .edges()
        .iter()
        .find(|e| !splits.contains(e))
        .expect("an edge the UVs don't split");
    run(
        &mut harness,
        Command::Record {
            id,
            operation: Operation::SetHardEdges {
                edges: vec![joined],
                hard: true,
                anchors: None,
            },
        },
    );
    let hard_joined = |harness: &mut H| {
        harness
            .state_mut()
            .workbench
            .bake_problems()
            .iter()
            .any(|p| matches!(p.kind, super::baking::Kind::HardJoined(_)))
    };
    assert!(hard_joined(&mut harness));
    // Weighted normals keeping hard edges: still there.
    run(
        &mut harness,
        Command::AddModifier {
            ids: vec![id],
            kind: ModifierKind::WeightedNormals(WeightedNormalsOptions::default()),
        },
    );
    assert!(hard_joined(&mut harness));
    // Smoothing them over: nothing for a normal map to hold there.
    let mut smooth = modifiers(&harness, id)[0].clone();
    smooth.kind = ModifierKind::WeightedNormals(WeightedNormalsOptions {
        keep_hard_edges: false,
        ..Default::default()
    });
    run(
        &mut harness,
        Command::ChangeModifier {
            id,
            index: 0,
            modifier: smooth,
        },
    );
    assert!(!hard_joined(&mut harness));
}
