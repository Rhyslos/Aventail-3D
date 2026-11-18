# 0310. Symmetry and masks for sculpting (Phase 17a, milestone 4)

- **Status:** Accepted, built while the project lead was away; the plan's choices and the new ones below wait for review in [docs/plans/phase-17.md](../plans/phase-17.md#choices-to-review) (42 to 47 new)
- **Date:** 2026-11-18
- **Follows:** [0300](0300-sculpting-plan.md) (the sculpting plan), [0304](0304-sculpt-mode-and-layers.md) (Sculpt mode), [0306](0306-sculpt-brushes-for-forms.md), [0308](0308-sculpt-pens-strokes-alphas.md), [0187](0187-mirror-painting.md) (mirror painting)

## Problem

Sculpting's fourth milestone: sculpt one side and see both (**symmetry**, across the shape's own planes and round an axis), **mask** parts to hold them still (the Mask brush, box and lasso masks, invert, clear, grow and shrink, sharpen and soften, masks from the creases, the open edges and the facing), move, turn and scale the unmasked part with the gizmo, **Make symmetrical** and **Make a shape from the mask**, and the brush's circle drawn on the surface with its reflected copies. The plan sets the keys; building it needed a few more choices.

## Options

1. **Where the mask is kept.** In the Sculpt layer's sculpt beside the moves (one undo step and file entry with the sculpting, fitted to a changed shape by position the same way); or as a separate per-object store (its own undo, saving and fitting).
2. **How symmetric copies of a dab are put together.** Each copy applied in turn (simple, but where they overlap on the middle line the second copy works on the first's result, so a symmetric shape drifts); or every copy worked out from the shape as it was before the dab, and each point moved by the sum of what the copies give it (stays symmetric).
3. **What the box and lasso masks catch.** Every point inside the outline on screen, the far side too (as other programs do as standard); or only the points seen.
4. **Where the Sculpt menu goes.** A menu shown in the menu bar in Sculpt mode only; always shown; or its entries in the Mesh menu.
5. **How thick a shape made from the mask is.** A setting in the Sculpt tab (1 cm as standard, as the Shell modifier); or a fixed share of the shape's size.
6. **How the mask is drawn.** Darker over the matcap, the mask carried to the GPU in each vertex's colour alpha (only for the sculpted shape, only in Sculpt mode); or a separate overlay drawn over the shape.

## Decision

- **The mask lives in the sculpt** (option 1, the first way): `sculpt::Mask`, one byte a point (0 free, 255 fully masked) in the same chunks of 16 384 points, shared between copies, so a mask stroke's undo step keeps only the chunks it touched. `Sculpt::with_mask`; `fitted_to` gives each new point its nearest old point's mask. Every mask change is a `Command::Sculpt` (a Sculpt layer is added for a first mask). Masking doesn't change the shape.
- **Masked points move by how free they are** under every brush (the grabbing ones and Elastic grab from where each point is) and the gizmo.
- **The Mask brush** (`BrushKind::Mask`, **9**, full strength as standard) adds mask by the brush's falloff and alpha; Ctrl takes it away. **9 again** goes to the **box mask**, then the **lasso mask**, then back; Ctrl while drawing unmasks. Box and lasso catch **every point inside the outline, through the shape** (option 3, the first way).
- **Whole-mask actions** (`sculpt::mask`): Invert (**Ctrl + I**), Clear (**Alt + M**), Grow and Shrink by one ring of points (**Ctrl + wheel** over the view while anything is masked, and the menu), Sharpen (values pushed away from a half), Soften (each point averaged with its neighbours), Mask from cavity (how far a point dips below its neighbours' middle against its edges' length; a tenth is fully masked), from the open edges, and what faces up, down or the view (fully masked within about 45°, fading to free square to it). Each is one undo step, in the new **Sculpt menu** (shown in Sculpt mode only, option 4, the first way), the Sculpt tab's **Mask** section and command search.
- **Symmetry** (`sculpt::symmetry`): **L** turns it on; **X**, **Y**, **Z** (X as standard) reflect across the shape's own planes through its pivot, every combination of the chosen axes; **Radial** repeats each dab 2 to 64 times round a chosen axis (with the mirrors, every combination). **Every copy is worked out from the shape before the dab and the moves added** (option 2, the second way), so a symmetric shape stays symmetric. A reflected copy turns its alpha over and turns it the other way, landing as the mirror image. The Mask brush mirrors too. The grabbing brushes hold a lump on each side.
- **Make symmetrical** (+ side to −, and − to +, across the first chosen axis) copies each point's place and mask from its partner; a point on the written side with no partner at its reflected place on the shape under the sculpt (within a ten-thousandth of the shape's size) stops it with a plain message, saying strokes still mirror in space.
- **The gizmo with a mask:** in Sculpt mode, while anything is masked, **W, R, S, T** show the gizmo at the unmasked part's middle (each point counted by how free it is), on the object's own axes; dragging moves, turns or scales the unmasked part, partly masked points partly, as one undo step (Esc cancels). Without a mask the gizmo moves the object, as before.
- **Make a shape from the mask:** the polygons whose corners are all more than half masked become a new object with the Sculpt tab's **Thickness** (1 cm as standard, option 5, the first way) through the Shell modifier's work, placed where the sculpt is, with its materials, as one undo step.
- **Drawing:** the mask is darker over the matcap (to 35%), carried in the sculpted shape's vertex colour alpha (`render::MeshGeometry::shade_points`, `SurfaceDraw::Matcap::masked`), only the vertices of points whose value changed uploaded (option 6, the first way).
- **The brush circle on the surface:** over the shape, the brush is drawn lying along the surface where the mouse is, with an inner ring where the falloff is down to half, and a dashed circle where each symmetric copy goes; off the shape, flat on the view as before.
- **File format 114** (`fileio::project::v114`): each sculpt's masked chunks, one byte a point, packed with zstd; older files hold no mask. Version 113 is scripting's (decision 0307).

## Consequences

- One undo step per mask change or symmetric stroke; a mask stroke on a heavy shape keeps only the chunks it touched.
- A symmetric stroke costs a dab per copy plus putting them together; radial with mirrors on all three axes is up to 512 copies (slow on a heavy shape, but it works).
- The mask is redrawn by comparing the whole mask each time it changes (about 3 MB on the heavy shape); 17a.6 measures it.
- Make symmetrical needs a shape symmetrical in position; a shape remeshed off-centre can't be made symmetrical until 17a.5's remesh is symmetric.
- Mask from a sculpt group waits for sculpt groups (17b.3).
