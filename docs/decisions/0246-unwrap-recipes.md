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
