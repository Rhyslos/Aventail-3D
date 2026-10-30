# 0194. Gradient fills

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's sixteenth milestone ([0177](0177-texture-painter-plan.md)), the first of the hand-painted kit. The quickest way to give a hand-painted shape form is a gradient: darker low, lighter high (light comes from above). Painting one by hand across many UV pieces is slow and never even.

## Decision

- **A gradient layer** is a paint layer that keeps its settings (`paint::Gradient`, `Layer::gradient`):
  - two colours, **From** (low end) and **To** (high end);
  - which way it runs: **left to right (X)**, **front to back (Y)** or **bottom to top (Z, the standard)**, along the object's own axes;
  - where it **starts** and **ends** along the object's box (0% to 100%). Before the start it is all From, after the end all To; start and end together make a hard step.
- **Its pixels are made on the shape** (`bake::gradient`): every texel the material's polygons cover gets the gradient's colour at its place along the object's own box. Each object sharing the material runs from its own low side to its own high side.
- **+ Gradient** in the Paint layers panel adds one above the chosen layer (dark blue-grey to warm light, bottom to top). Its settings sit with the other layer settings; changing them makes it again. The panel asks, the workbench makes it on the shapes, and the result joins the same undo step. Dragging a slider is one step.
- **Not painted into:** a stroke with a gradient layer chosen goes on a new paint layer above it, since the gradient would be made again from its settings. Mask it, clip to it, or change its blend mode and opacity like any layer.
- **It follows the model:** when its shapes or the texture size change, it is made again from its settings instead of moved or resized, so it stays exact.
- **Saving:** project format 72 saves the settings with the layer (its pixels are saved as for any paint layer, so the look survives even without them).

## Consequences

- A first pass of form is one click, and stays right through modelling.
- Gradients along any direction other than the object's axes, radial gradients and gradients in screen space are left for later.

## Amended 2026-10-30

+ Gradient now makes a **Multiply** layer from a cool dark (96, 92, 120) to white, so it shades what's under it; the old Normal navy-to-cream replaced the base colour (first painter review). `Gradient::default` is unchanged, so saved gradients look the same.
