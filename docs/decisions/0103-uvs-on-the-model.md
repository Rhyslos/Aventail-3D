# 0103. Editing UVs on the model

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0095](0095-uvs-before-advanced-modelling.md) (the UV phase plan), [0097](0097-editing-uvs-by-hand.md) (editing UVs by hand), [0102](0102-checker-and-stretch.md) (the Checker view), [0028](0028-ask-about-controls.md) (ask about controls)

## Problem

Many small UV fixes are easier to judge on the model than on the flat texture square: slide a checker so a panel line falls on a square's edge, turn wood grain along a plank, make a piece's texture a little bigger. The roadmap asks for moving, turning and scaling a texture by dragging it on the surface, and for an island under the mouse in 3D to light up in 2D and the other way round.

## Options

- **Controls** (a new mouse action, so decision 0028 says to ask; the project lead wasn't available, so the recommended choice was used and the pull request says so):
  - **the same keys as everywhere, W, R, S and T, then drag anywhere in the viewport while in the UV mode** (chosen, recommended): matches "all keys relatively the same" (decision 0095's answers) and the viewport's own drag-anywhere tools. In the UV mode the 3D gizmo is already hidden, so nothing clashes;
  - a gizmo on the surface: one more thing to aim for, and it would hide the texture being judged;
  - a modifier (Alt + drag): clashes with navigation schemes that use Alt.
- **Which way things move:** **the texture follows the mouse** (chosen): you see the texture, so a drag right slides it right, which means the UVs move the other way. Scaling right makes the texture look bigger (the UVs shrink). The other way round would read as backwards on the model.
- **Turning a screen drag into a texture move:** **a least-squares fit from where the selected islands' corners are on screen to where they are in the texture** (chosen): one 2×2 map, good for the surface in view, worked out once at the start of the drag (up to about a few thousand corners). Picking the surface under the mouse each frame would jump at polygon edges.
- **What is moved:** the selected islands, as in the UV Editor; a press on an island that isn't selected selects it first, so one drag is enough.
- **Lighting up:** the polygon under the mouse in the UV Editor lights up on the model, and the one under the mouse on the model lights up in the editor (both already shown in the editor's island selection).

## Decision

1. `ui::workbench::uv_on_model`: `edits_uvs_on_model` (UV mode with Move, Rotate, Scale or All), `UvModelDrag` (targets, pivot, the fitted `to_texture` map), `fit`, `carry_out_uv_model_drag`. Move: the UVs move by `-(to_texture × drag)`; Rotate: 0.01 radians per point, the way set by the fit's handedness, snapping to the snap angle; Scale: e^(0.005 per point), UVs scaled by its inverse, snapping to the scale step. Shift is ten times finer (the viewport's tool drags), Esc or a right click cancels. Recorded with `uv_editor::gizmo::record` (one `Operation::TransformUvs` per object), shown live and one undo step under `ON_MODEL_KEY`.
2. `ComponentTracker::set_outside_hover` lights up on the model what the UV Editor has under the mouse (`UvEditor::under_mouse`); `UvContext::outside_hover` does the reverse.
3. The UV mode's status note says the drags work in the editor or on the model.

## Consequences

- No new keys and no file format change: the drags make the same operation as the UV Editor's gizmo.
- On a strongly curved selection the fit is an average, so the texture follows the mouse best on the part facing you. Selecting fewer islands makes it exact.
