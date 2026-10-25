# 0165. The Levels panel: levels made with their own modifier changes, reduced, or by hand

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-25
- **Builds on:** [0155](0155-levels-of-detail.md) (levels of detail as built: `scene::lod`, `Command::SetLods`, `LodCache`, `fileio::export::lods`, `tools/gltf-check/lod_ranges.gd`), [0142](0142-modifier-stack-and-remesh.md) (the modifier stack), [0006](0006-base-plus-operation-stack.md) (tools never change shapes directly), [0163](0163-saving-and-exporting-in-the-background.md) and [0035](0035-heavy-meshes.md) (heavy work on other threads), [0164](0164-interface-fixes-from-the-phase-10-testers.md) (unique widget ids, and the test that fails on a clash), [0091](0091-edit-layers.md) and [0126](0126-rework-and-first-use.md) (how the right column's tabs are made), [0028](0028-ask-about-controls.md)
- **Replaces part of:** [0155](0155-levels-of-detail.md): the Attributes panel's Levels of detail section (now a line pointing to the Levels panel), what a level is made from (each level's real shape, as exported, not a reduction of the viewport's), and the development file format (version 63)

## Problem

On 2026-10-25 the project lead asked for an extra Phase 10 milestone: a **Levels panel** of its own, as a tab beside Attributes, showing the selected object's levels, each with a preview switch, its triangles against a **budget** and the distance it is **Shown from**. They chose all three ways for a level to be made:

1. **With its own modifier changes** (the main one): a level switches any of the object's modifiers off, or gives it simpler settings (Live bevel with 1 segment at Level 1, Subdivide off, Scatter's copies off at Level 2, a sweep's sides 16 → 6, a lower Array count). It stays live: editing the model updates every level.
2. **Automatic reduce**: today's Decimate percentage, kept as an optional last step per level, on top of 1.
3. **By hand**: "Make this level by hand" turns a level into its own editable shape, a copy of what it showed, modelled in the viewport while the level is edited. It no longer follows the model; the panel says when the model has changed since, with "Start again from the model".

No new keys. Left to decide: how a level's changes name a modifier, what a change holds, how a hand-made level is stored and edited (which object the viewport's modes, picking and gizmos use), how editing is entered and left, what is exported, saving, and keeping heavy shapes smooth.

## Options

How a level's changes name a modifier:
1. **A stable id per modifier (chosen).** Modifiers had none (0142 keyed everything by place in the list): moving a modifier would have moved a level's changes onto another. Each modifier now has a `ModifierId`, given when it is added and kept while it stays, saved with the file.
2. By place in the list: breaks on Up and Down.

What a change holds:
1. **On or off, and optionally the whole of the modifier's settings at that level (chosen).** One mechanism for all 17 kinds: the level's settings rows are the modifier's own (`kind_rows`), editing a copy.
2. Each setting changed on its own (a level following the model's bevel width but not its segments): needs a field-by-field description of every kind's settings, a large addition for one feature. Can come later if wanted.

How a hand-made level is stored and edited:
1. **As an ordinary object inside the object with the levels (chosen),** named `Rock LOD2`, which the level points at. Everything that edits an object works on it with nothing added: the modes, picking, the gizmos, the Mesh menu, edit layers, its own modifiers, undo. It is the level's while it stays inside its object; drawn only while selected, in the object's place.
2. A mesh inside the level's settings, with the editing tools taught to target "object X's level k": every tool, the picking and the component overlay would need a second kind of target. Rejected: it duplicates the editing path.
3. Swapping the level's shape into the object while it is edited: everything else (exports, other levels, saving during the edit) would see the wrong shape.

How editing a level is entered and left:
1. **Selecting its shape is editing it (chosen).** The panel's **Edit this level** (a toggle) selects the level's shape, **Make this level by hand** does too, and so does clicking it in the Objects panel. While it is selected the viewport draws it in place of its object, with a banner and a **Stop editing the level** button; selecting anything else (or Edit this level again, or undo) stops. No state of its own to get out of step with undo.
2. A separate "editing level" switch in the workbench: must be kept in step with selection, undo and deletes.

## Decision

### Modifier ids

`Modifier::id: ModifierId(u32)`. `modifier::with_ids` names a list: each keeps its id unless 0 (not given yet: `Modifier::new`) or taken by one before it, which gets the next after the highest. Every change to a list goes through `Run::replace_modifiers` (the commands, lattices, copies, sources forgotten, baking), which names it and then takes out of the object's levels (and its instances', whose levels change the original's modifiers) the changes to modifiers no longer there (`forget_gone_overrides`): **removing a modifier removes its changes, in the same undo step.** An id is reused only after its modifier and its changes have gone. Files before version 63 get ids in order.

### The levels (`scene::lod`)

`LodLevel` gains:
- `reduce: bool` (on for every level of a file before 63, and for new levels): reduce automatically to `ratio` of **the level's own triangles** (after its changes) as the last step.
- `budget: u32` (0: none): only shown beside the count, amber when over.
- `overrides: Vec<ModifierOverride { modifier, on, settings: Option<ModifierKind> }>`, at most one per modifier. `LodLevel::with_override` adds or changes one; a change that changes nothing (on as the model, settings as the model's) is dropped (`tidied_overrides`), so "Same as the model" is that. `LodLevel::applied` gives the modifiers as the level has them. `Lods::tidied(modifiers)` drops changes naming no modifier of the object's, or settings of another kind.
- `hand: Option<HandLevel { shape: ObjectId, made_from: u64 }>`.

**What a level is made from** (`lod/make.rs`): `Purpose::Level(k)` joins `Viewport` and `Export`: worked out as an export (`Purpose::is_export`), with level k's changes applied (`Scene::modifiers_for`); its sources (a scatter's shapes) are worked out for the export (`for_sources`). `Scene::stack_state` for a level starts from the longest run of steps it shares with the level's last run, or the export's: a change at the third modifier works out only from the third. Each level is:
- made by hand: its shape's exported shape (its own modifiers included), placed into the object's shape space when it was moved off it;
- with changes that change what is worked out: `Purpose::Level(k)`;
- otherwise: the object's exported shape.

Then levels side by side made from the same shape and reduced are reduced in one go (`decimate_levels`, nested as before); a level not reduced is its shape. With no changes and every level reduced, that is exactly 0155's levels. **Levels are always the shape a game gets** (export purpose), so the preview and the triangle counts are of what goes out (0155 used the viewport's shape for them).

