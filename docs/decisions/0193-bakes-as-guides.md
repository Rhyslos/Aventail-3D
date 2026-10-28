# 0193. Bakes as guides

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's fifteenth milestone ([0177](0177-texture-painter-plan.md)): the baked maps ([0192](0192-the-baker.md)) should help while painting. A painter wants to see where shade and wear go while laying in colour, and to start from them: a layer of shade, or a layer of dirt kept to the corners.

## Decision

- **A map clicked in Bakes shows on the model as a guide**, over the paint, in the 3D view and the UV Editor. Click it again (or another map) to stop or change.
  - It is mixed the way the map is read: occlusion multiplies (darkens corners), curvature overlays (lightens outer edges, darkens inner ones, leaves mid-grey alone), and the others simply cover.
  - **Strength** sets how strongly it shows (60% to start).
  - Where nothing is painted yet, it shows over the material's own colour.
- **The guide is only shown, never painted**: painting, saving and export see the paint alone. The views mix it over each tile of the painted texture, and only the tiles that change are mixed again as you paint.
- **Add as a layer** puts the guide's map in a new paint layer above the chosen one, mixed the same way (occlusion as Multiply, curvature as Overlay). It is then ordinary paint: paint over it, mask it, or lower its opacity.
- **Use as the layer's mask** makes the map the chosen layer's mask: the layer shows where the map is light. That puts dirt in corners (occlusion, inverted) or wear on outer edges (curvature). Smart masks with settings come in milestone 17.
- Both are one undo step each, and need the maps baked at the texture's size: after a size change, bake again.
- **Fix:** the maps were said to be out of date as soon as a texture set was made after baking. The layout is now worked out from the shapes for the check, once a drawing.

## Consequences

- Shade and wear can be painted by eye against the shape's own maps, and blocked in with a click.
- A guide over a big texture is mixed again in full when it is turned on or its strength changes, which can take a moment at 4K.
