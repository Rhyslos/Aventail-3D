# 0161. Triangles: Show triangles and the Triangulate modifier

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack and its recipe), [0148](0148-weighted-normals.md) (custom normals, and `carries_custom_normals` for "a later Triangulate preview"), [0149](0149-live-bevel.md) (`goes_before`), [0150](0150-quadify.md) (Quadify, the opposite), [0057](0057-after-phase-4-testing.md) (the one cut every reader uses: `mesh::triangulate`), [0053](0053-hard-and-soft-edges.md) and [0151](0151-collision-hulls.md) (the Display menu's guides, Show collision), [0043](0043-attribute-layers-and-the-builder.md) (values carried to new polygons), [0028](0028-ask-about-controls.md) (ask about controls), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0160](0160-kit-helpers.md) (the development file format moves to version 62)

## Problem

Phase 10's twenty-third and last milestone is the **Triangulate preview**: a viewport switch, **Show triangles** (Display menu and Settings → Viewport display), drawing as faint lines the triangulation the export will use, with the export's own code; and a **Triangulate** modifier for those who want it baked, with a method **Beauty** or **Fixed**, and quads only or everything.

A game engine draws triangles. Every quad and n-gon is cut on the way out (glTF always, OBJ with *Cut polygons into triangles* ticked). A bent quad folds along the diagonal the cut picks; a flat n-gon cut as a fan has slivers; and a bake must use the same triangles as the game. Until now the cut was invisible.

How the export cuts today: `fileio::export::export_mesh` calls `mesh::face_triangles` for each polygon of four or more corners when triangulating; the viewport's renderer (`render::mesh_draw`, `mesh::shaded_triangles`) and picking call the same function (decision 0057). Triangles stay; a quad is cut 0–2 unless that runs outside a dented quad (then 1–3); a flat polygon without dents as a fan from its first corner; anything else laid flat and cut along the shortest lines inside it (up to 32 corners, which follows a bent polygon's fold) or ear by ear.

Left to decide: what *Fixed* and *Beauty* mean, which polygons *Quads only* cuts, ids, what is kept (UVs, materials, marks, custom normals, shading), where a Triangulate goes in the list, what happens with nothing to cut, what Show triangles draws and for which objects, and saving.

## Options

What *Fixed* means:
1. **Exactly the export's cut (chosen),** `mesh::face_triangles`: the modifier then changes nothing an export would get, and Show triangles draws it. The notes ask for "the same triangulation the export uses".
2. Blender's *Fixed* (always the same diagonal): a second rule beside the export's, which would make the preview and the modifier disagree.

