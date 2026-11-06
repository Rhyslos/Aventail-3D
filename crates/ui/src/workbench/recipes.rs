//! Unwrap recipes ([decision 0246](../../../../docs/decisions/0246-unwrap-recipes.md)):
//! Auto-unwrap's steps with every UV setting, saved under a name, in the
//! project (to share with whoever opens it) or in the app (for every
//! project), and applied to every selected object in one click.

use super::Workbench;
use super::auto_unwrap::AutoStep;
use super::unwrapping::UvSettings;
use crate::text::tr;
use egui::Ui;
use scene::{Command, UnwrapRecipe};
use serde::{Deserialize, Serialize};

/// A recipe kept in the app's settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppRecipe {
    /// Its name.
    pub name: String,
    /// The UV settings, Auto-unwrap's steps among them.
    pub settings: UvSettings,
    /// The project's texel density when it was kept (pixels per metre),
    /// which its Set texel density step gives. `None` in recipes kept
    /// before it was kept: those use the project's density.
    #[serde(default)]
    pub density: Option<f32>,
}

/// A project recipe's settings text: the UV settings and the texel
/// density (pixels per metre) it was kept with. Recipes kept before the
/// density was kept hold the UV settings alone ([`read_recipe`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct RecipeText {
    /// The UV settings.
    pub(crate) settings: UvSettings,
    /// The texel density.
    #[serde(default)]
    pub(crate) density: Option<f32>,
}

/// Reads a project recipe's settings text: as [`RecipeText`], or, from a
/// recipe kept before the density was, the UV settings alone (no
/// density). An error says why it can't be read.
pub(crate) fn read_recipe(text: &str) -> Result<(UvSettings, Option<f32>), String> {
    if let Ok(recipe) = ron::from_str::<RecipeText>(text) {
        return Ok((recipe.settings, recipe.density));
    }
    ron::from_str::<UvSettings>(text)
        .map(|settings| (settings, None))
        .map_err(|e| e.to_string())
}

/// A density read from a recipe, if it is one a project can have.
pub(crate) fn usable_density(density: Option<f32>) -> Option<f32> {
    density.filter(|d| d.is_finite() && *d > 0.0 && *d <= scene::MAX_DENSITY)
}

/// The most recipes the app keeps.
pub(crate) const MAX_APP_RECIPES: usize = scene::MAX_RECIPES;

/// The menu's parts.
pub(crate) const RECIPES_LABEL: &str = crate::text!("Recipes");
pub(crate) const KEEP_IN_PROJECT_LABEL: &str = crate::text!("Keep in the project");
pub(crate) const KEEP_IN_APP_LABEL: &str = crate::text!("Keep in the app");
pub(crate) const RECIPE_NAME_HINT: &str = crate::text!("Recipe name");

/// Where a recipe is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kept {
    Project,
    App,
}

impl Workbench {
    /// The recipes kept in the app, for saving with the app's settings.
    pub fn saved_recipes(&self) -> &[AppRecipe] {
        &self.saved_recipes
    }

    /// The workbench with the app's recipes (from the app's settings).
    #[must_use]
    pub fn with_saved_recipes(mut self, recipes: Vec<AppRecipe>) -> Self {
        self.set_saved_recipes(recipes);
        self
    }

    /// Gives the app's recipes, leaving out unnamed ones and any past
    /// the most kept.
    pub(crate) fn set_saved_recipes(&mut self, mut recipes: Vec<AppRecipe>) {
        recipes.retain(|r| !r.name.trim().is_empty());
        recipes.truncate(MAX_APP_RECIPES);
        self.saved_recipes = recipes;
    }

