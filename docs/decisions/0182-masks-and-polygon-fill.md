# 0182. Masks and the polygon fill tool

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's fifth milestone ([0177](0177-texture-painter-plan.md)): masks, to show a layer only in places, and the polygon fill tool (key 4), to fill whole polygons, UV pieces or materials at once, as in Substance Painter. It decides what a mask holds, how it is painted, and what the fill tool fills and keeps.

## Options

1. **A mask as coverage only** (painting adds, erasing takes away). Simple, but unlike Substance Painter, where a mask is painted in black and white.
2. **A mask painted in grey over a white or black base** (chosen): the mask is an image like a paint layer's, laid over a base of white (show all) or black (hide all); how much the layer shows is the lightness there. The brush, eraser and fill paint it unchanged: painting white shows, black hides, erasing goes back to the base.

## Decision

- **Masks** (`paint::Mask`): any layer (paint, fill or group) can have one. **Add white mask** shows all of the layer to start with, **Add black mask** none; **Invert mask** swaps what shows and hides; **Remove mask** takes it away. How much shows is the lightness of what is painted on the mask, mixed over its base by the paint's coverage (`paint::set::amount`). The composite multiplies the layer's coverage by it, before clipping and blending, so layers clipped to a masked layer follow the mask.
- **Painting a mask:** the mask square at the end of a layer's row in the Paint layers panel chooses the mask; the brush and the polygon fill then paint on it (with the brush colour's lightness), until the layer's name is clicked. A mask keeps its own strokes.
- **The polygon fill tool** (key 4, Substance Painter's): a click fills, fully and with the brush's colour (or taking paint away with **Erase** ticked), the **polygon** under the mouse, its whole **UV piece**, or every polygon of its **material** (chosen in Attributes); a drag fills each new one it passes, all in one undo step. It works on the model and in the UV Editor, on the chosen layer or mask. Pieces are triangles joined by edges whose corners match in place and UV.
- **Kept fills:** each fill is kept with its stroke as the clicked polygon's id and what it filled (`FillRecord`), so later milestones can fill again after modelling.
- **Saving:** project format 67 adds masks (tiles, base, inverted, strokes) and each stroke's fills; a version 66 file opens without masks.
- Attributes shows the tool in use: **Polygon fill** with Fill (Polygon, UV piece, Material), Erase and Colour. Keys: 1 brush, 2 eraser, 4 polygon fill (3, 5 and 6 still say they come later).

## Consequences

- A fill layer with a black mask and polygon fills is the quick way to colour parts of a model, as in Substance Painter.
- Smart masks (milestone 17) will generate mask contents from bakes; they use the same `Mask`.
