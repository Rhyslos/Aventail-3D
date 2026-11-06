//! Version 91 of the project file ([decision 0246](../../../../docs/decisions/0246-unwrap-recipes.md)):
//! version 90's, with the project's unwrap recipes
//! ([`super::v43::ProjectV43::recipes`], stored as [`RecipeV91`]: a name
//! and its settings as text). Files before version 91 hold none.

use super::v1::Problem;
use crate::limits::Bounded;
use scene::{Document, UnwrapRecipe};
use serde::{Deserialize, Serialize};
use words::tr;

/// A recipe as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct RecipeV91 {
    /// Its name.
    pub(crate) name: String,
    /// Its settings, as the interface wrote them.
    pub(crate) settings: String,
}

/// The recipes to store.
pub(crate) fn recipes_of(document: &Document) -> Bounded<RecipeV91> {
    document
        .scene()
        .recipes()
        .iter()
        .map(|r| RecipeV91 {
            name: r.name.clone(),
            settings: r.settings.clone(),
        })
        .collect()
}

/// `document` with the stored recipes, checked: too many, a name missing,
/// too long or given twice, or too much in one, is damage.
pub(crate) fn with_recipes(
    document: Document,
    stored: Bounded<RecipeV91>,
) -> Result<Document, Problem> {
    let recipes: Vec<UnwrapRecipe> = stored
        .into_iter()
        .map(|r| UnwrapRecipe {
            name: r.name,
            settings: r.settings,
        })
        .collect();
    if !scene::recipe::recipes_are_valid(&recipes) {
        return Err(tr("its unwrap recipes are damaged").into());
    }
    Ok(document.with_recipes(recipes))
}
