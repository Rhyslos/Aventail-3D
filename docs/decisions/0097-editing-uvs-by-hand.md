# 0097. Editing UVs by hand: the flat gizmo, turns and flips, and the UV ring

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0095](0095-uvs-before-advanced-modelling.md) (the UV phase plan, its controls, and the rule that keys stay the same everywhere with the E menu in every area), [0096](0096-uv-editor.md) (the UV Editor), [0030](0030-transform-gizmos.md) (the gizmos), [0037](0037-tool-menu-and-command-list.md) (the tool menu), [0032](0032-edits-find-their-points-again.md) (anchors)

## Problem

The UV Editor shows UVs but can't change them. Every UV workflow starts with moving, turning and scaling pieces by hand, and the project lead's rule (0095) asks for the same keys as the viewport and an E menu with the tools for where the mouse is. A change to UVs must also be an edit like any other: one undo step, recorded on the object's stack so it holds when earlier edits change, saved in the file.

## Options

- **How UVs are moved:**
  - **the viewport's gizmo, laid flat** (chosen): W, R, S and T pick the same tools; in the editor they show arrows (red across, blue up, the viewport's X and Y colours), a ring and boxes. The same modifiers: Shift ten times finer, snapping from the palette button or Ctrl, Esc or a right click cancels;
  - Blender's "press G and move the mouse": a second way of moving things, which the rule (same keys, same feel) argues against.
- **Which UVs a change moves:**
  - **shared location** (chosen): the selected polygons' corners (or the selected points', or the selected edges' ends'), and every other corner of the same points sitting at the same spot in the texture, so an island never tears where its polygons meet; corners of a point at another spot (across a seam) stay. This is what other programs call "sticky selection: shared location";
  - corners of the selection only, which tears islands apart at every move.
- **What the UV mode moves:** whole islands, as it selects them (0096). In the Points, Edges and Polygons modes, single elements, selected in the editor.
- **How the change is recorded:**
  - **one edit, `Operation::TransformUvs`** (chosen): polygons or points, a 2D matrix (`glam::Affine2`, the texture's units) and anchors, so the edit finds its elements again when earlier edits change (0032). A drag shows itself live by taking back last frame's step and recording the new one under its merge key, as the 3D gizmo does;
  - writing the UV layer straight into a plain mesh, which only works on plain meshes and breaks the edit stack (0006).
- **Snapping:** moves put the gizmo's middle on the texture square's tenths (the grid drawn), turns step by the snapping angle and scaling by the snapping part, all from the Snapping settings. Snapping to texture pixels comes with the texture size (a later milestone).
- **Quarter turns and flips:** **Turn left 90°, Turn right 90°, Flip left to right and Flip upside down** (chosen), about the selection's middle, from the E menu. No keys: none were asked for (0028), and the project lead can give them keys later.
- **The E menu over the editor:**
  - **the UV ring** (chosen): with the mouse over the UV Editor in the Points, Edges, Polygons or UV mode (and always in the UV mode), E shows the UV tools. The gizmos and Frame selected keep their places from the object ring; seams at the right, unwrapping at the bottom, packing and the turns and flips at the left. Tools still to come (seams, unwrap, relax, straighten, pack) show greyed with when they arrive, so the ring doesn't move as they're built. The ring is chosen when the menu opens, so it never changes under the mouse;
  - the mode's own ring everywhere, which puts Extrude over a flat texture.
- **Frame selected from the UV ring** frames in the editor (as Z does there).

## Decision

1. **`mesh::uvs`:** `corners_moved` (shared location), `transform_uvs` (an `edit::Outcome`, nothing changed without UVs or with unreal numbers), `uv_bounds` and `uv_centre`.
2. **`scene::Operation::TransformUvs { faces, points, transform, anchors }`**, described "Move the UVs of N polygons/points"; it isn't mirrored by symmetry (a texture has no mirror plane in the model's space).
3. **File format version 28** (`project/v28.rs`): `OperationV28::TransformUvs` (six numbers, checked to be real) beside version 26's operations; `ProjectV27` is generic over its entries so `ProjectV28 = ProjectV27<EntryV28>`. A sample is kept in `tests/data/sample-v28.avtl`.
4. **`ui::uv_editor::gizmo`:** `UvHandle`, `hit`, `paint`, `GizmoDrag` (follows the mouse with Shift's fine steps, snaps, shows live and cancels under `UV_GIZMO_KEY`), `uv_targets` (what a change moves, per object, with anchors) and `record`. A press on a handle neither selects nor navigates. Esc is taken by the workbench while a UV drag is on.
5. **Commands:** `UvTurnLeft`, `UvTurnRight`, `UvFlipU`, `UvFlipV` (built), `MarkSeam`, `ClearSeam`, `Unwrap`, `Relax`, `Straighten`, `Pack` (arriving); `commands::UV_RING`; `menu_for(Mode::Uv)`. Move, Rotate, Scale, all gizmos and Frame selected list the UV mode among theirs. `Workbench::uv_menu` remembers, when the menu opens, whether the mouse is over the editor; `menu_mode()` picks the ring; `change_uvs` records turns and flips as one step.
6. **The UV Editor counts as under the mouse only if it was drawn in the last frame** (`shown_pass`), so a put-away editor no longer takes H and Z where it used to be.

## Consequences

- Islands can be laid out by hand, live, with undo, and the result is saved and survives earlier edits changing.
- The UV ring is in place for the next milestones: each fills its greyed places without moving the others.
- The same pattern (a `uv_targets` set and one recorded operation) carries the coming UV tools.
