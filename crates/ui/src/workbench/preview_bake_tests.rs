//! Tests for the live bake preview and its problems (decision 0252).

use super::baker::{PreviewOf, problem_words};
use super::seams_tests::add;
use super::*;
use crate::viewport::Surface;
use bake::maps::{MapKind, Problem};
use egui_kittest::kittest::Queryable;

#[test]
fn the_preview_shows_on_the_model_and_goes_when_stopped() {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let ctx = harness.ctx.clone();
    let w = &mut harness.state_mut().workbench;
    let before = w.viewport().options().surface;
    w.baker.open = true;
    w.baker.preview.on = true;
    w.baker.preview.of = PreviewOf::Map(MapKind::Normal);
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    assert_eq!(w.viewport().options().surface, Surface::Texture);
    let picture = w.viewport().picture().expect("the preview");
    assert!(picture.size <= super::baker::PREVIEW_SIZE);
    w.stop_bake_preview();
    assert_eq!(w.viewport().options().surface, before);
    assert!(w.viewport().picture().is_none());
}

#[test]
fn a_change_bakes_the_preview_again() {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let ctx = harness.ctx.clone();
    let w = &mut harness.state_mut().workbench;
    w.baker.open = true;
    w.baker.preview.on = true;
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    let first = std::sync::Arc::as_ptr(w.viewport().picture().unwrap());
    // The same: nothing new.
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    assert_eq!(
        std::sync::Arc::as_ptr(w.viewport().picture().unwrap()),
        first
    );
    w.baker.preview.of = PreviewOf::Map(MapKind::Id);
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    assert_ne!(
        std::sync::Arc::as_ptr(w.viewport().picture().unwrap()),
        first
    );
}

#[test]
fn problems_need_a_high_object_and_are_counted() {
    let mut harness = tests::fresh();
    let low = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let high = add(&mut harness, scene::Primitive::Sphere(Default::default()));
    let ctx = harness.ctx.clone();
    let w = &mut harness.state_mut().workbench;
    w.document_mut().set_selection([low]);
    w.baker.open = true;
    w.baker.preview.on = true;
    w.baker.preview.of = PreviewOf::Problems;
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    assert!(
        w.baker.preview.problems.is_none(),
        "baked from itself: nothing to check"
    );
    w.baker.high = Some(high);
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    let problems = w.baker.preview.problems.expect("counted");
    assert!(problems.texels > 0);
    // A sphere over a cube of the same size misses its corners.
    assert!(
        problems.missed + problems.wrong_part + problems.skewed > 0,
        "{problems:?}"
    );
}

#[test]
fn each_problem_says_what_to_do() {
    for problem in Problem::ALL {
        let words = problem_words(problem, 0.12);
        assert!(words.contains("12.0%") && words.contains("Fix"), "{words}");
    }
}

#[test]
fn a_preview_with_nothing_to_show_is_not_started_again_and_again() {
    let mut harness = tests::fresh();
    let low = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let ctx = harness.ctx.clone();
    let w = &mut harness.state_mut().workbench;
    w.document_mut().set_selection([low]);
    w.baker.open = true;
    w.baker.preview.on = true;
    w.baker.preview.of = PreviewOf::Problems;
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    // Baked from itself there are no problems to look for: once found,
    // nothing more runs until something changes.
    w.update_bake_preview(&ctx);
    assert!(!w.bake_preview_running());
    assert!(w.baker.preview.problems.is_none());
}

#[test]
fn the_preview_goes_when_nothing_is_selected() {
    // Regression: with the selection cleared, the last preview stayed
    // painted on the model.
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let ctx = harness.ctx.clone();
    let w = &mut harness.state_mut().workbench;
    let before = w.viewport().options().surface;
    w.baker.open = true;
    w.baker.preview.on = true;
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    assert!(w.viewport().picture().is_some());
    w.document_mut().clear_selection();
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    assert!(w.viewport().picture().is_none());
    assert_eq!(w.viewport().options().surface, before);
    // Still switched on: selecting again brings it back.
    assert!(w.baker.preview.on);
}

#[test]
fn problems_without_a_high_object_say_why() {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    {
        let w = &mut harness.state_mut().workbench;
        w.baker.open = true;
        w.baker.preview.on = true;
        w.baker.preview.of = PreviewOf::Problems;
    }
    // The preview bakes in the background and shows a spinner meanwhile,
    // so step the frames rather than waiting for them to settle (a slow
    // machine is still baking when `run` gives up).
    harness.run_steps(2);
    let ctx = harness.ctx.clone();
    harness.state_mut().workbench.finish_bake_preview(&ctx);
    harness.run_steps(2);
    assert!(
        harness
            .query_all_by_label_contains("choose a high-detail object")
            .next()
            .is_some(),
        "no word about needing a high-detail object"
    );
}

#[test]
fn the_preview_bakes_again_when_stacked_copies_are_switched() {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let ctx = harness.ctx.clone();
    let w = &mut harness.state_mut().workbench;
    w.baker.open = true;
    w.baker.preview.on = true;
    w.update_bake_preview(&ctx);
    w.finish_bake_preview(&ctx);
    w.baker.skip_stacked = !w.baker.skip_stacked;
    w.update_bake_preview(&ctx);
    assert!(w.bake_preview_running());
}

#[test]
fn a_godot_material_with_green_down_is_warned_about() {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let warned = |harness: &mut egui_kittest::Harness<'static, tests::State>| {
        harness.run();
        harness.run();
        harness
            .query_all_by_label_contains("Godot reads normal maps green up")
            .next()
            .is_some()
    };
    {
        let w = &mut harness.state_mut().workbench;
        w.baker.open = true;
        w.baker.godot = true;
    }
    assert!(!warned(&mut harness));
    harness.state_mut().workbench.baker.green_down = true;
    assert!(warned(&mut harness));
}
