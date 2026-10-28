# 0180. Painting in the UV Editor

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's third milestone ([0177](0177-texture-painter-plan.md)): the same brush in the UV Editor, so the texture can be painted flat (straight lines across a piece, painting a trim, reaching a spot the 3D view hides), live in both views. It decides where a flat stroke may paint, how big the brush is there, and how a flat stroke is kept.

## Options

1. **Paint anywhere on the texture square**, as a 2D paint program does. Paint landing between pieces is wasted and bleeds into neighbours when the texture is shrunk.
2. **Paint only inside the UV pieces of the material being painted** (chosen), plus the same thin rim outside their edges as on the model, so pieces' edges are covered and the empty space stays empty.

## Decision

- In **Paint mode** the UV Editor takes the brush in place of selecting: the same controls as the 3D view (left drag paints, 1 and 2, `[` `]`, Ctrl + right drag size and hardness, Ctrl + left drag flow), the same circle under the mouse. Navigation keeps its buttons. **Measure** still wins the left button while it's on.
- **Where it paints:** the material of the piece under the mouse when the stroke begins (as on the model); every shown object's pieces using that material are painted, as they share one texture set. Only texels inside those pieces (and within 0.75 of a texel outside their edges) are painted (`paint::flat::paint_flat_dab`). Nothing is hidden in the flat view, so **Paint through** doesn't apply.
- **Size:** measured on screen, the brush is its size in screen points at the editor's zoom. Measured on the surface, its centimetres are turned into the texture's size by the density of the piece where the stroke began, so it matches the brush on the model there.
- **Flow, hardness, spacing and the eraser** work as on the model: within a stroke each texel keeps its largest coverage.
- **Undo and live view:** the same as on the model: the layer goes to the document every frame under the stroke's merge key, so both views show the paint as it goes on, and the stroke is one step.
- **Kept strokes:** a flat dab is kept as a place on the texture (U, V, 0) with no way towards a camera (zero) and its radius in texture widths (`DabRecord::flat`). The file format doesn't change (65).
- The brush's capture (`viewport::BrushCapture`) is shared by both views: its events carry the view's own kind of place.

## Consequences

- Strokes can run across the 3D view and the UV Editor in turn, onto the same layer.
- Later milestones that paint strokes again (other texture sizes, after modelling) handle flat dabs by place on the texture.