`Scene::lod_state(id)` (no longer given a shape) gives `Ready`, `Needed(LodJob)` or `Waiting` (heavy booleans still being worked out elsewhere); it never works anything heavy out. A `LodJob` holds every shape still to work out (the export's stack, each level's, a hand-made level's modifiers) and the reductions, and reuses the reductions kept for groups whose shapes didn't change; the stacks it works out are stored with its result (`store_lods`). `Scene::lod_shapes(id)` works them out on the spot, for exports and for making a level by hand. Work is counted in triangles plus the stacks' estimates; from `LODS_IN_BACKGROUND_FROM` (5 000) the interface does it on another thread, showing the last level meanwhile.

### Levels made by hand (`lod/hand.rs`, `command/levels.rs`)

- **`Command::MakeLevelByHand { id, level }`** ("Make level 2 by hand"): what the level shows now (`automatic_level`, made exactly as the levels are) becomes a mesh object inside `id`, named `<name> LOD<n>` (`lod::level_name`), its pivot on the object's (its offset the object's), smoothed and coloured as the object is; the level points at it with a fingerprint of the model; the new shape is **selected, so it is edited at once**. Refused without levels on.
- **`Scene::level_shape(id, k)`**: the level's shape only while it is **inside** `id` and has a shape; otherwise the level is made from the model again. So deleting the shape (Delete, or the panel's **Make from the model**) or dragging it out in the Objects panel makes the level automatic, and an ordinary object again; `Run::forget_level_shapes` also clears the level in the same delete step. `Scene::level_owner(shape)` finds the object and level.
- **Drift:** `Scene::model_fingerprint(id)`, a steady FNV-1a hash of the model (the shape before modifiers, as edits, booleans and the live mirror leave it; each modifier's settings, on and Show in viewport; the smoothing), kept per object by shape so asking every frame costs nothing while nothing changes. `level_drifted` compares it with the one saved when the level was made.
- **`Command::RestartLevel { id, level }`** ("Start level 2 again from the model"): the shape's edits go, it becomes what the level would show now, back on the object's pivot, and its fingerprint is renewed.
- `SetLods` keeps each object's own hand-made levels (by place; they are only made by the command); taking away levels made by hand deletes their shapes in the same step. With several objects, each keeps its own changes to the modifiers (their ids are each object's own).
- Copies (`Duplicate`) point their levels at the copied shapes (`follow_copied_levels`); an object put in from elsewhere (pasted, imported) loses its hand-made levels (`keep_usable_levels`), as their shapes aren't its: its child shapes become ordinary.
- Select all and Invert leave level shapes out.

### Editing a level in the viewport

The drawing cache (`ui/src/drawing/levels.rs`) leaves out a level's shape unless it is selected, and leaves out an object while one of its level shapes is selected: the viewport shows the level in the object's place, and picking, the component modes, the gizmos and every Mesh menu command work on the level's shape because it is the selection. The viewport draws a banner, *Editing Level 2 of Rock: its own shape, made by hand. The object is hidden meanwhile.*, with **Stop editing the level** (selects the object). The Objects panel shows the level shape's name quieter while it isn't edited, and says what it is on hover.

