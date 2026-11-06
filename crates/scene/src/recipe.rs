//! Unwrap recipes kept in the project ([decision 0246](../../../docs/decisions/0246-unwrap-recipes.md)):
//! named Auto-unwrap settings, shared with whoever opens the project.
//!
//! The scene doesn't know what the settings mean (they belong to the
//! interface's UV tools); it keeps each recipe's name and its settings as
//! text, checked only for size. Changing the list is a step to undo
//! ([`crate::Command::SetRecipes`]); applying a recipe records ordinary
//! edits.

/// The most recipes a project keeps.
pub const MAX_RECIPES: usize = 64;
/// The longest name a recipe keeps, in bytes.
pub const MAX_RECIPE_NAME: usize = 64;
/// The longest settings text a recipe keeps, in bytes.
pub const MAX_RECIPE_SETTINGS: usize = 16 * 1024;

/// One named set of Auto-unwrap settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnwrapRecipe {
    /// Its name ("Kit walls").
    pub name: String,
    /// The settings, as the interface wrote them.
    pub settings: String,
}

/// Whether `list` can be kept: not too many, each named (and not too
/// long), no two with the same name, settings within size.
pub fn recipes_are_valid(list: &[UnwrapRecipe]) -> bool {
    list.len() <= MAX_RECIPES
        && list.iter().enumerate().all(|(i, r)| {
            !r.name.trim().is_empty()
                && r.name.len() <= MAX_RECIPE_NAME
                && r.settings.len() <= MAX_RECIPE_SETTINGS
                && !list[..i].iter().any(|s| s.name == r.name)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Document};

    fn recipe(name: &str) -> UnwrapRecipe {
        UnwrapRecipe {
            name: name.into(),
            settings: "(padding: 8)".into(),
        }
    }

    #[test]
    fn recipes_change_in_one_undo_step() {
        let mut doc = Document::new();
        doc.execute(Command::SetRecipes(vec![recipe("Walls")]))
            .unwrap();
        assert_eq!(doc.scene().recipes(), &[recipe("Walls")]);
        doc.undo();
        assert!(doc.scene().recipes().is_empty());
        doc.redo();
        assert_eq!(doc.scene().recipes().len(), 1);
    }

    #[test]
    fn the_same_list_records_nothing() {
        let mut doc = Document::new();
        doc.execute(Command::SetRecipes(vec![recipe("A")])).unwrap();
        let steps = doc.undo_steps();
        doc.execute(Command::SetRecipes(vec![recipe("A")])).unwrap();
        assert_eq!(doc.undo_steps(), steps);
    }

    #[test]
    fn bad_lists_are_refused() {
        let mut doc = Document::new();
        let bad = [
            vec![recipe(" ")],
            vec![recipe("A"), recipe("A")],
            vec![recipe(&"x".repeat(MAX_RECIPE_NAME + 1))],
            vec![UnwrapRecipe {
                name: "Big".into(),
                settings: "x".repeat(MAX_RECIPE_SETTINGS + 1),
            }],
            (0..=MAX_RECIPES).map(|i| recipe(&i.to_string())).collect(),
        ];
        for list in bad {
            assert!(!recipes_are_valid(&list));
            assert!(doc.execute(Command::SetRecipes(list)).is_err());
        }
        assert!(doc.scene().recipes().is_empty());
    }
}
