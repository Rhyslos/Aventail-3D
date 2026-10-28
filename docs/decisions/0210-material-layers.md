# 0210. Material layers

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's thirty-first milestone ([0177](0177-texture-painter-plan.md)): "tiling materials (metal, wood, stone, paint) made in the app, projected from three sides so they need no UVs." Painting a convincing wood or brushed metal by hand takes a long time; Substance Painter starts most realistic work from ready-made materials laid on with tri-planar projection.

## Options

- **Picture-based materials** (shipped texture images): look best, but need image assets with licences and grow the download; a rule forbids copying other programs' assets.
- **Patterns made in the app (chosen):** made from noise, so nothing is copied, any size, and every setting can change.

## Decision

- **A material layer** (`Layer::material`, `paint::MaterialFill`): a layer whose pixels are made on the shape from settings, like a gradient or light and shade, and not painted into.
  - **Kinds:** Brushed metal (streaks one way, a faint blotch), Wood (rings across the grain, wobbling, and fine grain along it), Stone (layered blotches with dark specks), Paint coat (a faint even texture).
  - **Settings:** Colour and Second colour, Scale (1 to 1000 cm per repeat), Sharpness (how the sides meet at corners), New pattern; with the realism channels ([0207](0207-realism-channels.md)), Roughness and Metal (the pattern varies the roughness a little around it).
- **Projected from three sides** (`bake::material`): each texel takes the pattern seen along X, Y and Z at its place on the shape, weighted by which way the surface faces (sharper weights cross over sooner). No particular UVs are needed, and UV seams don't show.
- **Its channels:** with the realism channels on, the layer's roughness and metal are made too, as painted parts; off, only the colour.
- **Remade** whenever its settings, the shape or the texture size change, in the same undo step, like the other made layers.
- **Paint layers panel → + Material** adds one (Brushed metal to start). Its mask, opacity, blend and clipping work as for any layer; dirt, wear and rust ([0209](0209-wear-dirt-and-rust-masks.md)) go on top.
- **Saving:** project format 80 stores the settings (`LayerV64::material`), checked on opening; the pixels are saved as paint, so an older app shows the layer as painted.
- **No keys.**

## Consequences

- A realistic prop can start from metal, wood or stone in a click, then get worn, dirty and hand-painted details.
- The patterns are simple: they read well at game distances, not as close-up photographs. Picture-based materials can come later as presets.
