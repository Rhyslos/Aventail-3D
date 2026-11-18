# 0312. Remesh for sculpting (Phase 17a, milestone 5)

- **Status:** Accepted, built while the project lead was away; the plan's choices and the new ones below wait for review in [docs/plans/phase-17.md](../plans/phase-17.md#choices-to-review) (56 to 64 new)
- **Date:** 2026-11-18
- **Follows:** [0300](0300-sculpting-plan.md) (the sculpting plan), [0304](0304-sculpt-mode-and-layers.md) (Sculpt mode and layers), [0310](0310-sculpt-symmetry-and-masks.md), [0311](0311-sculpt-mode-looks-like-paint-mode.md), [0142](0142-modifier-stack-and-remesh.md) (the Remesh modifier), [0055](0055-copies-and-split.md) (Join)

## Problem

Sculpting's fifth milestone: **Remesh for sculpting** (the plan's choices 6 and 14). Block a creature out of overlapping primitives, select them, press Ctrl + R, and get one even, closed surface to sculpt; or even out polygons a big pull stretched. It must bake (not stay live like the Remesh modifier, which would throw sculpting away each time it ran), be one undo step, keep sculpted detail the voxels can hold, close open shapes first when asked, carry sculpt groups through when they come (17b.3), warn before a heavy result and refuse one too big before it runs out of memory, and work on millions of polygons in the background with progress. The app's remesher kept a value for every point of the grid round the shape, limited to 2 million points: a sculpt of a million polygons needs a grid a hundred times bigger.

## Options

1. **How the grid is kept.** Every point of the box round the shape (as before: memory and time grow with the box's volume); or only blocks of points near the surface, with each row's crossings telling which side any other point is on (memory and time grow with the surface).
2. **What limits a remesh for sculpting.** The grid's size (as the modifier); or the polygons it would make, worked out before it starts from the shape's area as seen along each axis.
3. **How detail is kept.** Fewer relax passes (rougher everywhere); or the evened-out surface laid back on the old one (each point moved to the nearest place on the triangles nearest its grid cell).
4. **Where the voxel size and switches live.** All in the brush bar; or the voxel size (and a Remesh button) in the bar and everything in a Remesh section of the Sculpt tab.
5. **What is baked.** The object's edit layers (the sculpt and what is under it), its booleans, mirror and modifiers staying after it; or everything including the modifiers.
6. **Merging other shapes.** As Join merges them (their edits worked out, their materials joined by name, the others deleted, anything inside them taking their place); or their modifiers baked in too.
7. **UVs.** The modifier's box projection; or none.
8. **A heavy result.** A warning in the bar only; or a question before running past 3 million polygons, and a refusal past 8 million.

## Decision

- **A sparse grid on every core** (option 1, the second way; `mesh::remesh::sparse`): the grid is cut into blocks of 8 × 8 × 8 points; only blocks within a step of a triangle hold values (a signed distance, its sign the side: `-0.0` is inside), worked out block by block on every core from the triangles near each (gathered once, in triangle order, so ties go as before). Which side any other point is on comes from its row's crossings (`field::Rows`, the winding count the modifier always used, now kept as a sorted list instead of a full grid). Corner-only touches are filled in from a list of the cells that need it (a fill reaching an unheld block holds it). Surface nets, relaxing and the mesh work block by block too. Distances are kept within one step of the surface (was one and a half): the new surface's points come only from grid edges it crosses, whose ends are at most a step from it. **The Remesh modifier uses the same code** (same polygons, block by block in another order), keeping its 2-million-point limit.
- **Remesh for sculpting** (`mesh::remesh::remesh_for_sculpting`): limited by the **polygons it would make** (option 2, the second way): about the shape's projected area (each polygon's area times how much it faces X, Y and Z, added up) over the voxel size squared, refused past **8 million** (`MAX_SCULPT_POLYGONS`) with the smallest voxel size that fits, and refused when the grid's table of blocks would pass 32 million (pieces too far apart). **Keep detail** (option 3, the second way) and **Fix holes** (each loop of open edges, found by point ids and then checked by position, closed with a fan of triangles round its middle) are on as standard; two relax passes. A label per polygon (sculpt groups, 17b.3) is carried to the new polygons from the nearest. `Progress` says how far it has got and stops it. **No UVs** (option 7, the second way).
- **The command** (`Command::RemeshForSculpting`): the new shape becomes the object's base; its edit layers go (option 5, the first way) and a fresh, empty Sculpt layer starts on top; its booleans, mirror and modifiers stay; the other shapes merged are deleted (option 6, the first way: `Scene::joined_shape`, shared with Join, which now uses it); one undo step, refused for groups, instances and curves.
- **The interface:** **Ctrl + R** in Sculpt mode, the Sculpt menu's first entry, the Sculpt tab's new **Remesh** section and a **Remesh** button on the brush bar beside the **Voxel size** (option 4, the second way); the voxel size is in the world's centimetres (2 cm as standard, 0.05 to 50), shown for 1.5 s after a change as an 8 × 8 grid on the shape where the view looks; about how many polygons it makes is in the bar's hint and the section (each selected shape's area worked out once, a heavy one's on another thread). It runs on another thread with a progress window at the bottom of the view and **Cancel** (Esc); strokes and the mask's actions wait; a result whose shapes changed meanwhile (an undo) is dropped with a note. Past **3 million** polygons it asks first (option 8, the second way).
- **No new file format:** the remesh's settings are the interface's, like the brushes'; the new shape and the empty Sculpt layer are saved as before.

## Consequences

- Remeshing costs what the new surface costs: on two shared cores, the heavy test shape (3 million polygons) remeshes to 1 million in about 6 s and to 3 million in about 10 s (the targets are 2 s and 5 s on an ordinary machine; 17a.6 measures and speeds it up). The modifier is faster for big grids too.
- Remeshing throws away the mask, UVs, the edit layers' separate edits and a primitive's settings: one undo step brings them back.
- **Keep sculpt groups** is built (labels carried per polygon) but has no switch until sculpt groups exist (17b.3).
- Detail levels (17b.1) will rebuild lower levels from a remeshed shape.
