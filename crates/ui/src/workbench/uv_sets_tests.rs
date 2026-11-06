//! Tests for UV sets in the window (decision 0241).

use super::modelling_tests::{H, note};
use super::seams_tests::{add, shape, steps};
use super::tests::{fresh, uv_group};
use egui_kittest::kittest::Queryable;
use mesh::uv_sets::{self, SetChange};

fn cube(harness: &mut H) -> scene::ObjectId {
    add(harness, scene::Primitive::Cube(Default::default()))
}

#[test]
fn sets_are_added_switched_and_undone_one_step_each() {
    let mut harness = fresh();
    cube(&mut harness);
    let before = steps(&harness);
    harness
        .state_mut()
        .workbench
        .run_uv_set(SetChange::Add("detail".into()));
    harness.run();
    assert_eq!(uv_sets::names(&shape(&harness)).len(), 2);
    harness
        .state_mut()
        .workbench
        .run_uv_set(SetChange::MakeActive("detail".into()));
    harness.run();
    assert_eq!(uv_sets::active_name(&shape(&harness)), "detail");
    assert_eq!(steps(&harness), before + 2);
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(
        uv_sets::active_name(&shape(&harness)),
        uv_sets::STANDARD_NAME
    );
}

#[test]
fn a_bad_name_or_nothing_selected_says_why() {
    let mut harness = fresh();
    cube(&mut harness);
    harness
        .state_mut()
        .workbench
        .run_uv_set(SetChange::Add("a:b".into()));
    assert!(note(&harness).contains("give a name"), "{}", note(&harness));
}

#[test]
fn the_uv_menu_lists_the_sets() {
    let mut harness = fresh();
    cube(&mut harness);
    uv_group(&mut harness, super::uv_sets::UV_SETS_LABEL);
    assert!(harness.query_by_label(uv_sets::STANDARD_NAME).is_some());
    assert!(harness.query_by_label("Add a set with this name").is_some());
}
