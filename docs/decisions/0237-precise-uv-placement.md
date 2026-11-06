# 0237. Precise UV placement: snapping, pivots, exact numbers

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 9. Moves in the UV Editor could only snap to tenths of the square, the gizmo always worked about the selection's middle, and nothing could be typed, so pieces couldn't be butted together, sat on trim rows or set to an exact pixel size.

## Decision

- **Snapping** (the project lead's choice: as the viewport, the magnet button with Ctrl flipping it, and a Snap menu in the UV Editor): `uv_editor::placing::UvSnap` (a grid of your own size, standard on at 0.1; texture pixels; other pieces' corners; other pieces' edges; trim rows). As a drag begins, `placing::gather` collects the corners that move and the corners and edges that stay; `UvSnapping::snap_move` snaps the move so a moving corner lands on the nearest corner, edge or row within 10 screen points, otherwise puts the selection's middle on the grid (or a pixel when the grid is off). One-way moves keep their way. Turns and scales snap as before.
- **Pivots** (a Pivot menu in the UV Editor; **Ctrl + click** places your own point, chosen by the project lead): `UvPivot` (the selection's middle, each piece's own middle, the four corners of the selection's box, the texture's middle, your own point). With each piece, `placing::per_piece` splits the targets by UV piece and the drag records each turn or scale about its piece's middle. While your own point is chosen, Ctrl + click places it instead of adding to the selection.
- **Exact numbers**: an **Exact** menu in the UV Editor (the selection's middle and size in pixels or 0 to 1, and a typed turn about the pivot), each change one `TransformUvs` step. Changed from the plan (the Coordinates panel): the panel's fields are the viewport's in centimetres and degrees; a menu beside Snap and Pivot keeps UV placement in one place.
- No file format change: everything records `TransformUvs`. The menus' choices last for the session.

## Consequences

- Ctrl + click means something else while Pivot → Your own point is chosen; the manual says so.
- Snap targets are gathered once per drag, so a drag over many pieces stays quick (up to 4000 moving corners are tried).
- Grid snapping lands the gizmo's pivot on the grid (the selection's middle as standard, or the Pivot menu's choice), and one drag of a number in the Exact menu is one undo step. (Clarified after the UV test pass.)
