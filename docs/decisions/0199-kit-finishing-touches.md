# 0199. Hand-painted kit: finishing touches

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

Checking milestones 16 to 20 against the texture painter plan the project lead reviewed ([0177](0177-texture-painter-plan.md)) showed three promises not yet kept:

- Gradient fills "by height, by a direction you drag, per UV island or per part". Gradients ran only across each whole object ([0194](0194-gradient-fills.md)).
- The light-and-shadow helper: "shadows shift cooler and light warmer by themselves, with a slider for how much". The colours could be chosen, but there was no slider for how much ([0196](0196-light-and-shade.md)).
- Smart masks: "cavities from AO". Cavities come from the shape's curvature instead ([0195](0195-smart-masks.md)).

## Decision

- **Gradients run across each object, each part or each UV piece** (`paint::GradientSpan`, the **Across** setting). A part is a piece of the object not joined to the rest (triangles sharing corners); a UV piece is triangles sharing UVs. Each runs from its own low side to its own high side, along the chosen axis. A crate made of planks as one object gets a gradient on every plank.
- **Light and shade's Warm and cool** slider (`Lighting::warmth`, 0% to 100%, standard 100%): at 0% the light and shade are greys of their own lightness; at 100% the colours as chosen.
- **Saving:** project format 75 saves both. Older files read as each object and fully warm and cool, as they were drawn.
- **Cavities stay from curvature,** not occlusion (AO): curvature is quick enough to make again while a slider is dragged, and on hand-painted models it picks out the same creases. Occlusion needs many rays per texel, seconds on a big texture. A cavities kind from the baked occlusion map can come with the realism masks (milestone 30).
- **"A direction you drag"** for gradients is left for later: it needs a new mouse action in the viewport, and the project lead asks to be consulted on those ([0028](0028-ask-about-controls.md)). The axis choice covers most uses.

## Consequences

- The kit now does what the plan describes, apart from the dragged gradient direction.
- A gradient across parts or UV pieces costs a little more to make (grouping the triangles), still well under a frame's work on ordinary models.
