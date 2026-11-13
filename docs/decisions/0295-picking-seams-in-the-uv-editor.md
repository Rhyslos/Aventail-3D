# 0295. Picking seams in the UV Editor

- **Status:** Accepted
- **Date:** 2026-11-13
- **Follows:** [0293](0293-blend-seams.md) (Blend seams and the Seam blend layer), [0180](0180-painting-in-the-uv-editor.md) (painting in the UV Editor), [0028](0028-ask-about-controls.md) (asking about controls)

## Problem

Blend seams (milestone 1) picked seams only on the model: a click in the UV Editor said to click in the 3D view. The project lead asked on 2026-11-13 for the seam features left out, this one first. Picking on the texture helps when a seam is hard to reach on the model (inside a sleeve, under an arm), and when the texture shows which pieces don't match. This is milestone 3 of the [seam blending plan](../plans/seam-blending.md).

## Options

- **Controls:** the gestures already chosen for the 3D view (click marks in place of those marked, Shift + click marks more or unmarks a marked one), and the same **Every seam that shows** and **Unmark all** buttons. No new keys or mouse actions.
- **A seam is two lines on the texture**, one along each piece it joins. (a) Pick only the side nearer the mouse and light it alone: the user can't see where the other side is, which is the point of a seam. (b) **Either side picks the seam, and both sides always light together** (chosen): pointing at one shows which piece it meets; marked seams show in the accent colour on both pieces.
- **Lighting across views:** pointing at a seam on the model lights both its lines in the UV Editor, and pointing at one in the UV Editor lights the line on the model, so the two views explain each other.
- **Which seams:** the UV Editor shows the selected objects, so their seams show and pick there (the 3D view still shows every object's). The material tried first is the one of the piece under the mouse, then the one the Paint layers panel shows (the mouse is often just outside a piece, on its edge); the faint lines are the panel's material's, as on the model.
- **Speed:** the faint lines on the texture are worked out once per drawing, material, marks and objects shown, as the 3D view's are; the seams themselves are the ones already kept per shape (`seams_of`). Only the line under the mouse is found each frame: the seam sides within 10 screen points of the mouse, on the objects shown.

## Decision

- `uv_editor/seam_lines.rs`: `UvSeamLine` (a side's two UVs and its look), `UvSeamLines` (all, hovered), `UvEditor::set_seam_lines`, drawn over the UVs in Paint mode (`paint_seams`): faint, accent when marked, white under the mouse, the same as the 3D view's.
- `workbench/seam_picking.rs`: `pick_flat_seam` (the nearest seam side within reach, its whole line as for the 3D view), `flat_seam_lines` and `flat_seam_shapes` (the objects the UV Editor shows), `Picked::flat` (both sides of the line picked in the 3D view, lit in the UV Editor), the UV Editor's clicks marking through `mark_seams` (one `SetTextureSet` undo step, the blend joining it), and `NO_SEAM_ON_TEXTURE` for a click away from the lines.

## Consequences

Seams can be marked from whichever view shows them best, with the same clicks. Nothing new is saved, so the file format is unchanged. While the UV Editor isolates the pieces holding the selection, the seam lines of the pieces hidden still show (a later polish could hide them). The seam blend brush still strokes only on the model (milestone 2).
