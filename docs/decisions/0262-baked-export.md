# 0262. Baked export

- **Status:** Accepted
- **Date:** 2026-11-07

## Problem

Phase 12's sixth milestone ([the plan](../plans/phase-12.md), [decision 0256](0256-material-graph-plan.md)) is "Baked export": graphs into textures through the texture export, with every engine preset, and a graph's material given to a shape without painting. Games can't run the app's graphs; every engine reads pictures. Things to decide: what "giving a graph to a shape" is in the project, how it is made and kept up to date, how it goes out, how a graph goes out on its own (a tiling texture set for other apps), what size things are made at, and how the export says what won't come out as the app shows it, including animated boxes that don't exist yet (milestone 15).

## Options

- **What a shape given a graph holds:** a new "made by graph" field on materials (a new kind of thing for the scene, the viewport, undo, saving and the export to learn, and a file format bump), or **a texture set holding one graph fill layer** (chosen). Milestone 5 already makes graph fills on the model at the material's real-world size, in the background, again whenever the graph, the shape, its UVs or the texture size change, with their pixels joining the change's undo step; a texture set already shows in the viewport, saves (file format 97), and goes out through the texture export with every preset and packing. Nothing new to keep in step, no file format bump, and painting over a given material just works. A material is **made by a graph** when its set holds one graph fill and nothing else.
- **Giving a graph to a painted material:** refuse, replace the paint, or **put the graph under the paint** (chosen): the paint stays on top and the graph is the base, said in the status bar. A material made by another graph changes graph.
- **Where to give it:** the Material mode (**Give to the selected shapes**, under the preview: each selected shape's first material, the commonest case) and **Attributes → Materials → Graph…** on each material (a menu of the project's graphs, and **No graph** to take it back), for a shape with several materials. Both one undo step. A shape sharing the standard material with another first gets its own (decision 0232), in the same step.
- **A graph on its own:** through the model export (no model to put it on), or **Export textures…** in the Material mode (chosen): a folder is asked for and the material is written as a seamless texture set, made for the Export window's Made for. The maps follow the shared rule (`fileio::export::maps_going_out`, decision 0208), all beside as files: colour, the realism maps packed per engine (ORM; Unity's mask map; or separate maps), the normal map (green down for Unreal Engine). A texture set always takes its height as grey, and with separate maps its occlusion, because tools reading texture sets use them. The ORM and mask map carry the material's **own occlusion**. For Godot, a material file (`.tres`, an `ORMMaterial3D` naming the colour, normal map and ORM, with occlusion on) comes with it, as the baker's does (decision 0253). Nothing is padded: the set repeats seamlessly, so its edges already lead into each other.
- **Sizes:** a shape's material is made at its texture set's size (the project's texture size, as painting is). A texture set on its own is made at the Material mode's **Texture size** (the texture budget's, decision 0260), which a refusal's **Use … pixels across** fix now sets.
- **Saying what won't come out as shown:** `matgraph::bake_notes(registry, graph, on_model)` lists, in order, boxes from a newer app (they stop the bake), **animated boxes** (baked at their current setting, so they stand still in the game) and, off a model, **From the model** boxes (their stand-in is baked; give the graph to a shape to bake it on the model). `BoxInfo::animated` is the hook milestone 15's Time, Scroll, Pulse and Flow set; no box sets it yet. The Material mode shows these above Export textures… (with "no Material out" first), and the Export window's **Material graphs** part shows them for every graph going out, plus materials still being made or that can't be made, each with what to do.

## Decision

- **matgraph:** `baking` (`BakeNote`: `Animated`, `FollowsModel`, `Unknown`; `describe`, `stops`; `bake_notes`); `BoxInfo::animated` (false for every box so far).
- **fileio:** `export::graph_textures` (`SetOptions`, `SetFile`, `set_maps`, `file_names`, `texture_set`, `write_set`, written safely through `project::safe_write`); `TextureMap::Occlusion` (`_Occlusion`; white for painted sets); `bake_file::godot_material_with` (with a colour texture) and `bake_file::godot_path` (moved from the baker).
- **ui:** `material::giving` (`given_graph`, `with_graph_given`, `Given`), `material::export_set` (the **Use it** part under the preview: **Give to the selected shapes**, the export check and **Export textures…**, run on another thread), `workbench::graph_giving` (giving, taking back, `export_graph_textures_into`, `graph_export_checks`); Attributes → Materials gains **Graph…** and "Made by the graph …"; the Export window gains **Material graphs**; `Fix::UseSize` sets the Texture size; `WorkbenchAction::ChooseGraphTexturesFolder` and `FilePurpose::GraphTextures` (the app asks for the folder, or for a typed path without a file window).
- **No file format change:** a given material is a texture set with a graph fill (file format 97).
- **Controls:** no new keys or mouse actions: buttons and menus.

## Consequences

- A shape given a graph goes to Godot, Unity, Unreal Engine, Solis and Substance Painter as ordinary textures, with each engine's packing, with no extra steps.
- The occlusion part of a model export's ORM stays white for a material made on a shape, as for painted ones (decision 0208): the layer keeps colour, roughness, metal and height, not occlusion. Export textures… carries the graph's own occlusion. Putting occlusion into the model export is left for later.
- A model export's normal map comes from the material's height (as painted height's does, decision 0211), not from the graph's own normal map; Export textures… writes the graph's own.
- A graph's material on a shape repeats by the first shape using that material (as graph fills do).
- When the engine translators come (after Phase 12), `bake_notes` is where boxes marked "translatable" will be told apart from those baked.
