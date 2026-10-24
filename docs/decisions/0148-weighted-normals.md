# 0148. Weighted normals, and custom normals

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack and its recipe), [0143](0143-subdivide-and-creases.md) and [0145](0145-decimate.md) (the object's Smoothing in `ModifierContext::hardness`), [0146](0146-scatter.md) and [0147](0147-curve-deform-and-shrinkwrap.md) (the latest modifiers), [0053](0053-hard-and-soft-edges.md), [0064](0064-automatic-hard-edges.md) and [0126](0126-rework-and-first-use.md) (hard edges and each object's Smoothing), [0043](0043-attribute-layers-and-the-builder.md) (attribute layers), [0073](0073-mikktspace-tangents.md) (MikkTSpace tangents), [0077](0077-bake-check.md) (the bake check), [0088](0088-live-mirror.md) (the live mirror), [0135](0135-live-booleans.md) (booleans), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0147](0147-curve-deform-and-shrinkwrap.md) (the development file format moves to version 49). Fills in DESIGN.md §4.1's corner layer "custom normal (for weighted normals)".

## Problem

Phase 10's tenth milestone is **Weighted normals**: a modifier, best last in the stack, with **Weight by** face area (recommended), corner angle or both, **Strength**, **Keep hard edges** (on) and **Face influence**, so a bevelled hard-surface shape's big faces shade flat while its bevels shade round. It is what game artists use instead of baking a normal map for low-polygon bevelled props.

The app had no custom normals. Every corner's normal was worked out when needed (`mesh::shading`, from the hard edges, or by angle with the object's Smoothing, decision 0126), in four places: the viewport's vertex buffers (`render::MeshGeometry`), exports (`fileio::export::export_mesh`, giving glTF `NORMAL` and OBJ `vn`), MikkTSpace tangents (computed in `fileio::gltf` from the exported normals), and the hard edges the bake check and Seams from hard edges see (`mesh::hardness::shading_hard_edges`). A modifier's result is a mesh, so the modifier needs a way to put normals *in* a mesh that all four read. Left to decide: how custom normals are stored, how they reach those four places, what happens when something after them changes the shape, the weighting itself, its settings, and saving.

## Options

How custom normals are stored:
1. **A corner attribute layer (chosen),** `custom_normal`: three numbers per polygon corner, the standard (no length) meaning "none set here". The attribute layers (decision 0043) already keep values per corner, share them between copies, carry them through the builder and save them (every layer since file format 5). A corner can be set or not, one by one.
2. A field of its own on `Mesh`: every operation, copy and file reader would need teaching about it.
3. Normals only in the stack's result, outside the mesh: Bake into shape couldn't keep them.

Where they are read:
1. **In `mesh::corner_shading_with` (chosen),** the one function every reader goes through: a corner with a custom normal shades with it; corners of one smooth group whose normals now differ are split into groups of their own (the renderer shares one vertex per group). The viewport, exports, tangents and `shading_hard_edges` all follow with no change of their own, except: the renderer's quick update of moved points (`MeshGeometry::can_follow`) builds afresh when there are custom normals (they are set, not worked out from the points round them), and `shading_hard_edges` lists the edges where custom normals part (`custom_normals::split_edges`).
2. Each reader checks for them: four places to keep in step.

What a later change does to them:
1. **Drop them (chosen).** A custom normal fits the shape it was made for. Once points move, polygons change or hard edges change, it no longer does, and the shading goes back to being worked out. This is what a game artist expects of a "finishing" step and is easy to explain: *Weighted normals goes last*.
2. Work them out again after every later step with the Weighted normals settings: Weighted normals would then act last wherever it is in the list, which makes the list's order mean nothing for it, and a deformer's own change of shading (a bend) would be weighted without the user asking.
3. Keep them as they are: stale normals after a deformer or a subdivision (a bent shape lit as if straight), a common complaint in other programs.