What *Beauty* means:
1. **The export's cut, then Lawson's flips on flat parts (chosen):** each line between two triangles turns to the other diagonal of the four-sided shape they make when that widens the smaller of their smallest corners (by more than 0.0001 radians, so equal choices keep the export's), the new triangles both turning the polygon's way with some area (so no dent at either end of the old line, and a corner of 180° where a sliver had no area is fine), and only when the two triangles lie within **1°** of flat with each other. Round after round until nothing turns (at most 64). Deterministic, quick, and the standard way to even out triangles (a Delaunay triangulation of the flat polygon, as near as its sides allow). A bent polygon keeps the export's fold, so Beauty never changes the shape.
2. Beauty on bent polygons too: it would change the surface (a fold one way or the other is a different shape), against "the export as you see it".
3. Blender's polyfill-beautify: the same idea, with its own weighting; ours is simpler to explain ("the thinnest corner made wider").

Where a new Triangulate goes, and where others go relative to it:
1. **At the end, and every other kind added later goes before it (chosen):** it is the finishing step, cutting the shape a game gets; a Subdivide or Live bevel added after it would work on triangles, which they handle worst. One rule in `goes_before`: `other` is a Triangulate and `self` isn't.
2. Only the kinds that read polygons go before it: a longer list for no gain (a deformer works the same either side).

With nothing to cut (all triangles, or Quads only and no quads):
1. **Refused with a reason, shown as it is (chosen),** as Quadify does with nothing to join: the amber "Not worked out: every polygon is a triangle already." tells the person why nothing changed.
2. Pass the shape on silently: a Triangulate that does nothing would look broken.

Which objects Show triangles draws:
1. **Every shown shape (chosen),** selected or not, as a display switch like the Wireframe look: it's for looking over a scene before export. Cutters (outlines) are left out (they go out as part of what they cut), and so is any shape over 50 000 polygons (`GUIDES_UP_TO`, as every guide line), which draws line by line.
2. The selected objects only: the collision outlines do this when the switch is off, but "Show triangles" with no selection would show nothing, which reads as broken.

## Decision

### The work (`mesh::triangulate`, layer 0)

- `inner_lines(mesh)`: the lines inside polygons where `face_triangles` cuts them (not their sides), in the mesh's space, each once. Show triangles draws these.
- `TriangulateOptions` (`Copy`, `Default`, `clamped`): **Method** (`TriangulateMethod::Beauty`, standard, or `Fixed`) and **Quads only** (standard off: every polygon with four or more corners). Every value is valid, so `clamped` is the identity.
- `polygon_triangles(mesh, corners, method)`: `face_triangles`, then for Beauty `beauty::beautify` (above).
- `triangulated(mesh, options) -> Result<Mesh, TriangulateProblem>`: each cut polygon's **first triangle keeps its id** (`Builder::rewrite_faces`) and the others are new, in polygon order (`Builder::face`, like the polygon): ids are deterministic. Points never move or change, so every edge keeps its values (hard, seam, crease, bevel weight); each triangle's corners copy the polygon's corner values (`CornerFrom::Copy`: UVs, custom normals and every other corner layer) and its polygon values (material). The new inside lines carry nothing. Refused: `Empty` (no polygons), `AllTriangles`, `NoQuads` (Quads only, with n-gons but no quads), each a reason in plain words starting with a small letter.
- **Shading:** smooth normals are weighted by corner angle (`corner_shading_with`), so a flat polygon cut into triangles shades exactly as before (its corners' angles add up to the polygon's, with the same facing). A bent polygon shades a little differently as two triangles, as it does in a game.
- Never panics: zero-length edges, every point in one place, open borders, edges of three polygons, huge coordinates (tested). Deterministic.
- `corners_to_cut(mesh, options)`: the modifier's work.
- Timing (release build): a sphere of 100 000 polygons into 199 200 triangles in 69 ms (Fixed) and 86 ms (Beauty); its 99 200 inside lines in 5 ms.

### The modifier

`ModifierKind::Triangulate(TriangulateOptions)`, family **Game ready**, after Weighted normals on the palette and in Mesh → Modifiers (so the last kind; the Game bevel button stays after it), following decision 0142's recipe. It reads no other object and gives the viewport and exports the same. `work` is the corners to cut ÷ 20 (about 25 000 quads and more are worked out on another thread). It **carries custom normals** (`carries_custom_normals`) and **keeps the shading** (`keeps_shading`, new): `Scene::final_weighted_normals` and `normals_dropped_by` look past a Triangulate, so Weighted normals before it still reach the export, and the bake check, without an amber note. Every other kind added later goes before it (`goes_before`). Bake into shape keeps the triangles.

### Show triangles

`ViewOptions::show_triangles` (off as standard, remembered with the settings), a checkbox **Show triangles** in the viewport's Display menu after Show collision, in Settings → Viewport → Viewport display, and command search (`MenuAction::ShowTriangles`, under *Display*). The interface's component tracker (`ui/src/components/triangles.rs`) keeps each drawn object's `inner_lines` while its drawn shape is the same `Arc` (moving the object only moves them) and draws them under every other guide in a new `OverlayStyle::Triangle`: light grey-blue at 30% opacity, 1 pixel wide (`app/src/gpu.rs`). They come from the shape the viewport draws (`DrawItem::drawn`, after modifiers), which the renderer cuts the same way, so the lines lie exactly on the drawn triangles' edges. The Display menu grows by one line, well inside a 900-pixel-high window.

### In the interface

`CommandId::Triangulate` ("Triangulate"; its icon a six-sided polygon cut into four triangles, one in its middle, drawn in code) on the modifiers palette (which scrolls, decision 0159), in **Mesh → Modifiers** under *Game ready* and in command search. Its Attributes rows (`attributes/triangulate_rows.rs`): **Method** (a list: Beauty, Fixed, each with its explanation) and **Quads only**. **No keys or mouse actions.**

### Exports

Nothing new in `fileio`: the exported shape is the modifier's result. Tested: with *Fixed*, a glTF-style export (triangulating) gives exactly the same triangles, places, normals and UVs as the plain shape; the triangles go out as triangles even when the export doesn't triangulate (Quads only keeps a cylinder's caps); Weighted normals followed by a Triangulate exports the same normals as Weighted normals alone; baked, the export is the same.

### Saving

Development file format **version 62**: version 61's model with `KindV43::Triangulate(TriangulateV62 { method, quads_only })` (method as a number, anything unknown read as Beauty). New sample `sample-v62.avtl` (a cylinder with Weighted normals and a standard Triangulate after it, and a cylinder with a Fixed, Quads only Triangulate hidden in the viewport); files before 62 never hold it. Show triangles is a setting, saved with the other viewport options (no file format change).

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** Show triangles: **Display → Show triangles**, Settings → Viewport display and command search. Triangulate: the palette button, **Mesh → Modifiers → Triangulate** (under *Game ready*) and command search.
- **Show triangles** is off as standard and shows **every shown shape's** triangles, not only the selected ones'; shapes over 50 000 polygons are left out (as all guide lines).
- The lines are **faint grey-blue, 1 pixel, 30% opaque**, under every other guide.
- **Beauty** is the standard method (Blender's default for n-gons); **Fixed** is exactly the export's cut. Beauty only turns lines inside flat parts (within 1°), so it never changes the shape.
- **Quads only** is off as standard.
- A new Triangulate goes at the **end**, and every other modifier added later goes **before** it.
- It **keeps Weighted normals'** custom normals (they reach the export), as decision 0148 foresaw.
- With nothing to cut, it shows the shape as it is with an amber reason, as Quadify does.
- No one-off "Triangulate now" command: Bake into shape makes the modifier permanent. (Connect cuts a polygon by hand.)

## Consequences

- Projects with a Triangulate modifier need this version of the app or later.
- `ModifierKind::keeps_shading` is the switch for any later kind that only regroups polygons without changing the shading.
- Phase 10 is complete (23 of 23).
- Not yet: a "Triangulate now" one-off edit; Beauty for bent polygons (choosing the flatter fold); a choice of which diagonal *Fixed* takes for quads (Blender's *shortest diagonal*); showing the triangles in the UV Editor; a minimum corner count other than four.
