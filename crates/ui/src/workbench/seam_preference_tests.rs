//! Tests for seam preferences and stretch budgets in the window
//! (decision 0111): the UV menu's commands and submenu, what they record,
//! and the budget for every piece reaching Suggest seams and Unwrap.

use super::modelling_tests::{H, note, press};
use super::seam_preferences::*;
use super::tests::fresh;
use super::*;
use egui_kittest::kittest::Queryable;
use scene::{Element, ElementSelection, SeamPreference};

fn sphere(harness: &mut H) -> scene::ObjectId {
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        scene::Primitive::Sphere(Default::default()),
    )))
    .unwrap();
    let id = *doc.selection().first().unwrap();
    harness.run();
    id
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

fn last_operation(harness: &H, id: scene::ObjectId) -> Option<scene::Operation> {
    let doc = harness.state().workbench.document();
    doc.scene()
        .object(id)?
        .stack
        .layers()
        .last()?
        .entries
        .last()
        .map(|e| e.operation.clone())
}

/// The sphere in the Edges mode with two of its inner edges selected.
fn two_edges(harness: &mut H) -> (scene::ObjectId, Vec<scene::EdgeKey>) {
    let id = sphere(harness);
    press(harness, Modifiers::NONE, Key::Num2);
    let mesh = shape(harness, id);
    let topology = mesh.topology();
    let edges: Vec<scene::EdgeKey> = topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 2 && !mesh::uvs::is_seam(&mesh, *e))
        .take(2)
        .collect();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(
                id,
                ElementSelection::of(edges.iter().map(|e| Element::Edge(*e))),
            )],
            "Select",
            None,
        );
    harness.run();
    (id, edges)
}

#[test]
fn preferences_are_one_step_each_and_say_so() {
    for preference in [SeamPreference::Prefer, SeamPreference::Never] {
        let mut harness = fresh();
        let (id, edges) = two_edges(&mut harness);
        let before = harness.state().workbench.document().undo_steps();
        harness
            .state_mut()
            .workbench
            .run_seam_preference(preference);
        harness.run();
        assert_eq!(
            harness.state().workbench.document().undo_steps(),
            before + 1
        );
        match last_operation(&harness, id) {
            Some(scene::Operation::SetSeamPreference {
                edges: recorded,
                preference: p,
                ..
            }) => {
                assert_eq!(p, preference);
                assert_eq!(recorded, edges);
            }
            other => panic!("{other:?}"),
        }
        assert!(note(&harness).contains("Ctrl + Z"), "{}", note(&harness));
        let mesh = shape(&harness, id);
        assert!(
            edges
                .iter()
                .all(|e| mesh::uvs::seam_preference(&mesh, *e) == preference)
        );
        // The same again says so; clearing takes them off.
        harness
            .state_mut()
            .workbench
            .run_seam_preference(preference);
        harness.run();
        assert!(note(&harness).contains("already"), "{}", note(&harness));
        harness
            .state_mut()
            .workbench
            .run_seam_preference(SeamPreference::Free);
        harness.run();
        assert!(mesh::uvs::seam_preferences(&shape(&harness, id)).is_empty());
    }
    // Nothing selected says what to select.
    let mut harness = fresh();
    sphere(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::Num2);
    harness
        .state_mut()
        .workbench
        .run_seam_preference(SeamPreference::Never);
    harness.run();
    assert!(
        note(&harness).contains("select some edges"),
        "{}",
        note(&harness)
    );
}

#[test]
fn never_edges_get_no_seam_from_hard_edges() {
    for never in [false, true] {
        let mut harness = fresh();
        let cube = harness.state().workbench.document().scene().children(None)[0];
        let mesh = shape(&harness, cube);
        let all: Vec<scene::EdgeKey> = mesh.topology().edges().collect();
        let doc = harness.state_mut().workbench.document_mut();
        doc.set_selection([cube]);
        let mut ops = vec![
            scene::Operation::SetSeams {
                edges: mesh::uvs::seams(&mesh),
                seam: false,
                anchors: None,
            },
            scene::Operation::SetHardEdges {
                edges: all.clone(),
                hard: true,
                anchors: None,
            },
        ];
        if never {
            ops.push(scene::Operation::SetSeamPreference {
                edges: all.clone(),
                preference: SeamPreference::Never,
                anchors: None,
            });
        }
        for operation in ops {
            doc.execute(scene::Command::Record {
                id: cube,
                operation,
            })
            .unwrap();
        }
        harness.run();
        harness.state_mut().workbench.run_seams_from_hard_edges();
        harness.run();
        let seams = mesh::uvs::seams(&shape(&harness, cube)).len();
        assert_eq!(seams == 0, never, "{seams} seams, never {never}");
    }
}

#[test]
fn a_budget_goes_on_whole_pieces_and_reaches_the_tools() {
    let mut harness = fresh();
    let id = sphere(&mut harness);
    harness.state_mut().workbench.uv_settings.piece_budget = 1.3;
    let budget = harness.state().workbench.uv_settings.piece_budget;
    harness
        .state_mut()
        .workbench
        .run_stretch_budget(Some(budget));
    harness.run();
    let mesh = shape(&harness, id);
    let faces: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).collect();
    assert_eq!(mesh::uvs::budget_of(&mesh, &faces), Some(1.3));
    assert!(note(&harness).contains("1.30×"), "{}", note(&harness));
    harness.state_mut().workbench.run_stretch_budget(None);
    harness.run();
    assert!(mesh::uvs::budgets(&shape(&harness, id)).is_none());
    // The budget for every piece reaches Unwrap, kept in range.
    {
        let s = &mut harness.state_mut().workbench.uv_settings;
        s.budget_on = true;
        s.budget = 40.0;
    }
    let ctx = harness.ctx.clone();
    harness.state_mut().workbench.run_command(
        crate::commands::CommandId::Unwrap,
        &ctx,
        &mut Vec::new(),
    );
    harness.run();
    match last_operation(&harness, id) {
        Some(scene::Operation::Unwrap { options, .. }) => {
            assert_eq!(options.budget, Some(*scene::BUDGET_RANGE.end()));
        }
        other => panic!("{other:?}"),
    }
    harness.state_mut().workbench.uv_settings.budget_on = false;
    assert_eq!(harness.state().workbench.suggest_options().budget, None);
}

#[test]
fn the_uv_menu_holds_the_commands_and_the_budget_submenu() {
    let mut harness = fresh();
    sphere(&mut harness);
    super::tests::uv_group(&mut harness, super::UV_SEAMS_MENU);
    for label in [PREFER_LABEL, NEVER_LABEL, CLEAR_PREFERENCE_LABEL] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    harness.key_press(egui::Key::Escape);
    harness.run();
    super::tests::uv_group(&mut harness, super::UV_PIECES_MENU);
    harness.get_by_label_contains(BUDGET_LABEL).hover();
    harness.run();
    for label in [SET_BUDGET_LABEL, CLEAR_BUDGET_LABEL, BUDGET_VALUE_LABEL] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    harness.get_by_label(SET_BUDGET_LABEL).click();
    harness.run();
    assert!(
        note(&harness).contains("stretch budget"),
        "{}",
        note(&harness)
    );
}
