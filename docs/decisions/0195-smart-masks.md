# 0195. Smart masks

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's seventeenth milestone ([0177](0177-texture-painter-plan.md)), part of the hand-painted kit. Wear on edges, dirt in creases, moss or dust on top and a tide line low down all follow the shape. Painting their masks by hand across many UV pieces is slow, and they go wrong when the model changes.

## Decision

- **A smart mask** is a layer mask that keeps settings (`paint::SmartMask`, `Mask::smart`) and is made from them on the shape (`bake::smart_mask`). It has one of four kinds (`paint::SmartKind`):
  - **Edges:** outer edges, where paint wears and chips (the shape's curvature, convex side).
  - **Cavities:** inner edges and creases, where dirt gathers (curvature, concave side).
  - **Facing up:** surfaces that face up, where dust, snow and moss settle.
  - **Height:** above a level on each object's box, for a tide line or a fade toward the top.
- **Its settings:** how much it takes in (**Width** for edges and cavities, **Reach** for facing up, **Level** for height), **Softness** of its border, **Noise** to break the border up, **Noise size**, and **New pattern** for another noise pattern with the same settings. Noise is worked out in 3D on the shape, so it has no seams across UV pieces.
- **Add smart mask** in the Paint layers panel gives the chosen layer one, picking out edges to start with. Its base is black, so it hides the layer wherever it isn't made (off the polygons). Like gradients ([0194](0194-gradient-fills.md)), the panel asks, the workbench makes it, and the result joins the same undo step; dragging a slider is one step.
- **Not painted into:** a stroke with the smart mask chosen paints the layer instead, since the mask would be made again from its settings. Invert it with Invert mask, or remove it.
- **It follows the model:** when its shapes or the texture size change, it is made again from its settings.
- **Saving:** project format 73 saves the settings with the mask.

Substance Painter calls these smart masks and generators; the kinds match the common ones there.

## Consequences

- Edge wear and grime are two clicks and a slider, and stay right through modelling.
- A smart mask can't be touched up by hand yet: put a second, painted layer above or below instead. Mixing a painted mask with a smart one is left for later.
- Edges and cavities need the shape's curvature, so very low-poly shapes with soft normals show softer bands.
