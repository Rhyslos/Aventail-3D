# 0142. The modifier stack, and Remesh

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0006](0006-base-plus-operation-stack.md) (base plus operation stack), [0035](0035-heavy-meshes.md) (heavy work off the interface thread), [0088](0088-live-mirror.md) and [0126](0126-rework-and-first-use.md) (the live mirror, Bake into shape, the development file format), [0135](0135-live-booleans.md) (live booleans: the pattern for a live feature), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's modifiers and their order)
- **Replaces part of:** [0126](0126-rework-and-first-use.md) (one development file format: the modifier stack gets version 43)

## Problem

Phase 10's fourth milestone is Remesh, and it comes first of about fifteen modifiers (smoothing, lattice, decimate, scatter, curve deform and shrinkwrap, weighted normals, live bevel, quadify, collision hull, weld, array, shell, LODs, displace, bend/twist/taper, wireframe, triangulate). Each is non-destructive: it sits on the modifiers palette and in the Mesh menu, command search finds it, its settings are in Attributes, Bake into shape makes it permanent, it is worked out with a cache (on another thread when heavy), and it exports as what you see. Rather than each milestone building that again, this one builds the **modifier stack** they all plug into, then Remesh on it.

Left to decide: the data model, the commands, where booleans and the live mirror sit, caching and threads, the Attributes section, the palette and menu, Bake into shape, saving, and Remesh's method, limits, materials and UVs.

## Options

Where booleans and the mirror sit:
1. **Booleans and the mirror stay where they are, before the stack (chosen).** Booleans keep their own list (0135) and files, unchanged; the stack starts from what was shown before. Clear to explain: *edits → booleans → mirror → modifiers*.
2. Booleans become one kind of modifier. More uniform, but booleans name other objects and draw them as outlines, and 0135's files, commands and panel would all move for no gain to the user.

Whether the mirror comes before or after the modifiers:
1. **Before (chosen),** as in Blender, where the mirror usually goes first. A modifier sees the whole shape: smoothing runs across the seam with no crease; Remesh gets a closed shape from a half that is only closed once mirrored.
2. After: Bake into shape could keep the mirror live, but Remesh would refuse every half-modelled shape (open at the seam), and smoothing would crease along it.

Remesh's method:
1. **Surface nets on a signed distance grid (chosen):** simple, robust, quads only, one point per cell, so even quads. Sharp corners are rounded off at about a voxel (expected for a voxel remesh).
2. Dual contouring: keeps sharp corners, but needs normals at every crossing and a solver per cell, and makes folded polygons near thin parts. Can be added later as an option if wanted.
3. Marching cubes: triangles only, uneven sizes.

## Decision

### The data model

`Object::modifiers: Vec<Modifier>` (in `scene/src/modifier.rs`), first to last, at most `MAX_MODIFIERS` (32). Each `Modifier` has:

