# 0098. Seams: marking, clearing, from hard edges, suggested and selected

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0095](0095-uvs-before-advanced-modelling.md) (the UV phase plan; seams from menus with live unwrapping, the project lead's choice), [0097](0097-editing-uvs-by-hand.md) (the UV ring), [0053](0053-hard-and-soft-edges.md) (hard edges), [0064](0064-automatic-hard-edges.md) (hard edges by angle)

## Problem

A seam says where the surface is cut open to lie flat. Every UV workflow needs seams marked by hand, seams from hard edges (a normal-map bake needs a UV split wherever the shading breaks), and, to start quickly, seams suggested for the whole shape, in places a viewer won't look. The project lead chose menus rather than new keys or mouse actions for seams (0095), with the UVs following the seams live once unwrapping exists.

## Options

- **How seams are marked:**
  - **Mark seams and Clear seams on the selection, from the UV ring (E) and the UV menu** (chosen). In the Edges mode, the selected edges; in the Points mode, edges between selected points; in the Polygons and UV modes, Mark seams marks the border of the selected polygons (select a region, cut it out) and Clear seams clears every edge of them;
  - a seam-painting tool with its own drag: a new mouse action nobody asked for. **Painting** is the Paint select brush on edges, then Mark seams.
- **What a seam changes now:** **only the mark** (chosen). The UVs follow when the shape is unwrapped, which is the next milestone and will then happen live as seams change (0095). Marking a seam on an open edge does nothing: it is cut already.
- **Seams from hard edges:** the edges shading hard **as shown** (by angle while Automatic hard edges is on, otherwise those marked), on the selected objects (chosen).
- **Suggested seams** (`uv::seams::suggest`), replacing the selected objects' seams in one step:
  1. cut along the shown hard edges and corners sharper than 66° (a piece bent further round one edge stretches badly);
  2. make every piece left a **disk** (one border, no handles: what lies flat in one go), with as little cutting as possible: a piece with no border is opened along the least seen path from its lowest point to its highest; then a spanning tree of its polygons keeps the most visible edges joined, the rest are cut, and cuts that lead nowhere are joined again (the classic tree–cotree cut, trimmed);
  3. "seen" scores an edge higher facing up or to the front, across flat stretches and on outward corners, and lower facing down or to the back (+Y, away from the Front view), in concave corners and on sharp ones.

  Other options: Blender's Smart UV Project (many small charts by angle, which hard-surface work then stitches back) or xatlas's charting (a large C++ library). The chosen way gives few pieces, keeps hard-edge splits, and is small, tested Rust.
- **Seen in the UV Editor:** seams are drawn **orange in the editor too** (chosen), matching the viewport, so a seam marked but not yet unwrapped is visible where you work.
- **Select seams** in the Select and UV menus, as Select hard edges.

## Decision

1. `mesh::uvs::set_seams` (adds the seam layer when needed; open and missing edges are skipped and counted).
2. `scene::Operation::SetSeams { edges, seam, anchors }`, mirrored with symmetry like hard edges; described "Mark as seams N edges" / "Clear the seams of N edges".
3. **File format version 29** (`project/v29.rs`): `OperationV29::SetSeams` and `MirroredSetSeams`; older versions refuse them. Sample in `tests/data/sample-v29.avtl`.
4. The `uv` crate's first module, `seams`: `pieces`, `is_disk` (from the Euler characteristic of the cut-open piece), `suggest` and `CORNER_ANGLE`. `uv` now depends on `mesh` (and `glam`); `ui` on `uv`.
5. UI: `workbench::seams` (`run_seams`, `run_seams_from_hard_edges`, `run_suggest_seams`, `run_select_seams`); `Selector::select_seams`; the UV menu built from the command list (seams, turns and flips, and the tools to come); `theme::UV_SEAM` in the editor.

## Consequences

- Seams can be placed by hand, from hard edges or suggested, undone in one step, saved, and they hold when earlier edits change.
- The next milestone (live unwrap) lays each piece flat along these seams and re-unwraps as they change.
- Suggestions use the object's own up and front: an object turned on its side gets its seams where its own bottom and back are.
