//! Tests for unwrap recipes (decision 0246).

use super::auto_unwrap::{AUTO_UNWRAP_LABEL, AutoStep};
use super::modelling_tests::note;
use super::recipes::{AppRecipe, KEEP_IN_APP_LABEL, KEEP_IN_PROJECT_LABEL, Kept};
use super::seams_tests::{add, shape, steps};
use super::tests::{fresh, uv_group};
use super::*;
use egui_kittest::kittest::Queryable;

#[test]
fn a_project_recipe_is_kept_in_one_undo_step_and_saved_with_the_project() {
    let mut harness = fresh();
    harness.state_mut().workbench.uv_settings.padding = 12;
    harness.state_mut().workbench.uv_settings.auto_steps.lightmap = false;
    let before = steps(&harness);
    harness
        .state_mut()
        .workbench
        .keep_recipe(Kept::Project, "  Kit walls ");
    assert_eq!(steps(&harness), before + 1);
    let doc = harness.state().workbench.document();
    let recipes = doc.scene().recipes();
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].name, "Kit walls");
    let read: UvSettings = ron::from_str(&recipes[0].settings).unwrap();
    assert_eq!(read.padding, 12);
    assert!(!read.auto_steps.lightmap);
    let back = fileio::project::from_bytes(&fileio::project::to_bytes(doc).unwrap()).unwrap();
    assert_eq!(back.scene().recipes(), recipes);
}

#[test]
fn keeping_again_under_the_same_name_replaces_it() {
    let mut harness = fresh();
    harness.state_mut().workbench.keep_recipe(Kept::Project, "A");
    harness.state_mut().workbench.uv_settings.padding = 20;
    harness.state_mut().workbench.keep_recipe(Kept::Project, "A");
    let recipes = harness.state().workbench.document().scene().recipes().to_vec();
    assert_eq!(recipes.len(), 1);
    assert!(recipes[0].settings.contains("20"));
    harness.state_mut().workbench.keep_recipe(Kept::App, "A");
    harness.state_mut().workbench.keep_recipe(Kept::App, "A");
    assert_eq!(harness.state().workbench.saved_recipes().len(), 1);
}

#[test]
fn a_recipe_needs_a_name() {
    let mut harness = fresh();
    harness.state_mut().workbench.keep_recipe(Kept::Project, "   ");
    harness.state_mut().workbench.keep_recipe(Kept::App, "");
    assert!(harness.state().workbench.document().scene().recipes().is_empty());
    assert!(harness.state().workbench.saved_recipes().is_empty());
    assert!(note(&harness).contains("name"), "{}", note(&harness));
}

#[test]
fn applying_a_recipe_runs_its_steps_and_leaves_the_settings() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    {
        let w = &mut harness.state_mut().workbench;
        w.uv_settings.auto_steps.lightmap = false;
        w.uv_settings.auto_steps.straighten = false;
        w.keep_recipe(Kept::App, "No lightmap");
        w.uv_settings.auto_steps = Default::default();
    }
    let own = harness.state().workbench.uv_settings;
    harness
        .state_mut()
        .workbench
        .apply_recipe(Kept::App, "No lightmap");
    let w = &harness.state().workbench;
    assert_eq!(w.uv_settings, own);
    let report = w.auto_report.clone().unwrap();
    assert_eq!(report.steps.len(), 4);
    assert!(report.steps.iter().all(|s| s.step != AutoStep::Lightmap));
    assert!(!mesh::uv_sets::names(&shape(&harness)).contains(&"lightmap".to_string()));
    assert!(note(&harness).contains("No lightmap"), "{}", note(&harness));
}

#[test]
fn a_damaged_or_missing_recipe_says_so() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetRecipes(vec![scene::UnwrapRecipe {
            name: "Broken".into(),
            settings: "(padding: \"lots\")".into(),
        }]))
        .unwrap();
    let before = steps(&harness);
    harness.state_mut().workbench.apply_recipe(Kept::Project, "Broken");
    assert_eq!(steps(&harness), before);
    assert!(note(&harness).contains("can't be read"), "{}", note(&harness));
    harness.state_mut().workbench.apply_recipe(Kept::App, "Nowhere");
    assert!(note(&harness).contains("no recipe"), "{}", note(&harness));
}

#[test]
fn deleting_recipes() {
    let mut harness = fresh();
    harness.state_mut().workbench.keep_recipe(Kept::Project, "A");
    harness.state_mut().workbench.keep_recipe(Kept::App, "B");
    harness.state_mut().workbench.delete_recipe(Kept::Project, "A");
    harness.state_mut().workbench.delete_recipe(Kept::App, "B");
    assert!(harness.state().workbench.document().scene().recipes().is_empty());
    assert!(harness.state().workbench.saved_recipes().is_empty());
    harness.state_mut().workbench.document_mut().undo();
    assert_eq!(harness.state().workbench.document().scene().recipes().len(), 1);
}

#[test]
fn the_app_keeps_its_recipes_between_sessions() {
    let recipes = vec![AppRecipe {
        name: "Props".into(),
        settings: UvSettings::default(),
    }];
    let text = ron::to_string(&recipes).unwrap();
    let mut read: Vec<AppRecipe> = ron::from_str(&text).unwrap();
    read.push(AppRecipe {
        name: " ".into(),
        settings: UvSettings::default(),
    });
    let mut harness = fresh();
    harness.state_mut().workbench.set_saved_recipes(read);
    // The unnamed one is left out.
    assert_eq!(harness.state().workbench.saved_recipes(), recipes.as_slice());
}

#[test]
fn the_menu_lists_and_keeps_recipes() {
    let mut harness = fresh();
    harness.state_mut().workbench.keep_recipe(Kept::App, "Props");
    uv_group(&mut harness, AUTO_UNWRAP_LABEL);
    assert!(harness.query_by_label("Props").is_some());
    assert!(harness.query_by_label(KEEP_IN_PROJECT_LABEL).is_some());
    assert!(harness.query_by_label(KEEP_IN_APP_LABEL).is_some());
}