The weighting:
1. **Per smooth group, each polygon's facing weighted by its area, corner angle or both, raised by Face influence (chosen).** At a point, every polygon of the corner's smooth group (the object's Smoothing, when Keep hard edges is on; the whole point otherwise) adds its facing times `(its weight ÷ the group's biggest)^power`, `power = 1 ÷ (1 − Face influence)` (1 at 0%, up to 64 at 100%: in effect only the biggest count). Strength blends from the usual normal to the weighted one. Simple, deterministic and quick (one pass over the corners).
2. Blender's ranked weights ("weight 50", a threshold for equal weights): harder to explain and to give one setting for.

## Decision

### Custom normals (`mesh::custom_normals`, layer 0)

- `CUSTOM_NORMAL` (`"custom_normal"`), a `Domain::Corner`, `Vec3` layer, standard `Vec3::ZERO` (none). `has_custom_normals`, `custom_normal_list` (unit length or `None` per corner; values that aren't real directions count as none), `set_custom_normals`, `forget_custom_normals`, `shading_unchanged(before, after)` (the same points in the same places, polygons and hard edges), `split_edges`.
- `corner_shading_with` (and so `corner_normal_list`, `corner_normals`, `shaded_triangles`) takes them; `shading_hard_edges` lists `split_edges` when there are any, whatever the Smoothing.
- **Kept or dropped:**
  - recorded edits (`scene::stack::apply_from`): an edit after which `shading_unchanged` fails drops them; marking seams, unwrapping, materials and other changes that don't touch the shading keep them;
  - the live mirror (`mesh::mirror::with_copies`): dropped (the halves' shading joins at the seam);
  - booleans (`booleans::rebuild`): dropped (the layer isn't carried into the cut shape);
  - every modifier after the one that made them (`StackJob::run`, by `ModifierKind::carries_custom_normals`, false for every kind but Weighted normals; a later kind that only cuts polygons up, such as the triangulate preview, can carry them over and say yes);
  - `mesh::parts::append` (Join, Split off, scatter copies, the mirror's copies) turns them with the part (by the inverse transpose of its matrix).
- A modifier that fails passes its input on unchanged, custom normals and all.

### The Weighted normals modifier (`mesh::weighted_normals`, layer 0)

- `WeightedNormalsOptions` (`Copy`, `Default`, `clamped`): **Weight by** (`WeightBy::Area`, standard; `Angle`; `AreaAndAngle`), **Strength** (0–1, standard 1), **Face influence** (`influence`, 0–1, standard **0.75**: `power` 4), **Keep hard edges** (standard on). Values that aren't numbers become the standard ones.
- `weighted_normals(mesh, options, hardness)`: the input with custom normals on every corner (any it had replaced); nothing else changes. Polygons with no area count for nothing; a group with no area keeps its usual normal; an empty mesh comes back as it is. Never fails.
- With Weight by Corner angle and Face influence 0% the result is the usual shading.

### In the scene

`ModifierKind::WeightedNormals(WeightedNormalsOptions)`, family **Game ready** (its first kind), last on the palette and in Mesh → Modifiers, following decision 0142's recipe. It reads no other object, gives the viewport and exports the same, and its hard edges are those `ModifierContext::hardness` (the object's Smoothing) shows. `work` is the corners ÷ 15 (from 75 000 corners, about 19 000 quads, it is worked out on another thread). `Scene::final_weighted_normals(id, purpose)` gives its settings when its normals reach the final shape (it is the last modifier worked out for that purpose); `Scene::normals_dropped_by(id, index)` names the first modifier after it that is on (`scene/src/weighted.rs`). Bake into shape keeps the normals in the baked mesh.

### In the interface

- `CommandId::WeightedNormals` ("Weighted normals", its icon a bevelled box seen from the side, arrows straight up on its flat top and fanning round its corners, drawn in code) on the modifiers palette, in **Mesh → Modifiers** under *Game ready* and in command search. **No keys or mouse actions.**
- Its Attributes rows (`attributes/weighted_rows.rs`): **Weight by** (Polygon area, Corner angle, Area and angle), **Face influence**, **Strength** (percentages) and **Keep hard edges**. When a modifier after it is on, an amber note: "*Subdivide* comes after this and changes the shape, so these normals are dropped: move Weighted normals to the end with Down."
- **The bake check:** custom normals baked into a shape count where they part (through `shading_hard_edges`); a live Weighted normals whose normals reach the export with Keep hard edges off has no hard edges for the "hard edges that aren't UV seams" check (nothing for a normal map to hold there).

### Exports

Nothing new in `fileio`: `export_mesh` reads `corner_shading_with`, so glTF `NORMAL` and OBJ `vn` carry the custom normals, and MikkTSpace tangents are made from them. Tested by reading a `.glb` and an `.obj` back (the big faces' normals within 0.05° of their facing), a baked shape giving the same file as the live modifier, and a modifier after it giving the usual shading. `files_for_outside_checkers` writes `weighted-normals.glb` (a cylinder keeping its hard edges, a cube smoothed over), which the Khronos glTF Validator passes.

### Saving

Development file format **version 49**: version 48's model with `KindV43::WeightedNormals { weight_by, strength, influence, keep_hard_edges }` (weight as a number, anything unknown area), clamped on reading. Custom normals baked into a shape are its `custom_normal` corner layer, saved as every layer is; a layer of that name of another kind is ignored. New sample `sample-v49.avtl` (a standard modifier, one with every setting changed and hidden in the viewport, and a capsule with weighted normals baked in).

### Timings

A sphere of 200 000 polygons (800 000 corners): 0.21 s in a release build (the usual shading's work twice over, smooth groups and normals, then the weighting).

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** The palette button, **Mesh → Modifiers → Weighted normals** (under *Game ready*) and command search.
- Standard settings: **Weight by Polygon area**, **Face influence 75%**, **Strength 100%**, **Keep hard edges on**. Face influence 75% (rather than 50%) because at 50% a single 45° bevel still leans a 1 m face's corners by about a third of a degree; at 75% it is under a hundredth.
- **Modifiers after it drop its normals** (rather than working them out again or keeping them stale), with an amber note saying so and recommending it go last. The same for edits that move points or change polygons or hard edges after baking it in, the live mirror and booleans.
- **Keep hard edges** follows the object's **Smoothing** (by angle or by its marks), as Decimate's does.
- With Keep hard edges off, the bake check has no hard edges to check for that object.

## Consequences

- Projects with Weighted normals need this version of the app or later; a project with weighted normals baked into a shape opens in an older one, which shades it the usual way.
- Normals can now be stored per corner: an importer (FBX or glTF normals, Phase 11) or a later normal-editing tool can set the same layer and every reader follows.
- `ModifierKind::carries_custom_normals` is the switch for a later kind (Triangulate preview) that should keep them.
- Not yet: a per-polygon strength (Blender's *face strength*), a weight threshold for "equal" polygons, and editing normals by hand.
