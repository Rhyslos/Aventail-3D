# 0155. Levels of detail

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0145](0145-decimate.md) (Decimate's `decimate_levels`, made for this), [0151](0151-collision-hulls.md) (extra shapes beside an object: a per-object setting, worked out in the scene with a cache, written beside the object in exports), [0078](0078-kit-export.md) and [0122](0122-solis-preset-and-remembered-export-settings.md) (kit export, Godot's names only in exports for Godot, the last export's settings remembered), [0135](0135-live-booleans.md) and [0142](0142-modifier-stack-and-remesh.md) (live work kept in the scene, heavy work on another thread), [0110](0110-controllable-automation.md) (automatic tools show their settings and give the same result every time), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0154](0154-shell.md) (the development file format moves to version 56)

## Problem

Phase 10's seventeenth milestone is the **LOD generator**. A game draws a far-away object with as many triangles as a near one unless it has **levels of detail** (LODs): simpler copies swapped in by distance. The notes ask for a **LODs** setting per object with **Levels** (recommended 3), each level a **Ratio** (50%, 25%, 12%) and a **Distance**, made with Decimate; a **Preview level** switch in Attributes; and, for export, to find out what Godot 4 actually reads (a glTF LOD extension such as `MSFT_lod`, named meshes, or only its own levels made on import), make the export do that, and also export each level as its own named mesh in the kit export, with the option documented. The README's roadmap line said "exported with Godot's LOD names", which assumed such names exist.

Left to decide: a modifier or a setting; what Godot reads, and so what the export writes; what the levels are made from; names and notes in the file; the preview; saving.

## What Godot 4 does with levels of detail

Checked in Godot's source at the tag CI pins (4.7.2-stable) and by importing files into Godot 4.7.2 itself:

- **No glTF LOD extension.** Godot's glTF module (`modules/gltf`, `gltf_document.cpp` and `extensions/`) reads `KHR_lights_punctual`, `KHR_node_visibility`, `KHR_texture_transform`, `KHR_materials_*`, `KHR_animation_pointer` and a few texture extensions; `MSFT_lod` appears nowhere in it.
- **No LOD name ending.** The scene importer's name endings (`editor/import/3d/resource_importer_scene.cpp`, `_teststr`) are `-noimp`, `-col`, `-convcol`, `-colonly`, `-convcolonly`, `-rigid`, `-navmesh`, `-occ`, `-occonly`, `-vehicle`, `-wheel` and the animation loop endings; there is no `-lod`. A node called `Rock_LOD1` imports as an ordinary mesh.
- **Godot makes its own.** Import option `meshes/generate_lods` (on as standard; per mesh `generate/lods` Default / Enable / Disable and `lods/normal_merge_angle`, 20°) runs `ImporterMesh::generate_lods` (meshoptimizer) on every mesh; at run time Godot picks a level by screen-space error (Project Settings → Rendering → Mesh LOD → threshold, and each instance's LOD bias). Imported, the app's standard sphere comes with 6 levels, the cone with 2. The Godot manual's "Mesh level of detail (LOD)" page (docs.godotengine.org, `tutorials/3d/mesh_lod.html`) describes this automatic, import-time LOD; Godot's LOD tracker ([godotengine/godot#57416](https://github.com/godotengine/godot/issues/57416)) says "Automatic LOD generation is only supported when importing 3D scenes", and that manual (H)LOD is done with **visibility ranges** instead.
- **What it does read that helps:** a node's glTF `extras` become its metadata (`set_meta("extras", …)`, `_attach_extras_to_meta`), and `KHR_node_visibility` with `"visible": false` imports the node hidden (`gltf_document.cpp`, node parsing). Both confirmed by importing a test file in Godot 4.7.2. A post-import script (`EditorScenePostImport`) can then set each node's visibility range (`visibility_range_begin` / `_end`) from that metadata; the import settings can also set visibility ranges per node by hand.

So Godot can't take hand-made levels from a glTF file by itself. The honest design is the one the brief foresaw: **Godot's own import levels for Godot**, and the app's levels written as named meshes for whoever wants them, with the distances in the file, hidden for Godot, and a short script in the manual that turns them into visibility ranges.

## Options

A modifier, or a setting:
1. **A per-object setting, as Collision is (chosen).** The levels are extra shapes *beside* the object; the object itself never changes. A modifier changes the shape you see and export, has a place in the stack's order and is made permanent by Bake into shape: none fits (baking levels into a shape means nothing, and they must be made from the end of the stack, not from somewhere in it).
2. A modifier at the end of the stack: a palette button, but it would change nothing, could be moved where it makes no sense, and Bake into shape would have to skip it.

What Godot gets:
1. **The full shape, for Godot to make its own levels (chosen as the standard), and the app's levels on request, hidden, with their distances in `extras`.** Godot's levels are on as standard and switch by screen size, which suits most games. Someone who wants their own counts and distances ticks one box, and either uses the manual's script (visibility ranges) or wires the hidden meshes up by hand.
2. Always write the levels: four overlapping copies in every Godot scene, for no gain over Godot's own.
3. Write a Godot `.import` file setting each level's visibility range: Godot's own file, which it rewrites, in a format it changes between versions; the app only replaces files it wrote (0076).

Names:
1. **`name_LOD1`, `name_LOD2`, … beside the object, which keeps its own name as level 0 (chosen).** `_LOD` and a number is the naming most game tools look for (Unity's model importer groups `_LOD0`, `_LOD1`… meshes into a LOD Group, for example), and the object keeping its name keeps its Godot collision ending (`Rock-convcol`) and every scene that places it by name.
2. `name_LOD0` for the full shape too: the full convention, but it renames every exported object that has levels, and breaks `-col` endings for Godot.
3. `name-lod1`: in Godot's ending style, but Godot gives it no meaning, and other tools don't look for it.

What the levels are made from:
1. **The object's final shape for export (chosen):** every modifier that is on, booleans and the live mirror, as the export writes level 0, so each level is a simplification of what the game gets. The Attributes panel's counts and the viewport's preview are made from the shape the viewport draws (the same unless a modifier has a lighter viewport setting, Subdivide's levels), so previewing never waits for export-only work.
2. The shape before modifiers: levels wouldn't match the exported shape.

## Decision

### The setting (`scene::lod`)

`Object::lods: Lods { on, levels: Vec<LodLevel { ratio, distance }>, preview }`, off as standard, its settings kept while off:

- **Levels** 1 to 8 (`Lods::MAX_LEVELS`), standard **3**: **50%** from **10 m**, **25%** from **20 m**, **12%** from **40 m** (`Lods::STANDARD`). A level added after the last has half its ratio and twice its distance (`next_level`, `with_count`).
- **Ratio** within Decimate's (0.1% to 100%); a level whose ratio is above the one before's is made like that one (`Lods::targets` takes the running minimum), so levels never get finer. **Distance** 0 to 10 km. **Preview** 0 (the full shape) to the number of levels. `clamped` brings anything within limits (not a number: a halved ratio, a distance of 0; no levels at all: the standard three).
- `Command::SetLods { ids, lods }` (clamped; groups left alone; `Edit::SetLods`), "Change the levels of detail": one undo step a drag.

The levels: `lod_shapes(mesh, lods, hardness)` calls `mesh::decimate::decimate_levels` with `Keep::default()` (UV seams, hard edges as the object's shading shows them, material borders and open borders all kept), triangles out, once for all levels, so they are **nested** (each level's points are some of the one before's) and cost about as much as the simplest alone. A shape with a point at no real place gives none, with the reason. The scene keeps each object's last levels with what they were made from (`LodCache`, not part of equality, forgotten on delete): `Scene::lod_state(id, shape)` (`Ready` or `Needed(LodJob)`, work in triangles, `LODS_IN_BACKGROUND_FROM` 5 000 as modifiers), `store_lods`, `lod_shapes` (on the spot, for exports) and `lods_if_ready` (the Attributes panel's counts, never working anything out). The key is the shape, the ratios and the object's smoothing: a distance alone doesn't make them again.

### In the interface

- **Attributes → Levels of detail** (every shape, after Modifiers): **Make levels** (switch), then **Levels**, each **Level n** (its percentage) with **Shown from** (a length) and **Triangles** (once made, … until then), **Preview** (Full shape, Level 1, …), and a line giving the full shape's triangles, or the reason in amber.
- **Preview** draws that level in the shape's place (`ui/src/drawing/lod.rs`, from the drawn shape; light shapes at once, heavy ones on another thread with the last level shown meanwhile). The shape picked, selected and edited stays the full one. A selected object's levels are made even while it shows its full shape, so its counts appear.
- **Command search: Make levels of detail** (`MenuAction::MakeLods`): levels on for the selected shapes, with the first one's settings. **No keys or mouse actions.**
- The viewport doesn't switch levels by distance: Preview chooses one, and the distances are for the game.

### Exported

`ExportOptions::lod_levels`, the Export window's **Levels of detail** chip beside *Includes* (off as standard, kept when Made for changes, remembered with the export settings, used by Export…, Export again, Export kit… and exports into the game project folder). With it on, an object with levels gets `ExportedObject::levels` (made from its exported shape, placed and shaded as it is, distances in the file's units) and:

- **glTF:** each level a node and mesh of its own beside it, `name_LOD1` and on, at its pivot, with `extras` `{ lod_level, lod_begin, lod_end }` (metres; an end of 0 is no end), and the object's own node `{ lod_level: 0, lod_begin: 0, lod_end: Level 1's distance }`. In an export with Godot's names, the levels carry `KHR_node_visibility` `{ visible: false }` (listed in `extensionsUsed`, not required). The glTF Validator passes the file (no errors or warnings).
- **OBJ:** each level an object of its own after the others, `o name_LOD1`.
- The Export window's numbers count the objects, not their levels.

`tools/gltf-check/lod_ranges.gd` is the post-import script the manual prints; the Godot import check (`godot-import.sh`, `check-scenes.gd`) imports `godot-lods.glb` (a sphere and a ring with the standard levels) with it and checks every level is shown between its distances (0–10 m, 10–20 m, 20–40 m, 40 m on). Godot 4.7.2 passes; without the script, the levels import hidden.

### Saving

Development file format **version 56**: version 55's model with, in each object's development settings (`v39::ObjectV39`), `lods` (`v56::LodsV56`: on, each level's ratio and distance, preview), written when not the standard. Settings are clamped when read (more than 8 levels cut short); levels on for an object with no shape are refused as damaged. Files before 56 hold none.

### Timings

The levels cost one decimation (decision 0145): three levels of a 100 000-triangle sphere about 0.66 s in a release build, on another thread from 5 000 triangles. The standard sphere's three levels take a few milliseconds.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **A per-object setting, not a modifier**: no palette button and no Mesh menu entry; it's in **Attributes → Levels of detail** and command search's **Make levels of detail**.
- **For Godot, Godot's own levels**: exports leave the levels out unless **Levels of detail** is ticked under *Includes* in the Export window (off as standard). The roadmap's "exported with Godot's LOD names" can't be done: Godot has none.
- Levels written as **`name_LOD1`, `name_LOD2`…**, the object keeping its own name as level 0; **hidden** in Godot exports; distances in glTF `extras` (`lod_begin`, `lod_end`, metres).
- Standard **3 levels: 50%, 25%, 12%, from 10 m, 20 m and 40 m**; up to 8; a new level halves the last's ratio and doubles its distance.
- Seams, hard edges, material borders and open borders are **always kept** (Decimate's standard); no switches for them here.
- **Preview** is saved with the project and is an undo step, like the other settings; it draws one level, it doesn't switch by distance.
- **No new keys or mouse actions.**

## Consequences

- Projects with levels of detail need this version of the app or later.
- Godot users get levels with no work (Godot's own); those who want their own counts get them with one tick and a 20-line script.
- Not yet: switching the preview by the camera's distance; per-level Keep switches; Unity's `_LOD0` grouping (Unity reads glTF only through a package); writing a level's own collision.
