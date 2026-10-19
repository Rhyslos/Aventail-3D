# 0115. Seeing and checking in the UV Editor

- **Status:** Accepted
- **Date:** 2026-10-17
- **Follows:** [0105](0105-bake-readiness-for-uvs.md) (the UV bake check), [0108](0108-uv-powerhouse-and-baking.md) (the Phase 8 plan), [0114](0114-layout-snapshots.md) (layout snapshots)

## Problem

Phase 8's milestone 15, the second of the six the project lead asked for in least-work order:

- always-on warnings for overlapping and mirrored pieces in the UV Editor;
- stretch, pixel size and texel density shown on each piece;
- measuring in texture pixels and real centimetres.

The controls were already chosen: warnings always on, numbers on pieces as a View switch, measuring from the menu and then a drag.

## Options

- **Warnings:**
  1. **Tint the polygons (red for overlapping, purple for mirrored) and count the pieces top right (chosen).** Seen at a glance, even on small pieces, without hiding the outlines.
  2. Outlines only. Easy to miss among the seams' orange.
  3. Run the full bake check on every change. Too slow while dragging: it draws pieces into a 1024-cell grid.
- **What overlaps:** the bake check's own test (`uv::check::check_sheet`) on a 256-cell grid, quick enough to run on every change, per texture as packing groups pieces (by material and object). Stacked identical pieces and trim pieces overlap on purpose and aren't warned about. Polygons folded over inside their own piece count as overlapping (red), since they share pixels with it too.
- **What is mirrored:** a piece whose flipped polygons have more area than its unflipped ones (`uv::warnings::mirrored_pieces`).
- **Numbers on pieces:** each piece's box in texture pixels, its texel density in px/m, and its worst polygon's stretch when over 1.05×; on pieces at least 70 points wide on screen, so they never pile up.
- **Measuring on the surface:**
  1. **Walk the line in 256 steps, find the triangle under each step, and add up the world distance between steps on the same piece (chosen).** Gaps and jumps across a seam count nothing, which is what "on the surface" means.
  2. Map only the two ends. Wrong for any line crossing a curved piece.
- **Stopping measuring:** Esc (as for the slit's line and later brushes), or choosing Measure again.

## Decision

1. `uv::warnings`: `Warnings { overlapping, mirrored, overlapping_pieces, mirrored_pieces }`, `warnings(meshes, group, rows)` and `mirrored_pieces`; `WARNING_CELLS` 256.
2. `uv::measure`: `Surface::new(mesh, to_world)` (triangles in the texture and in the world, in a 64 × 64 grid), `Surface::at` and `surface_length`; `MEASURE_STEPS` 256.
3. `ui::uv_editor::checking`: the warning cache (worked out again when the document's revision, what is shown or the texture grouping changes), the tints (`theme::UV_OVERLAP`, `UV_MIRRORED`) and summary (`UV_WARNING_TEXT`); numbers on pieces (`piece_numbers`, `numbers_text`); measuring (`use_measure`, `paint_measure`, `theme::UV_MEASURE`). The UV Editor's View menu gains **Numbers on pieces** and **Measure**; `UvContext::texture_groups` carries packing's grouping. While measuring, a left drag measures instead of moving or selecting; Esc stops it (in `Workbench::document_keys`).

## Consequences

- No new keys apart from Esc to stop measuring.
- Nothing is saved: the switches start off each time.
- On very heavy shapes (up to the editor's 300 000 polygons), the warnings add to the work done when the UVs change, as the stretch colours already do.
- Milestone 23 ("Why is this here?") can explain a warned piece when hovered.
