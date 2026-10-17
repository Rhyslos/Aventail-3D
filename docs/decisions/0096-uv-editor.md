# 0096. The UV Editor, the UV mode and the UV Edit layout

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0095](0095-uvs-before-advanced-modelling.md) (the UV phase plan and its controls), [0029](0029-selection.md) (selection), [0008](0008-window-and-panels.md) (window and panels)

## Problem

UVs need a place to be seen: a flat view of the texture square beside the 3D view, a mode to work in, and a layout that puts the two together. DESIGN §5.1 and §7 plan a dockable UV Editor (C4D style), a UV mode in the left palette and a UV Edit layout on the menu bar's tabs. The controls were chosen in 0095: the editor moves and uses keys exactly like the viewport.

## Options

- **How the editor is drawn:**
  - **with egui, as flat meshes** (chosen): UVs are 2D lines and fills, a few thousand for a game asset. A cap (300,000 polygons) keeps heavy shapes from freezing the window;
  - with the GPU renderer, which can come later if needed.
- **Which objects:** **the selected ones with a shape** (chosen), as other programs do; an instance shows its original's UVs.
- **Selection:**
  - **one selection, shared with the viewport** (chosen, like C4D and Blender's sync mode): what is picked in either lights up in both, and every tool sees one selection;
  - a separate UV selection, which a learner must keep in mind twice.
- **What the UV mode works on:**
  - **polygons, a whole island at a time** (chosen): a click in the editor or on the model takes the island, Ctrl + click adds or takes one away. The Points, Edges and Polygons modes select single elements in the editor, as in the viewport;
  - UV points only, which needs the editing tools to be useful.
- **The UV mode's 3D gizmo:** **off** (chosen) until UVs can be edited (the next milestone), so W/R/S never move the shape while working on UVs.
- **Layouts:**
  - **the menu bar's tabs switch whole arrangements, each kept as it was left and saved apart** (chosen). UV Edit puts the viewport and the editor side by side and switches to the UV mode (from the Model or Object mode); Standard leaves the UV mode;
  - one layout with the editor always there, which crowds modelling.
- **The UV Editor panel in other layouts:** **optional, from Window → UV Editor** (chosen), beside the viewport. The UV Edit layout always has it.
- **Keys over the editor:** H (frame the texture square) and Z (frame the selection's UVs) go to the editor while the mouse is over it, to the viewport otherwise: the same keys, where you work (0095).
- **Seams in the viewport:** **orange lines, always in the UV mode, and in the element modes with Display → Seams** (chosen; hard edges stay cyan).

## Decision

1. **`uv_editor::UvEditor`:**
   - draws the texture square, its grid, and each shown object's UVs (`UvShape`: polygons, edges drawn once, island borders, islands), cached per evaluated shape;
   - tints selected polygons, and draws selected edges and points, in the accent colour, with what is under the mouse lighter;
   - picking: the polygon a point is inside (the smallest, where they overlap), the nearest edge or point within 9 points;
   - box select: points inside, edges with both ends inside, polygons with their middle inside;
   - all through `Document::select_elements`, one undo step each;
   - a note at the bottom: counts, or why nothing shows.
2. **The UV mode** is an element mode on polygons (`components::kind_for`). The viewport's selections in it are widened to whole islands (`selecting::to_islands`: an island that gained a polygon is taken whole, one that lost one is let go whole), and its 3D gizmo is off.
3. **Layouts:**
   - `LayoutChoice` (Standard, UvEdit);
   - `Workbench::switch_layout` swaps the two arrangements;
   - `layout()` and `uv_layout()` are saved under their own keys, and `with_uv_layout` and `restore_uv_layout` bring them back;
   - `layout::uv_edit_layout`;
   - `toggle_uv_editor` (Window menu);
   - `Panel::UvEditor`, in `Panel::OPTIONAL`, so a layout without it is complete.
4. **Seams:** `Guides::seams`, `OverlayStyle::Seam` (orange) and Display → Seams.
5. **The Keys window** says H and Z frame the UV Editor when the mouse is over it.

## Consequences

- The ready-made UVs (0095) can be seen and checked against the model, and parts found in either view.
- The next milestone puts the viewport's move, rotate and scale (with the same keys) on UVs in the editor and in the UV mode, and fills the UV mode's E menu.
