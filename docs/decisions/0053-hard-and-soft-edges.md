# 0053. Hard and soft edges, by hand or by angle; hard edges and normals shown from the Display menu; file format version 11

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0028](0028-ask-about-controls.md) (ask about controls), [0037](0037-tool-menu-and-command-list.md) (the command list), [0052](0052-clean-up.md)

## Problem

Shapes have hard edges since the primitives (a cube's corners are hard, a cylinder's sides soft), but there was no way to change them, and no way to see them, or to see which way polygons face.

## Decision

1. **Hard edges and Soft edges** (tool menu `E` and Mesh menu; no keys, as the project lead chose).
   - In the Edges mode they mark the selected edges; in the Polygons mode, every edge of the selected polygons (so softening a group of polygons smooths them into one surface, and hardening them outlines them).
   - Only edges that change are recorded; if none would, nothing is recorded and the status bar says they already are.
   - Hardness is only shading: the shape doesn't change.
2. **Hard edges by angle** (Mesh menu): in each selected object, every edge whose two polygons meet more sharply than an **Angle** becomes hard and every other edge between two polygons soft. Other programs call this auto-smooth.
   - Angle starts at **30°**: a box's corners come out hard, and curves of 12 or more sides soft. It shows as the Last edit, and the angle used last is used next time.
   - Edges with three or more polygons count as sharp. Border edges are left as they are (shading never flows across them anyway). Polygons with no area count as soft.
   - Worked out from the shape as it is, so it follows changes underneath.
3. **Select hard edges** (Select menu): selects every hard edge, in the Edges mode.
4. **Seeing them** (the viewport's Display menu, as the project lead chose):
   - **Hard edges**, on as standard: in the Points, Edges and Polygons modes, the edited objects' hard edges are drawn in light cyan, a little thicker than the plain edges.
   - **Normals**, off as standard: each polygon of the selected objects shows a short violet line from its middle, the way it faces, half as long as the polygon is across. In every mode, so flipped polygons can be spotted while modelling.
   - Both are drawn for objects of up to 50,000 polygons; heavier ones would slow the view down.
   - Both are remembered with the view's other choices.
5. **The edits:** `Operation::SetHardEdges` (edges, hard or soft, anchors) and `Operation::HardEdgesByAngle` (the angle).
6. **File format version 11.** Version 10's operations plus these two. Edges must join two different points; the angle must be a real number, and is brought within 0–180°.

## Consequences

- Every modelling command in the tool menu is now built.
- Weighted normals, and showing each corner's shading normal, are left for later.
- The hard edge colour is fixed, not the accent colour, so it never looks like a selection.
