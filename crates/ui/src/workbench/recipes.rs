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
        match kept {
            Kept::Project => {
                let Ok(text) = ron::to_string(&settings) else {
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

    /// The settings of the recipe `name`, if it can be read.
    fn recipe_settings(&self, kept: Kept, name: &str) -> Option<Result<UvSettings, String>> {
        match kept {
            Kept::Project => {
                let recipe = self
                    .document
                    .scene()
                    .recipes()
                    .iter()
                    .find(|r| r.name == name)?;
                Some(ron::from_str(&recipe.settings).map_err(|e| e.to_string()))
            }
            Kept::App => self
                .saved_recipes
                .iter()
                .find(|r| r.name == name)
                .map(|r| Ok(r.settings)),
        }
    }

    /// Auto-unwraps every selected object with the recipe `name`'s
    /// settings; the UV settings are as they were afterwards.
    pub(crate) fn apply_recipe(&mut self, kept: Kept, name: &str) {
        let settings = match self.recipe_settings(kept, name) {
            Some(Ok(settings)) => settings,
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
        self.run_auto_unwrap();
        self.uv_settings = own;
        if self.auto_report.as_ref().is_some_and(|r| r.open) {
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
