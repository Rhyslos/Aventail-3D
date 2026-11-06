# 0246. Unwrap recipes, kept in the project and in the app

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 20. A modular kit has many pieces that want the same unwrap: the same steps, the same padding, seam rules and density. Auto-unwrap (0245) runs the steps, but its settings are whatever the UV settings are at the moment; switching between "walls" and "props" settings by hand is slow and easy to get wrong, and a team member opening the project can't see what was used.

## Options

- **Where recipes are kept** (asked; the project lead chose **both**, 2026-11-05):
  - **in the project**, saved with it, so whoever opens it gets the same recipes;
  - **in the app's settings**, for every project;
  - both (chosen).

## Decision

- **A recipe is a name and the whole of the UV settings** (`UvSettings`, Auto-unwrap's steps among them, `AutoSteps`), so applying one gives exactly the result the settings gave when it was kept.
- **In the project:** `scene::UnwrapRecipe { name, settings }`, the settings as RON text the interface writes and reads (the scene doesn't know the UV settings); `Command::SetRecipes` changes the list as one undo step, refused when invalid (`recipes_are_valid`: at most 64, a name of up to 64 bytes, no two the same, settings up to 16 KiB). **File format 91** stores them (`v91`, a field of `ProjectV43`); older files hold none.
- **In the app:** `ui::AppRecipe { name, settings }`, kept with the app's settings (`Workbench::saved_recipes`, `with_saved_recipes`); not undoable, like kept brushes.
- **UV → Auto-unwrap → Recipes:** each recipe as a button (click: Auto-unwrap the selected objects with it, the UV settings left as they were), a × to delete; a name box with **Keep in the project** and **Keep in the app** (the same name replaces). No keys.
- A project recipe whose text can't be read (written by a later app, say) says so and changes nothing.

## Consequences

- A recipe holds every UV setting, not only the ones that differ from the standard; settings added later read as their standard value (`#[serde(default)]`).
- Copying a recipe between the project and the app is applying it and keeping it again; a button for that can follow if wanted.

## Amended 2026-11-06, the texel density

The manual said a recipe keeps the texel density, but the density is the project's (UV Editor → Texture), not a UV setting, so applying a recipe used the project's density at the time. A recipe now also keeps the project's density when it was kept, in pixels per metre: a project recipe's text is `RecipeText { settings, density }` (a recipe kept before reads as the UV settings alone, with no density), an app recipe has `density` (absent in older settings). Applying it gives Set texel density that density, with the current project's texture size; the project's own density stays as it was. Kept as pixels per metre rather than texture widths, so a recipe gives the same pixels per metre in a project with another texture size. No new file format: the project keeps a recipe's settings as text.

Applying a recipe with nothing selected said "Applied the recipe…" when an earlier Auto-unwrap's window was still open; it now says only that the objects must be selected.