- `kind: ModifierKind`, an enum with one variant per kind holding its settings (`Remesh(RemeshOptions)` now). It is exhaustive on purpose: adding a kind makes the compiler point at every `match` that needs it.
- `on: bool`: off, the shape is as if the modifier weren't there.
- `in_viewport: bool` (**Show in viewport**): off, the viewport skips it but exports and Bake into shape still use it (for a heavy modifier while modelling). This fits cleanly because evaluation already knows its **`Purpose`**: `Viewport` or `Export` (smoothing's *Levels* and *Levels for export* need it anyway). `Modifier::applies(purpose)` = `on && (Export || in_viewport)`.

`ModifierKind` gives each kind: `name`, `description` (the palette's and menu's tooltip), `family` (Generate, Deform, Clean up, Game ready: how the palette and menu group them), `clamped` (settings within limits; values that aren't numbers become standard), `sources` (other objects it reads: none yet; a lattice's cage, a curve to bend along), `with_sources_renamed` (for copies made with their sources), `work(input)` (a cost estimate, in polygons of a boolean's worth) and `apply(input, context) -> Result<Mesh, String>` (the work; an error is a plain-words reason starting with a small letter). `ModifierContext` carries the `Purpose` and the sources' final shapes placed into the object's space (`SourceShape`).

Groups take no modifiers; nor do instances: an instance shows its original's final shape, modifiers included (`Scene::modifier_owner`). `Scene::check_modifiers` refuses those, more than 32, and sources that aren't shapes, are the object itself, or depend on it (`shape_uses`, which now follows modifier sources too, so loops are refused as booleans' are).

### The commands

All change the list through one reversible edit, `Edit::SetModifiers { id, before, after }`:

- `Command::AddModifier { ids, kind }`: at the end of each object's list, on and shown, settings clamped. Refused as a whole, changing nothing, if any object can't take it. Undo label "Add Remesh".
- `Command::ChangeModifier { id, index, modifier }`: settings, on/off or Show in viewport; same kind only (`ModifierRefusal::OtherKind`). Labels "Change Remesh", "Switch Remesh off", "Hide Remesh in the viewport". The Attributes panel sends it merged by gesture, so one drag is one undo step.
- `Command::RemoveModifier { id, index }` and `Command::MoveModifier { id, from, to }`.

Deleting an object removes every modifier that reads it (`forget_modifier_sources`), in the same undo step, as deleting a cutter ends its booleans. Duplicating keeps the copy's modifiers, pointed at copied sources (`follow_copied_sources`); an instance copy gets none. An object put in from elsewhere loses modifiers that don't make sense there (`keep_usable_modifiers`).

### Evaluation order and caching

An object's shape is worked out in this order:

1. its base and edits, layer by layer: `Scene::evaluated` (what the Points, Edges and Polygons modes edit and pick, unchanged);
2. its booleans: `Scene::cut_shape` (0135);
3. its live mirror: `Scene::with_mirror` (0088). Steps 1–3 together are `Scene::unmodified_shape` (what `shown_shape` was before);
4. its modifiers that apply for the purpose, first to last, each on the one before's result: `Scene::final_result(id, purpose)` / `final_shape`, with `Scene::shown_shape` (viewport) and `Scene::exported_shape` (exports and baking).

The scene keeps, per object and purpose, the input and each step's result (`StackCache`, not part of the scene's equality, emptied in copies, forgotten when the object goes). `Scene::stack_state(id, purpose, input)` reuses the longest run of steps whose kind, settings and sources are unchanged on the same input, and hands back either `Ready(StackResult)` or `Needed(StackJob)` with only the rest to do: changing the last modifier works out only it. A step that fails passes its input on and records its reason; `StackResult::problems` lists them by place, and `Scene::modifier_problems(id)` gives them to the Attributes panel (worked out at once when light, left out while heavy work is pending). A result over `MAX_SHAPE_POLYGONS` fails with a reason. The mirrored shape the stack starts from is kept as well (`Scene::mirrored_shape`, until the shape, mirror or place changes), so the input is the same shared shape each time it's asked for and the cache recognises it without comparing meshes.

**Threads.** `StackJob::work()` sums the remaining kinds' estimates; from `MODIFIERS_IN_BACKGROUND_FROM` (5 000, as booleans) the interface's drawing cache (`ui/src/drawing/stack.rs`) runs `StackJob::run` on another thread, one job per object at a time, shows the last result (or the shape without its modifiers) meanwhile, and hands the result back with `Scene::store_stack`. The window keeps redrawing while work is out (`DrawingCache::busy`). The drawing cache passes the whole shape (hidden polygons included, as booleans do) cut and mirrored as the input, so its cache entries are the scene's. Exports and Bake into shape work out what they need on the spot.

### The Attributes section

**Modifiers** (`ui/src/attributes/modifier_rows.rs`), after Booleans, shown when the object has any: for each modifier, a line with its on/off switch and name, **Up**, **Down** and **Remove**; its own settings (`kind_rows`, one function per kind); **Show in viewport**; and, in the warning colour, "Not worked out: *reason*." when it couldn't do its work. A short note says the order and that Bake into shape makes them permanent.

### The palette and the menu

One `CommandId` per kind (`CommandId::Remesh`), named and described from `ModifierKind`, in `commands::OFF_THE_RING` (not on the E ring, like the booleans) and usable in every mode. `ui/src/modifiers.rs` maps commands to kinds (`COMMANDS`, `kind_of`) and decides the targets: the selected shapes, groups left out, refused with a reason when nothing is selected, an instance is, or a shape already has 32. The **modifiers palette** shows one icon button per kind after the booleans, family by family with a little space between families; a click adds that modifier to the selected shapes. The Mesh menu gets a **Modifiers** submenu, each family under its small grey name. Command search finds them under "Mesh". **No new keys or mouse actions.**

### Bake into shape

Bake into shape (`Command::MakeEditable`, Ctrl + E) on an object with modifiers that are on bakes its **whole exported shape**, edits, booleans, mirror and modifiers, into a plain mesh (`bake_with_modifiers`): its edit layers go, its booleans that worked end (cutters hidden, as 0135), its **live mirror ends** (its copies are part of the mesh now, and a mirror left on would double them), and the modifiers that were on and worked are removed. Modifiers switched off, or that couldn't do their work, stay. One undo step. Without modifiers that are on, nothing changes from before (the mirror stays live). **Apply mirror** bakes the shape before the modifiers, which stay live on the result. Baking uses the **export** results: what you get is what an export would get (for smoothing, its export level).

### Saving

Development file format **version 43** (`fileio/src/project/v43.rs`): version 41's model (as version 42 used) plus, per object with any, its modifiers (`KindV43` with each kind's settings, `on`, `in_viewport`). A version of its own, as booleans and curves had, because an older app would drop the modifiers without a word. Settings are clamped when read; an object listed twice, a missing object, a group or instance with modifiers, or too many are refused as damaged. Older files open with no modifiers.

### Remesh

`mesh::remesh::remesh(mesh, options)` (layer 0):

1. **Closed only.** Every edge, with points within 0.001 cm counted as one, must be used by an even number of polygons; otherwise refused: "the shape isn't closed…". Zero-length edges are ignored.
2. **The grid.** Points `voxel_size` apart round the shape's box, two empty points of padding each side, centred on the shape. More than **2 000 000** points is refused with the smallest voxel size that fits ("Make Voxel size 0.67 cm or more").
3. **Inside and outside** by the winding number along a line in X through each row of points: +1 where it goes into the surface (a triangle facing back along X), −1 where it comes out, inside where the sum isn't zero, so overlapping shapes are one solid (counting crossings alone would hollow out their overlap). The line is nudged a tiny odd fraction of a step off each row so it never runs through an edge or corner. **Distance** to the nearest triangle (exact, Ericson's closest point) within 1.5 steps of the surface; that triangle is remembered for materials.
4. **Clean corners.** Where inside points touch only along a square's diagonal or a cube's long diagonal, an outside point is filled in (only ever filling in, so it ends). The grid is then *well-composed*, and the surface is a clean closed solid: every edge shared by exactly two quads, no pinched points.
5. **Surface nets.** One point per cell the surface crosses (the mean of where it crosses the cell's edges, placed by the distances), one quad per grid edge it crosses, facing out.
6. **Smooth:** optional relax passes (Taubin: in by 0.5, out by 0.53), evening the surface without shrinking it.
7. **Materials** from the nearest original polygon (the most common among a quad's corners). **UVs:** none can be kept meaningfully, so a box projection (each quad seen from the side it faces most, the shape fitted into the texture square, sides overlapping) with seams where the projection changes side. No hard edges: the object's smoothing angle shades the result.

Settings: **Voxel size** (standard **2 cm**, 0.01 cm to 100 m) and **Smooth** (standard **2** passes, up to 20). Deterministic: the same shape and settings always give the same mesh. Timings (release build): a standard sphere at 4 cm 4 ms, at 2 cm 14 ms (166 000 points), at 1 cm 57 ms (1.2 million). `ModifierKind::work` counts 40 grid points as one polygon, so from about 200 000 points the viewport remeshes on another thread.

### How to add a new modifier kind (the recipe)

1. **The work**, as a pure function in the crate whose job it is (usually `mesh`, a new module with its own tests: degenerate input, open borders, n-gons, NaN/zero/huge settings clamped or refused, never a panic). Settings in a `Copy`, `PartialEq` struct with `Default` (the recommended standard) and `clamped()`.
2. **`scene/src/modifier.rs`:** add the variant to `ModifierKind`; fill in every `match` the compiler then points at: `standard_kinds` (its place in the palette order), `name`, `description`, `family`, `clamped`, `sources` and `with_sources_renamed` (if it reads other objects: also see *Sources* below), `work` (a cost estimate; heavy work from 5 000), `apply` (use `context.purpose` if the viewport and export differ). Add scene tests (`scene/src/modifier_tests.rs` or a new `*_tests.rs`).
3. **`fileio/src/project/v43.rs`:** add a variant to `KindV43` with its settings (fields added later get `#[serde(default)]`), and to `KindV43::of` and `kind`; **move `FORMAT_VERSION` up by one** in `fileio/src/project.rs` (the `_ =>` arm already decodes it with `decode_modifiers`), write the new sample with `WRITE_SAMPLE=1 cargo test -p fileio write_the_sample` (after pointing `write_the_sample` at a rich document that holds the new kind), and add a `vNN_tests.rs` (round trip, the sample opens, older samples open, fuzz).
4. **`ui/src/commands.rs`:** a `CommandId` variant (name and help from the kind, as `Remesh`), in `ALL`, in `OFF_THE_RING`, and in the `modes()` arm with `Remesh`.
5. **`ui/src/modifiers.rs`:** add the command to `COMMANDS` (same order as `standard_kinds`) and to `kind_of`. The palette, Mesh → Modifiers and command search then offer it; `workbench/command.rs` needs the command added to the `CommandId::Remesh` arms of `readiness` and `run` (both call the generic modifier code).
6. **An icon** drawn in code in `ui/src/icons/modifiers.rs`, its `Icon` variant in `icons.rs` (`ALL` too).
7. **`ui/src/attributes/modifier_rows.rs`:** its settings rows in `kind_rows` (use `stepper::length` for lengths, so units show; `Rows` merges a drag into one undo step).
8. **Words:** `UPDATE_LANG=1 cargo test -p ui --lib text_checks`.
9. **Documents:** a manual page linked from `modifiers.md` and `SUMMARY.md`, concepts, CHANGELOG, the README tracker, its own decision record (referring here), and the reference pictures that change (the palette).

**Sources** (a modifier reading another object, for lattice, scatter, curve deform, shrinkwrap): list them in `sources()`; the scene then refuses loops, removes the modifier when a source is deleted, renames sources in copies, passes each source's final shape placed into the object's space in `ModifierContext::sources`, and re-works the step when a source's shape or place changes. A kind that draws helpers (a cage, an axis) adds them in the interface's drawing code; a kind that should outline its source as booleans do adds that in `Scene::cutters`-like code of its own.

**Edge or point layers** a modifier needs (creases, bevel weights, point sets) are mesh attribute layers set by recorded operations, like hard edges and seams, so they survive edits; the modifier reads them from its input.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- The order: edits → booleans → mirror → modifiers, the mirror before the modifiers as in Blender.
- Bake into shape on an object with modifiers also ends its live mirror (baked in); modifiers switched off stay.
- Baking and exporting use every modifier that is on, including those hidden in the viewport.
- A **Show in viewport** switch on each modifier, besides on/off.
- Up and Down buttons to reorder (no dragging), Remove with no question asked (undo brings it back).
- Remesh's standard **Voxel size 2 cm** and **Smooth 2** passes; the limit 2 million grid points.
- Remesh rounds sharp corners off at about one voxel (surface nets); box-projected UVs.
- No new keys or mouse actions: the palette button, Mesh → Modifiers → Remesh and command search.

## Consequences

- Projects with modifiers need this version of the app or later.
- `Scene::shown_shape` now includes the modifiers; what it meant before is `Scene::unmodified_shape`. Exports use `Scene::exported_shape`.
- Join, Split off, the mesh check, picking and the Points, Edges and Polygons modes work on the shape before the modifiers, as with booleans. Hiding polygons hides them from editing only.
- A cutter cuts with its shape before its own modifiers.
- Every later Phase 10 modifier follows the recipe above and records its own decision.
