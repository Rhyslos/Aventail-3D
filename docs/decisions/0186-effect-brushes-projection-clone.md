# 0186. Effect brushes, projection and clone

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's eighth milestone ([0177](0177-texture-painter-plan.md)), second part, after the brush bar and tips ([0185](0185-brush-bar-and-tips.md)): the brushes that change what is already painted (smudge, blend, blur, hue shift), projection (Substance Painter's 3) and clone (6).

## Decision

- **New brush kinds** (`paint::BrushKind`): **Smudge**, **Blend**, **Blur**, **Hue shift**, **Projection** and **Clone**, besides Paint and Erase. They paint with the same dabs, tips, pressure and smoothing; within a stroke each texel still takes its largest coverage, and the flow is how strong the effect is.
  - **Blur** moves each texel toward the average around it (4 texels), read from the layer as it was when the stroke began, so a stroke is one even pass; stroke again for more.
  - **Blend** does the same over a wider reach (9 texels): colours under the brush run together.
  - **Smudge** carries a colour picked up where the stroke starts and lays it along the stroke, picking up 30% of what's under each dab as it goes: paint is dragged.
  - **Hue shift** turns the hue of what's under it by a set amount (−180° to 180°, Attributes), keeping saturation, lightness and coverage.
  - Mixing is done premultiplied, so empty texels don't bring their colour in.
- **Projection (3):** a picture chosen in Attributes (**Picture…**, any image the app reads) is laid over the views, at a scale and offset set in Attributes (and how strongly it shows); painting puts its colours on the surface where they lie on screen. In the UV Editor the picture is stretched over the texture square. Moving and scaling the picture by dragging (Substance Painter's S + drag) is left for later: S is the scale tool's key in the app.
- **Clone (6):** **V + click** (Substance Painter's) sets where to copy from; the first stroke after fixes the offset on the texture, kept for later strokes (aligned), until V + click sets another. Clone copies texels at that offset in texture space, the same on the model and in the UV Editor. In Paint mode V does nothing else (it isn't Bevel there).
- **The brush bar** has the six as built-in brushes, with an **Effects** filter; kept brushes keep their kind.
- **Saving:** strokes of these kinds are saved (project format 71, kinds 2 to 7).

## Consequences

- Hand-painted textures can be softened, mixed, tinted and patched in the app.
- Clone works in texture space: copying across UV seams copies what lies next to the source on the texture.
