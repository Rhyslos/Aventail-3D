# 0118. Select connected (Alt + A) and the outside-the-square warning

- **Status:** Accepted
- **Date:** 2026-10-18
- **Follows:** [0029](0029-selection.md) (selection), [0115](0115-seeing-and-checking.md) (seeing and checking)

## Problem

Two requests from the project lead while testing:

1. A way to select every polygon of one connected part of a shape (in the UV mode a click takes one piece, which stops at seams). He asked for a key and chose **Alt + A**, checking it was free.
2. The UV Editor warned about overlapping and mirrored pieces, but not about pieces sticking out of the texture square.

## Options

- **Select connected:**
  1. **Everything joined to the selection through edges and polygons, in the mode's kind (chosen).** Seams and UV splits don't part anything: it's the shape that counts. The same in the Points, Edges, Polygons and UV modes.
  2. Only the UV piece (what a click in the UV mode does already).
- **Where:** **Alt + A** (the project lead's choice; Ctrl + L, as Blender has it, and Ctrl + Shift + A were offered), **Select → Select connected**, and **UV → Pieces → Select connected**, as he asked for it in the UV menu.
- **Outside the square:** **a piece any of whose UVs lies past 0 or 1 (with the bake check's slack) is tinted amber and counted top right (chosen).** Trim pieces are left out: they run past the sides on purpose. The bake check already lists them for a fix.

## Decision

1. `mesh::selection::connected(mesh, topology, selection, kind)`: a flood through points' neighbours from the selection's points.
2. `ui`: `selecting::CONNECTED_SHORTCUT` (Alt + A), `Selector::select_connected` (one undo step, "Select connected"; says what to do with nothing selected), `SelectRequest::Connected`, the Select menu's and UV → Pieces' entries, and the keyboard map.
3. `uv::warnings::Warnings::outside` and `outside_pieces` (`OUTSIDE_SLACK`); `ui::uv_editor::checking::WarningCounts`, `theme::UV_OUTSIDE`.

## Consequences

- Alt + A is taken. On macOS, Option + A is used the same way as the app's other Alt keys.
- A piece both outside and overlapping is tinted red (overlaps first), and counted in both.
