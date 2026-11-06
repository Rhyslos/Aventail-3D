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
    harness
        .state_mut()
        .workbench
        .uv_settings
        .auto_steps
        .lightmap = false;
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
    let (read, density) = super::recipes::read_recipe(&recipes[0].settings).unwrap();
    assert_eq!(density, Some(doc.scene().texture().density));
    assert_eq!(read.padding, 12);
    assert!(!read.auto_steps.lightmap);
    let back = fileio::project::from_bytes(&fileio::project::to_bytes(doc).unwrap()).unwrap();
    assert_eq!(back.scene().recipes(), recipes);
}

#[test]
fn keeping_again_under_the_same_name_replaces_it() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .keep_recipe(Kept::Project, "A");
    harness.state_mut().workbench.uv_settings.padding = 20;
    harness
        .state_mut()
        .workbench
        .keep_recipe(Kept::Project, "A");
    let recipes = harness
        .state()
        .workbench
        .document()
        .scene()
        .recipes()
        .to_vec();
    assert_eq!(recipes.len(), 1);
    assert!(recipes[0].settings.contains("20"));
    harness.state_mut().workbench.keep_recipe(Kept::App, "A");
    harness.state_mut().workbench.keep_recipe(Kept::App, "A");
    assert_eq!(harness.state().workbench.saved_recipes().len(), 1);
}

#[test]
fn a_recipe_needs_a_name() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .keep_recipe(Kept::Project, "   ");
    harness.state_mut().workbench.keep_recipe(Kept::App, "");
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .recipes()
            .is_empty()
    );
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
    harness
        .state_mut()
        .workbench
        .apply_recipe(Kept::Project, "Broken");
    assert_eq!(steps(&harness), before);
    assert!(
        note(&harness).contains("can't be read"),
        "{}",
        note(&harness)
    );
    harness
        .state_mut()
        .workbench
        .apply_recipe(Kept::App, "Nowhere");
    assert!(note(&harness).contains("no recipe"), "{}", note(&harness));
}

#[test]
fn deleting_recipes() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .keep_recipe(Kept::Project, "A");
    harness.state_mut().workbench.keep_recipe(Kept::App, "B");
    harness
        .state_mut()
        .workbench
        .delete_recipe(Kept::Project, "A");
    harness.state_mut().workbench.delete_recipe(Kept::App, "B");
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .recipes()
            .is_empty()
    );
    assert!(harness.state().workbench.saved_recipes().is_empty());
    harness.state_mut().workbench.document_mut().undo();
    assert_eq!(
        harness.state().workbench.document().scene().recipes().len(),
        1
    );
}

#[test]
fn the_app_keeps_its_recipes_between_sessions() {
    let recipes = vec![AppRecipe {
        name: "Props".into(),
        settings: UvSettings::default(),
        density: Some(256.0),
    }];
    let text = ron::to_string(&recipes).unwrap();
    let mut read: Vec<AppRecipe> = ron::from_str(&text).unwrap();
    read.push(AppRecipe {
        name: " ".into(),
        settings: UvSettings::default(),
        density: None,
    });
    let mut harness = fresh();
    harness.state_mut().workbench.set_saved_recipes(read);
    // The unnamed one is left out.
    assert_eq!(
        harness.state().workbench.saved_recipes(),
        recipes.as_slice()
    );
}

#[test]
fn the_menu_lists_and_keeps_recipes() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .keep_recipe(Kept::App, "Props");
    uv_group(&mut harness, AUTO_UNWRAP_LABEL);
    assert!(harness.query_by_label("Props").is_some());
    assert!(harness.query_by_label(KEEP_IN_PROJECT_LABEL).is_some());
    assert!(harness.query_by_label(KEEP_IN_APP_LABEL).is_some());
}

/// Test pass (fix 6): with nothing selected, applying a recipe does
/// nothing and says so, even with an earlier Auto-unwrap's window open.
#[test]
fn applying_a_recipe_with_nothing_selected_does_not_claim_it_ran() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.run_auto_unwrap();
    harness.run();
    harness
        .state_mut()
        .workbench
        .keep_recipe(Kept::Project, "Walls");
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([]);
    harness.run();
    let before = steps(&harness);
    harness
        .state_mut()
        .workbench
        .apply_recipe(Kept::Project, "Walls");
    harness.run();
    assert_eq!(steps(&harness), before);
    let said = note(&harness);
    assert!(!said.contains("Applied"), "{said}");
    assert!(said.contains("select the objects first"), "{said}");
}

