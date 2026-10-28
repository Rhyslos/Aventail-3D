# 0189. Paint follows the model

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's eleventh milestone ([0177](0177-texture-painter-plan.md)), the first of "paint that follows the model". Paint lies on the texture, and the texture is laid on the shape through its UVs. Any change that moves polygons on the texture used to leave the paint behind, in the wrong places: an extrude, a bevel, a loop cut, a UV move, a re-pack or a re-unwrap. In other painters the answer is "finish modelling before painting". This app owns the modelling and the UVs, so it can do better.

## Options

- **Replay the strokes** kept with each layer ([0179](0179-the-round-brush.md)) on the changed shape. That repaints sharply, but only what was painted with the brush (not imported pictures, fills or effects). The dabs are kept in space, so paint stays where it was in the world rather than on the polygons that moved.
- **Move the texels with their polygons** (chosen). After a change, each triangle whose UVs moved takes its texels from where the same place on the shape was before. This works for every layer and mask, whatever made the paint.

## Decision

- **What moves:** after each change, a texture set whose polygons lie differently on the texture has its paint moved:
  - A triangle of a polygon still there, cut into as many triangles as before, takes its texels from the same triangle of that polygon, point for point. Paint stays on the polygon however its UVs moved or it was reshaped.
  - Any other triangle (a new polygon, or one cut differently) takes them from the nearest point of the shape as it was. A new polygon picks up the paint next to it.
  - Texels of triangles whose UVs didn't change aren't touched, so paint doesn't soften edit after edit. Moved texels are read smoothly from the four around their old place (weighted by coverage, so edges don't darken).
  - Every paint layer and mask moves, groups included; fill layers have nothing to move.
- **When:** the workbench keeps, per texture set, the set and the shapes it was last seen with. A *new* change that leaves the set as it was, but its shapes laid out differently, moves the paint.
  - The moved set joins the change's own undo step (`Document::execute_with_last`), so one undo takes both back.
  - Undo and redo restore shape and paint together, and aren't new changes (`Document::changes_made`), so nothing moves twice.
  - During a drag (a mouse button held) it waits until the drag ends.
- **What counts as the layout:** each object's triangles using the set's material, their polygons and their UVs (`paint::follow::layout_key`). Moving points without moving UVs changes nothing: the paint is on the UVs.
- **Strokes:** a stroke's frames now merge into one change of the texture set per undo step (the set before and after), so the history no longer keeps every frame's tiles. The strokes kept with layers stay as painted; resizing the texture (milestone 13) decides how to use them on a changed shape.
- **No file format change:** what was last seen is worked out again when a project opens.

## Consequences

- Model, unwrap and pack in any order around painting. Re-packing and re-unwrapping keep the paint on its polygons (milestone 12 checks them in detail).
- Paint that moves is read from its old place, so it can soften a little where polygons are stretched or shrunk. Paint that doesn't move stays exact.
- A modifier that rebuilds polygons (subdivision, say) makes new polygons. Switching it on or off moves the paint by nearest point each time, which can soften it.
- A new polygon far from any painted one picks up whatever paint is nearest, which may need painting over.