### The Levels panel

`Panel::Levels`, titled **Levels**: a tab beside Attributes in the standard layout; **Window → Levels** shows it or puts it away; it comes back beside Attributes by itself when a shape with levels is selected (as Edit layers does, 0126); Make levels of detail (command search) and the Attributes line's **Open the Levels panel** bring it to the front. It shows the levels of the first selected shape (while a level is edited, its object's):

- **Make levels**, **Levels** (1 to 8).
- **Full shape**: its **Preview** switch and triangles.
- Each **Level n**: its **Preview** switch (the row is filled in the accent's soft colour while the viewport shows it) and *triangles*, or *triangles of budget* (amber when over); **Shown from**; **Budget**; then, made from the model: **Reduce** with its share, **Modifiers at this level** (each of the object's modifiers with its switch at this level, *changed here* and **Same as the model** when the level changes it, and **Settings here** opening its settings rows for this level), and **Make this level by hand**; made by hand: **Edit this level**, **Make from the model**, a line naming its shape, and when the model has changed since, *The model has changed since this level was made.* with **Start again from the model**.

Every change is a command (`SetLods` merged by gesture, so a drag is one undo step; `MakeLevelByHand`; `RestartLevel`; `Delete`; selecting). Widget ids are salted by object, level and modifier id; 0164's clash test opens the panel with eight levels, half the modifiers changed, settings open, and a level made by hand. Attributes keeps a **Levels of detail** line: how many levels, and the button.

### Exported

`fileio::export::lods::levels` writes **each level's real shape** from `Scene::lod_shapes`, as 0155's file layout (`name_LOD1` beside the object, extras, hidden for Godot). Level shapes are never exported as objects of their own (`Scene::is_level_shape` in `prepare_for`, `summary`, `prepare_piece`). The Godot import check's `godot-lods.glb` now has a sphere with Subdivide whose Level 1 switches it off and Level 2 smooths once, and a ring whose Level 2 is made by hand with a slice taken out; `check-scenes.gd` checks the levels' triangles and that no `Torus LOD2` node comes in. The glTF Validator passes the files.

### Saving

Development file format **version 63**: version 62's, with each modifier's `id` (`v43::ModifierV43::id`) and per object the levels' details (`v63::LevelsV63`, in `v43::ProjectV43::levels`: reduce, budget, changes by modifier id with settings as `KindV43`, the hand-made shape's object id and fingerprint), applied after the modifiers. A hand-made level naming an object not in the file, or an object listed twice, is refused as damaged; changes naming no modifier are left out; repeated modifier ids get new ones. Files before 63 open with ids in order and every level reduced, as they were made.

### Timings

Measured on a 100 000-polygon ball with Subdivide-free modifiers (Triangulate and a Live bevel) and three levels, one switching the bevel off (release build; see the pull request for the numbers): asking for the levels each frame costs nothing heavy (`lod_state` only plans), switching Preview between levels hands the work to another thread and the window keeps drawing at full speed while the last level stays shown.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** Edit this level is a toggle in the panel; the viewport banner has a button.
- A level's settings change replaces **all** of that modifier's settings at that level (the rows start as the model's; once changed, that level keeps its own copy and no longer follows the model's later changes to that modifier, while still following every edit of the shape). **Same as the model** takes the change away.
- **Reduce** keeps a share of **the level's own triangles** after its modifier changes, not of the full shape's.
- **Levels show the exported shape** (Subdivide's export levels, every modifier that is on), in the preview and the counts too.
- **Make this level by hand selects the new shape at once** (editing starts); a hand-made level is a child object `Rock LOD2` in the Objects panel, shown only while selected; deleting it or dragging it out makes the level automatic again; copies get their own; pasted objects lose theirs.
- **Editing a level = selecting its shape**; the banner's *Stop editing the level* selects the object again.
- **Start again from the model** throws the level's edits away (undo brings them back).
- The model counts as changed (the drift note) when its shape before modifiers, its modifiers' settings or its smoothing change; moving or renaming it doesn't.
- **A budget is only shown** (amber when over); nothing is reduced to fit it automatically.
- The Levels panel is a tab beside Attributes, comes back there by itself when a shape with levels is selected, and **Window → Levels** shows or hides it.

## Consequences

- Projects saved with this version need it or later to open.
- `Scene::lod_state` and `lod_shapes` take only the object; `Purpose` has a third case, `Level(k)`.
- Levels can now be heavier to work out than before (a whole stack per level with changes); the work is in the background and reuses the export's steps.
- Not yet: changing single settings of a modifier per level; automatic reduction to a budget; a level's own collision; switching the preview by the camera's distance.
- The manual's Levels of detail page describes the panel; Attributes, Export, Interface tour, Objects and Modifiers pages mention it.
