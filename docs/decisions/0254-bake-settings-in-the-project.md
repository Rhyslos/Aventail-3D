# 0254. The Bake window's settings in the project

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

The Bake window's settings (which maps, what to bake from, matched by name, the cage, showing it, leaving out stacked copies, the Godot material's roughness and metal, green-down normals) lived only in the window: closing the app forgot them, so every session began by setting a kit's bake up again. A test pass noted it; the baker fixes left it for later.

## Options

- **With the project** (chosen): a kit's bake belongs to its project, as its texture size and trim sheet do; whoever opens the project bakes it the same way.
- With the app's settings, like the UV settings: the same for every project, which suits a person's habits but not the high-detail object chosen, which belongs to one project.
- As a step to undo: every tick would be an undo step mixed in with the modelling, and Ctrl + Z after a bake would untick a map. The window's settings aren't changes to the model.

## Decision

- **The scene keeps them as text** it doesn't read (`Scene::bake_settings`, at most `MAX_BAKE_SETTINGS`, 4 KiB), as it keeps unwrap recipes' settings (0246): the interface writes and reads it (`workbench::bake_saving::BakeSettings`, RON, every field with its standard value when missing; maps by their file names' ends, so maps added later don't spoil older texts; numbers out of range brought within it). Empty for the standard settings, so a project nobody baked in keeps nothing.
- **Not a step to undo:** `Document::set_bake_settings` keeps the text and counts it as a change to save (the revision goes up), without an undo step. The window writes it each frame its settings differ from the project's, but not while a mouse button is down, so dragging the cage slider is one change.
- **Opening a project, or a new one,** gives the window the project's settings (`load_bake_settings`); a project without them gives the standard ones.
- **File format 94** stores the text (`v94`, a field of `ProjectV43`); older files hold none. Too long a text is damage.
- The live preview's on/off isn't kept: it changes how the model looks, and starts off.

## Consequences

- The high-detail object is kept by its id. If it is deleted, the window shows "The object itself" as before, and the id stays kept until another choice is made.
- Changing a bake setting marks the project unsaved, as other project settings do.
