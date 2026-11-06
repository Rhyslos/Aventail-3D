//! Tests for lightmap UVs in the window (decision 0242).

use super::modelling_tests::note;
use super::seams_tests::{add, shape, steps};
use super::tests::{fresh, uv_group};
use super::*;
use egui_kittest::kittest::Queryable;

#[test]
fn a_lightmap_set_is_made_and_the_painted_uvs_stay() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let own = mesh::uv_sets::values(&shape(&harness), mesh::uv_sets::STANDARD_NAME);
    let before = steps(&harness);
    harness.state_mut().workbench.run_make_lightmap();
    harness.run();
    assert_eq!(steps(&harness), before + 1);
    let m = shape(&harness);
    assert!(mesh::uv_sets::names(&m).contains(&"lightmap".to_string()));
    assert_eq!(mesh::uv_sets::values(&m, mesh::uv_sets::STANDARD_NAME), own);
    assert_eq!(mesh::uv_sets::second(&m).as_deref(), Some("lightmap"));
    assert!(note(&harness).contains("UV2"), "{}", note(&harness));
}

#[test]
fn nothing_selected_says_so() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.state_mut().workbench.run_make_lightmap();
    assert!(
        note(&harness).contains("select the objects"),
        "{}",
        note(&harness)
    );
}

#[test]
fn the_uv_menu_offers_lightmap_uvs() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    uv_group(&mut harness, lightmap::LIGHTMAP_LABEL);
    assert!(
        harness
            .query_by_label(lightmap::MAKE_LIGHTMAP_LABEL)
            .is_some()
    );
}

/// Test pass (fix 6): with eight UV sets and none called lightmap, there
/// is no room for one. Nothing is recorded and the note doesn't claim
/// one was made.
#[test]
fn lightmap_uvs_with_eight_sets_already_do_not_claim_success() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    for k in 1..mesh::uv_sets::MAX_SETS {
        harness
            .state_mut()
            .workbench
            .run_uv_set(mesh::uv_sets::SetChange::Add(format!("s{k}")));
        harness.run();
    }
    assert_eq!(
        mesh::uv_sets::names(&shape(&harness)).len(),
        mesh::uv_sets::MAX_SETS
    );
    let before = steps(&harness);
    harness.state_mut().workbench.run_make_lightmap();
    harness.run();
    assert_eq!(steps(&harness), before, "no step that changes nothing");
    assert!(!mesh::uv_sets::names(&shape(&harness)).contains(&"lightmap".to_string()));
    let said = note(&harness);
    assert!(!said.contains("a UV set called lightmap"), "{said}");
    assert!(
        said.contains("nothing changed") && said.contains("8 UV sets"),
        "{said}"
    );
}

/// With two objects, one full: the other gets its lightmap, and the note
/// names the one left out. Auto-unwrap's step says the same.
#[test]
fn a_full_object_is_left_out_and_named() {
    let mut harness = fresh();
    let full = add(&mut harness, scene::Primitive::Cube(Default::default()));
    for k in 1..mesh::uv_sets::MAX_SETS {
        harness
            .state_mut()
            .workbench
            .run_uv_set(mesh::uv_sets::SetChange::Add(format!("s{k}")));
        harness.run();
    }
    let room = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([full, room]);
    let before = steps(&harness);
    harness.state_mut().workbench.run_make_lightmap();
    harness.run();
    assert_eq!(steps(&harness), before + 1);
    let names = |id| {
        let doc = harness.state().workbench.document();
        mesh::uv_sets::names(&doc.scene().evaluated(id).unwrap().mesh)
    };
    assert!(names(room).contains(&"lightmap".to_string()));
    assert!(!names(full).contains(&"lightmap".to_string()));
    let said = note(&harness);
    assert!(said.contains("a UV set called lightmap"), "{said}");
    assert!(said.contains("1 object already has 8 UV sets"), "{said}");
}