/// Test pass (fix 6): a recipe gives the texel density the project had
/// when it was kept, not the one it has when applied, in the project and
/// in the app; the project's density stays as it is.
#[test]
fn a_recipe_keeps_the_texel_density_it_was_made_with() {
    for kept in [Kept::App, Kept::Project] {
        let mut harness = fresh();
        let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
        let on = &mut harness.state_mut().workbench.uv_settings.auto_steps;
        on.pack = false;
        on.lightmap = false;
        harness.state_mut().workbench.keep_recipe(kept, "Dense");
        let mut texture = harness.state().workbench.document().scene().texture();
        let made_with = texture.uv_per_unit();
        texture.density *= 4.0;
        harness
            .state_mut()
            .workbench
            .document_mut()
            .execute(scene::Command::SetTexture(texture))
            .unwrap();
        harness
            .state_mut()
            .workbench
            .document_mut()
            .set_selection([id]);
        harness.state_mut().workbench.apply_recipe(kept, "Dense");
        harness.run();
        let m = shape(&harness);
        let faces: Vec<_> = m.faces().map(|(f, _)| f).collect();
        let density = uv::density::density(&m, &faces).unwrap();
        assert!(
            (density - made_with).abs() < made_with * 0.05,
            "{kept:?}: the recipe gave {density}, it was kept at {made_with}"
        );
        let now = harness.state().workbench.document().scene().texture();
        assert_eq!(now, texture, "the project's density stays");
    }
}

/// A project recipe kept before the density was (the UV settings alone)
/// still reads, without a density: it uses the project's, as it did.
#[test]
fn an_old_project_recipe_still_reads_without_a_density() {
    let old = ron::to_string(&UvSettings {
        padding: 30,
        ..UvSettings::default()
    })
    .unwrap();
    let (settings, density) = super::recipes::read_recipe(&old).unwrap();
    assert_eq!(settings.padding, 30);
    assert_eq!(density, None);
    assert_eq!(
        super::recipes::read_recipe("()").unwrap(),
        (UvSettings::default(), None)
    );
    // The app's recipes kept before have no density field either.
    let app: AppRecipe = ron::from_str("(name: \"Old\", settings: (padding: 9))").unwrap();
    assert_eq!(app.density, None);
    assert_eq!(app.settings.padding, 9);
}

/// Round trip: a recipe's text, with its density, reads back exactly; and
/// through a saved project.
#[test]
fn a_recipe_with_its_density_survives_a_round_trip() {
    let mut harness = fresh();
    harness.state_mut().workbench.uv_settings.padding = 14;
    let mut texture = harness.state().workbench.document().scene().texture();
    texture.density = 333.0;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTexture(texture))
        .unwrap();
    harness
        .state_mut()
        .workbench
        .keep_recipe(Kept::Project, "Kit");
    let doc = harness.state().workbench.document();
    let back = fileio::project::from_bytes(&fileio::project::to_bytes(doc).unwrap()).unwrap();
    let (settings, density) =
        super::recipes::read_recipe(&back.scene().recipes()[0].settings).unwrap();
    assert_eq!(settings.padding, 14);
    assert_eq!(density, Some(333.0));
}

/// Fuzz: any text, damaged or random, read as a recipe never panics; an
/// unusable density (not a number, 0, too big) is never given.
#[test]
fn reading_any_recipe_text_never_panics() {
    let mut seed: u64 = 0x5eed_0f7e_c19e;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let good = ron::to_string(&super::recipes::RecipeText {
        settings: UvSettings::default(),
        density: Some(512.0),
    })
    .unwrap();
    let pieces = [
        "(",
        ")",
        "settings:",
        "density:",
        "Some(",
        "None",
        "-1.0",
        "NaN",
        "inf",
        "1e40",
        ",",
        "\"",
        "padding: 8",
        "auto_steps: (",
        "true",
        "0",
    ];
    for _ in 0..2000 {
        let mut text = String::new();
        if next() % 2 == 0 {
            // A good text, cut or with a piece put in.
            let cut = (next() as usize) % (good.len() + 1);
            text.push_str(good.get(..cut).unwrap_or(&good));
        }
        for _ in 0..(next() % 8) {
            text.push_str(pieces[(next() as usize) % pieces.len()]);
        }
        let _ = super::recipes::read_recipe(&text);
    }
    for bad in [f32::NAN, f32::INFINITY, 0.0, -5.0, 1e9] {
        let text = format!("(settings: (), density: Some({bad:?}))");
        if let Ok((_, density)) = super::recipes::read_recipe(&text) {
            assert!(super::recipes::usable_density(density).is_none(), "{bad}");
        }
    }
}
