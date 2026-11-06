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
