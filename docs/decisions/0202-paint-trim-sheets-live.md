# 0202. Paint trim sheets and kits live

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's twenty-third milestone ([0177](0177-texture-painter-plan.md)): "paint a trim sheet once and watch every kit piece that uses it update, with your trim sheet and modular kit tools". Trim strips are laid along their row and run past the texture square, since the trim repeats left to right ([0106](0106-trim-sheets.md)). Painting skipped every part of a triangle outside the square, so most of a trim strip couldn't be painted; and a broad stroke on a strip spilled into the rows above and below, ruining other trims.

## Decision

- **Painting follows the repeat.** A triangle whose UVs run past the square is painted (and filled, and made for gradients, smart masks, light and shade and bakes) in every square it reaches, moved back onto the texture (`paint::surface::tile_shifts`, at most 16 squares each way, so a broken UV can't run away). A stroke on any part of a strip lands on the texture where that part repeats from.
- **Strokes stay in their trim row.** When the project has trim rows, a stroke (or a polygon fill) keeps to the row it starts in: texels above and below it are left alone (`StrokeCanvas::keep_to_rows`). **Trim row: Stay in the row** in the brush's Attributes turns it off; it shows only while there are trim rows.
- **Every piece updates at once.** Kit pieces share a trim sheet by sharing a material, and a material has one texture set ([0178](0178-textures-on-materials.md)): painting any piece changes the set, so every piece using it updates in the viewport and the UV Editor while you paint. Nothing new is needed for that; the manual explains how to set it up.
- **No keys.**

## Consequences

- A trim sheet is painted where it's seen: on the model, across any of the strips that use it.
- Painting along a row in the UV Editor still stops at the square's edge (the UV Editor shows one square); paint following the model (moving paint when UVs change) still keeps to the square for strips past it.
