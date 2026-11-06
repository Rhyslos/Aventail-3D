//! Tests for Auto-unwrap (decision 0245).

use super::auto_unwrap::{AUTO_UNWRAP_LABEL, AutoStep, RUN_LABEL};
use super::modelling_tests::note;
use super::seams_tests::{add, shape, steps};
use super::tests::{fresh, uv_group};
use super::*;
use egui_kittest::kittest::Queryable;

/// Every corner's UV of the shape.
fn uv_values(m: &scene::Mesh) -> Option<Vec<glam::Vec2>> {
    mesh::uvs::Uvs::of(m).map(|u| u.into_values(m.face_layout().1.len()))
}

fn the_object(harness: &egui_kittest::Harness<'static, tests::State>) -> scene::ObjectId {
    harness.state().workbench.document().selection()[0]
}

#[test]
fn every_step_is_an_edit_in_one_layer_and_one_undo_step() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let before = steps(&harness);
    harness.state_mut().workbench.run_auto_unwrap();
    harness.run();
    assert_eq!(steps(&harness), before + 1);
    let id = the_object(&harness);
    let workbench = &harness.state().workbench;
    let object = workbench.document().scene().object(id).unwrap();
    let top = object.stack.layers().last().unwrap();
    assert_eq!(top.name, AUTO_UNWRAP_LABEL);
    let report = workbench.auto_report.as_ref().unwrap();
    assert!(report.open);
    assert_eq!(report.steps.len(), AutoStep::ALL.len());
    // Every recorded edit is in the new layer, and named by a step.
    let in_layer: Vec<_> = top.entries.iter().map(|e| e.id).collect();
    for done in &report.steps {
        for (_, edit) in &done.edits {
            assert!(in_layer.contains(edit));
        }
        assert!(!done.said.is_empty());
    }
    assert!(mesh::uv_sets::names(&shape(&harness)).contains(&"lightmap".to_string()));
    assert!(note(&harness).contains("Ctrl + Z"), "{}", note(&harness));
}

#[test]
fn undo_takes_it_all_back() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let id = the_object(&harness);
    let layers = |h: &egui_kittest::Harness<'static, tests::State>| {
        h.state()
            .workbench
            .document()
            .scene()
            .object(id)
            .unwrap()
            .stack
            .layers()
            .len()
    };
    let before = layers(&harness);
    let uvs = uv_values(&shape(&harness));
    harness.state_mut().workbench.run_auto_unwrap();
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(layers(&harness), before);
    assert_eq!(uv_values(&shape(&harness)), uvs);
}

#[test]
fn a_step_switched_off_comes_back_on() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.run_auto_unwrap();
    let report = harness.state().workbench.auto_report.clone().unwrap();
    let pack = report
        .steps
        .iter()
        .find(|s| s.step == AutoStep::Pack)
        .unwrap()
        .clone();
    assert!(!pack.edits.is_empty());
    let packed = uv_values(&shape(&harness));
    let before = steps(&harness);
    harness.state_mut().workbench.switch_auto_step(&pack, false);
    harness.run();
    assert_eq!(steps(&harness), before + 1);
    let id = pack.edits[0].0;
    let enabled = |h: &egui_kittest::Harness<'static, tests::State>| {
        h.state()
            .workbench
            .document()
            .scene()
            .object(id)
            .unwrap()
            .stack
            .entry(pack.edits[0].1)
            .unwrap()
            .2
            .enabled
    };
    assert!(!enabled(&harness));
    harness.state_mut().workbench.switch_auto_step(&pack, true);
    harness.run();
    assert!(enabled(&harness));
    assert_eq!(uv_values(&shape(&harness)), packed);
}

#[test]
fn steps_switched_off_in_the_menu_are_skipped() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    {
        let steps = &mut harness.state_mut().workbench.uv_settings.auto_steps;
        steps.lightmap = false;
        steps.straighten = false;
    }
    harness.state_mut().workbench.run_auto_unwrap();
    let report = harness.state().workbench.auto_report.clone().unwrap();
    assert_eq!(report.steps.len(), 4);
    assert!(report.steps.iter().all(|s| s.step != AutoStep::Lightmap));
    assert!(!mesh::uv_sets::names(&shape(&harness)).contains(&"lightmap".to_string()));
}

#[test]
fn nothing_selected_or_every_step_off_says_so() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.state_mut().workbench.run_auto_unwrap();
    assert!(
        note(&harness).contains("select the objects"),
        "{}",
        note(&harness)
    );
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let before = steps(&harness);
    for step in AutoStep::ALL {
        *step.switch(&mut harness.state_mut().workbench.uv_settings.auto_steps) = false;
    }
    harness.state_mut().workbench.run_auto_unwrap();
    assert_eq!(steps(&harness), before);
    assert!(
        note(&harness).contains("switched off"),
        "{}",
        note(&harness)
    );
}

#[test]
fn an_icosphere_without_quads_skips_straightening_in_words() {
    let mut harness = fresh();
    add(
        &mut harness,
        scene::Primitive::Icosphere(Default::default()),
    );
    harness.state_mut().workbench.run_auto_unwrap();
    let report = harness.state().workbench.auto_report.clone().unwrap();
    let straighten = report
        .steps
        .iter()
        .find(|s| s.step == AutoStep::Straighten)
        .unwrap();
    assert!(straighten.edits.is_empty());
    assert!(
        straighten.said.contains("nothing to straighten"),
        "{}",
        straighten.said
    );
}

#[test]
fn the_menu_runs_it_and_the_window_lists_the_steps() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    uv_group(&mut harness, AUTO_UNWRAP_LABEL);
    harness.get_by_label(RUN_LABEL).click();
    harness.run();
    assert!(harness.state().workbench.auto_report.is_some());
    for step in AutoStep::ALL {
        assert!(
            harness.query_all_by_label(step.label()).next().is_some(),
            "{}",
            step.label()
        );
    }
    harness.get_by_label("Close").click();
    harness.run();
    assert!(!harness.state().workbench.auto_report.as_ref().unwrap().open);
}

#[test]
fn old_settings_without_the_steps_read_as_all_on() {
    let old: UvSettings = ron::from_str("(padding: 8)").unwrap();
    assert_eq!(old.auto_steps, auto_unwrap::AutoSteps::default());
    assert!(old.auto_steps.lightmap);
}
