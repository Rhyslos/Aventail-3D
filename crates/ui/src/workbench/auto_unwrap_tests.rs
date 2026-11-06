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

fn all_faces(m: &scene::Mesh) -> Vec<scene::FaceId> {
    m.faces().map(|(f, _)| f).collect()
}

/// Test pass (fix 6): the Straighten and Set texel density steps give the
/// pieces the intents the menu tools give (decision 0248). Without Pack,
/// the density kept is the one set.
#[test]
fn auto_unwrap_sets_the_same_intents_as_the_tools() {
    let mut harness = fresh();
    add(
        &mut harness,
        scene::Primitive::Cylinder(mesh::CylinderParams {
            caps: false,
            ..Default::default()
        }),
    );
    harness.state_mut().workbench.uv_settings.auto_steps.pack = false;
    harness.state_mut().workbench.run_auto_unwrap();
    harness.run();
    let m = shape(&harness);
    let aim = harness
        .state()
        .workbench
        .document()
        .scene()
        .texture()
        .uv_per_unit();
    assert!(
        matches!(mesh::uv_intents::piece_intent(&m, &all_faces(&m)),
            mesh::uv_intents::UvIntent::Density(d) if (d - aim).abs() < aim * 1e-4),
        "{:?}",
        mesh::uv_intents::piece_intent(&m, &all_faces(&m))
    );
    // Straighten alone: a strip.
    let mut harness = fresh();
    add(
        &mut harness,
        scene::Primitive::Cylinder(mesh::CylinderParams {
            caps: false,
            ..Default::default()
        }),
    );
    let steps_on = &mut harness.state_mut().workbench.uv_settings.auto_steps;
    steps_on.pack = false;
    steps_on.density = false;
    harness.state_mut().workbench.run_auto_unwrap();
    harness.run();
    let m = shape(&harness);
    assert_eq!(
        mesh::uv_intents::piece_intent(&m, &all_faces(&m)),
        mesh::uv_intents::UvIntent::Strip
    );
}

/// With Pack after Set texel density, the pieces keep the density Pack
/// gave them, so a later edit doesn't shrink them back to the project's.
/// Switching Pack off in the window takes that intent with it.
#[test]
fn after_pack_the_pieces_keep_the_packed_density() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness
        .state_mut()
        .workbench
        .uv_settings
        .auto_steps
        .lightmap = false;
    harness.state_mut().workbench.run_auto_unwrap();
    harness.run();
    let m = shape(&harness);
    let faces = all_faces(&m);
    let packed = uv::density::density(&m, &faces).unwrap();
    match mesh::uv_intents::piece_intent(&m, &faces) {
        mesh::uv_intents::UvIntent::Density(d) => {
            assert!((d - packed).abs() < packed * 0.01, "{d} {packed}");
        }
        other => panic!("{other:?}"),
    }
}

/// Test pass (fix 6), the manual's tip: Pack scales every piece together
/// to fill the texture, so on a cube (whose unwrapped pieces already have
/// one density) Set texel density before Pack doesn't change how big the
/// pieces end up. It evens out pieces of different densities instead.
#[test]
fn pack_fills_the_texture_whether_or_not_the_density_was_set() {
    let result = |density: bool| {
        let mut harness = fresh();
        add(&mut harness, scene::Primitive::Cube(Default::default()));
        let on = &mut harness.state_mut().workbench.uv_settings.auto_steps;
        on.density = density;
        on.lightmap = false;
        harness.state_mut().workbench.run_auto_unwrap();
        harness.run();
        uv_values(&shape(&harness)).unwrap()
    };
    let with = result(true);
    let without = result(false);
    assert_eq!(with.len(), without.len());
    assert!(
        with.iter()
            .zip(&without)
            .all(|(a, b)| a.distance(*b) < 1e-4),
        "Pack gave the same layout"
    );
}
