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

/// Test pass (fix 6): deleting the only set, or adding a ninth, changes
/// nothing, records no step and says why rather than claiming it was
/// done.
#[test]
fn the_last_uv_set_cannot_be_deleted_and_the_ninth_cannot_be_added() {
    let mut harness = fresh();
    cube(&mut harness);
    let before = steps(&harness);
    harness
        .state_mut()
        .workbench
        .run_uv_set(SetChange::Delete("UVMap".into()));
    harness.run();
    assert_eq!(uv_sets::names(&shape(&harness)).len(), 1);
    assert_eq!(steps(&harness), before);
    let said = note(&harness);
    assert!(!said.starts_with("Delete the UV set UVMap."), "{said}");
    assert!(
        said.contains("nothing changed") && said.contains("at least one"),
        "{said}"
    );
    for k in 1..uv_sets::MAX_SETS {
        harness
            .state_mut()
            .workbench
            .run_uv_set(SetChange::Add(format!("s{k}")));
        harness.run();
    }
    let full = steps(&harness);
    harness
        .state_mut()
        .workbench
        .run_uv_set(SetChange::Add("ninth".into()));
    harness.run();
    assert_eq!(uv_sets::names(&shape(&harness)).len(), uv_sets::MAX_SETS);
    assert_eq!(steps(&harness), full);
    let said = note(&harness);
    assert!(
        said.contains("nothing changed") && said.contains("at most 8"),
        "{said}"
    );
    // A name that's taken, and switching to the set worked on.
    harness.state_mut().workbench.run_uv_set(SetChange::Rename {
        from: "UVMap".into(),
        to: "s1".into(),
    });
    assert!(
        note(&harness).contains("already a set called s1"),
        "{}",
        note(&harness)
    );
    harness
        .state_mut()
        .workbench
        .run_uv_set(SetChange::MakeActive("UVMap".into()));
    assert!(
        note(&harness).contains("already the set worked on"),
        "{}",
        note(&harness)
    );
    assert_eq!(steps(&harness), full);
}

/// With two objects selected and only one having the set, switching to
/// it switches that one, and the note says the other was left as it was.
#[test]
fn switching_set_with_two_objects_says_which_changed() {
    let mut harness = fresh();
    let a = cube(&mut harness);
    harness
        .state_mut()
        .workbench
        .run_uv_set(SetChange::Add("extra".into()));
    harness.run();
    let b = cube(&mut harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([a, b]);
    harness
        .state_mut()
        .workbench
        .run_uv_set(SetChange::MakeActive("extra".into()));
    harness.run();
    let names = |id| {
        let doc = harness.state().workbench.document();
        uv_sets::names(&doc.scene().evaluated(id).unwrap().mesh)
    };
    assert_eq!(names(a)[0], "extra");
    assert_eq!(names(b)[0], "UVMap");
    let said = note(&harness);
    assert!(said.contains("on 1 of the 2 objects"), "{said}");
    assert!(
        said.contains("the other is as it was, as there is no set called extra"),
        "{said}"
    );
}