    /// The recipes part of UV → Auto-unwrap.
    pub(super) fn recipe_entries(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new(tr(RECIPES_LABEL)).strong());
        let project: Vec<String> = self
            .document
            .scene()
            .recipes()
            .iter()
            .map(|r| r.name.clone())
            .collect();
        let app: Vec<String> = self.saved_recipes.iter().map(|r| r.name.clone()).collect();
        if project.is_empty() && app.is_empty() {
            ui.weak(tr(
                "None yet: choose the steps and settings, then keep them below",
            ));
        }
        let mut apply = None;
        let mut delete = None;
        for (kept, names, place) in [
            (Kept::Project, &project, tr("in this project")),
            (Kept::App, &app, tr("in the app")),
        ] {
            for name in names {
                ui.horizontal(|ui| {
                    if ui
                        .button(name)
                        .on_hover_text(crate::trf!(
                            "Auto-unwrap the selected objects with this recipe ({})",
                            place
                        ))
                        .clicked()
                    {
                        apply = Some((kept, name.clone()));
                    }
                    if ui
                        .small_button("×")
                        .on_hover_text(crate::trf!("Delete the recipe {}", name))
                        .clicked()
                    {
                        delete = Some((kept, name.clone()));
                    }
                });
            }
        }
        ui.separator();
        ui.add(
            egui::TextEdit::singleline(&mut self.recipe_name)
                .hint_text(tr(RECIPE_NAME_HINT))
                .char_limit(scene::MAX_RECIPE_NAME),
        );
        ui.horizontal(|ui| {
            if ui
                .button(tr(KEEP_IN_PROJECT_LABEL))
                .on_hover_text(tr(
                    "Keep the steps above and every UV setting under this name in the project: \
                     saved with it, so whoever opens it can use them",
                ))
                .clicked()
            {
                let name = self.recipe_name.clone();
                self.keep_recipe(Kept::Project, &name);
            }
            if ui
                .button(tr(KEEP_IN_APP_LABEL))
                .on_hover_text(tr(
                    "Keep the steps above and every UV setting under this name in the app, for \
                     every project",
                ))
                .clicked()
            {
                let name = self.recipe_name.clone();
                self.keep_recipe(Kept::App, &name);
            }
        });
        if let Some((kept, name)) = apply {
            self.apply_recipe(kept, &name);
            ui.close();
        }
        if let Some((kept, name)) = delete {
            self.delete_recipe(kept, &name);
        }
    }

    /// Keeps the UV settings as it now stands under `name`, replacing a
    /// recipe of that name.
    pub(crate) fn keep_recipe(&mut self, kept: Kept, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            self.note = Some(tr("Give the recipe a name first.").to_string());
            return;
        }
        let settings = self.uv_settings;
        let density = Some(self.document.scene().texture().density);
        match kept {
            Kept::Project => {
                let Ok(text) = ron::to_string(&RecipeText { settings, density }) else {
                    self.note = Some(tr("Couldn't write the recipe.").to_string());
                    return;
                };
                let mut list = self.document.scene().recipes().to_vec();
                let recipe = UnwrapRecipe {
                    name: name.to_string(),
                    settings: text,
                };
                match list.iter_mut().find(|r| r.name == name) {
                    Some(old) => *old = recipe,
                    None => list.push(recipe),
                }
                if let Err(why) = self.document.execute(Command::SetRecipes(list)) {
                    self.note = Some(crate::trf!("Couldn't keep the recipe: {}.", why));
                    return;
                }
                self.note = Some(crate::trf!(
                    "The recipe {} is kept in the project, saved with it. Ctrl + Z takes it back.",
                    name
                ));
            }
            Kept::App => {
                let recipe = AppRecipe {
                    name: name.to_string(),
                    settings,
                    density,
                };
                let full = self.saved_recipes.len() >= MAX_APP_RECIPES;
                match self.saved_recipes.iter_mut().find(|r| r.name == name) {
                    Some(old) => *old = recipe,
                    None if full => {
                        self.note = Some(
                            tr("The app keeps no more recipes: delete one first.").to_string(),
                        );
                        return;
                    }
                    None => self.saved_recipes.push(recipe),
                }
                self.note = Some(crate::trf!(
                    "The recipe {} is kept in the app, for every project.",
                    name
                ));
            }
        }
        self.recipe_name.clear();
    }

    /// The settings and texel density of the recipe `name`, if it can be
    /// read.
    fn recipe_settings(
        &self,
        kept: Kept,
        name: &str,
    ) -> Option<Result<(UvSettings, Option<f32>), String>> {
        match kept {
            Kept::Project => {
                let recipe = self
                    .document
                    .scene()
                    .recipes()
                    .iter()
                    .find(|r| r.name == name)?;
                Some(read_recipe(&recipe.settings))
            }
            Kept::App => self
                .saved_recipes
                .iter()
                .find(|r| r.name == name)
                .map(|r| Ok((r.settings, r.density))),
        }
    }

    /// Auto-unwraps every selected object with the recipe `name`'s
    /// settings and texel density; the UV settings (and the project's
    /// density) are as they were afterwards.
    pub(crate) fn apply_recipe(&mut self, kept: Kept, name: &str) {
        let (settings, density) = match self.recipe_settings(kept, name) {
            Some(Ok(kept)) => kept,
            Some(Err(why)) => {
                self.note = Some(crate::trf!("The recipe {} can't be read ({}).", name, why));
                return;
            }
            None => {
                self.note = Some(crate::trf!("There is no recipe called {}.", name));
                return;
            }
        };
        let own = std::mem::replace(&mut self.uv_settings, settings);
        let ran = self.auto_unwrap_at(usable_density(density));
        self.uv_settings = own;
        // When it didn't run, its note says why (nothing selected, say).
        if ran {
            let steps = AutoStep::ALL
                .iter()
                .filter(|s| s.on(&settings.auto_steps))
                .count();
            self.note = Some(crate::trf!(
                "Applied the recipe {}: {} steps, each an edit in the Auto-unwrap layer. Ctrl + Z \
                 takes it all back.",
                name,
                steps
            ));
        }
    }

    /// Deletes the recipe `name`.
    pub(crate) fn delete_recipe(&mut self, kept: Kept, name: &str) {
        match kept {
            Kept::Project => {
                let mut list = self.document.scene().recipes().to_vec();
                list.retain(|r| r.name != name);
                if let Err(why) = self.document.execute(Command::SetRecipes(list)) {
                    self.note = Some(crate::trf!("Couldn't delete the recipe: {}.", why));
                    return;
                }
                self.note = Some(crate::trf!(
                    "Deleted the recipe {}. Ctrl + Z takes it back.",
                    name
                ));
            }
            Kept::App => {
                self.saved_recipes.retain(|r| r.name != name);
                self.note = Some(crate::trf!("Deleted the recipe {} from the app.", name));
            }
        }
    }
}
