# 0209. Wear, dirt and rust masks

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's thirtieth milestone ([0177](0177-texture-painter-plan.md)): "smart masks for realism, built on the bakes." Realistic materials get their story from wear, dirt and rust: worn edges showing bare metal, grime low down and in creases, rust creeping from edges and running down. The smart masks so far ([0195](0195-smart-masks.md)) pick out plain edges, cavities, upward faces and heights.

## Decision

- **Three more smart mask kinds** (`SmartKind::Wear`, `Dirt`, `Rust`), made on the shape like the others, from its curvature and place, with noise:
  - **Wear:** the outer edge band cut by fine noise into chips and scratches, with a few flecks past it. *Width* sets the band.
  - **Dirt:** creases, and the lower part of each object fading upward (most on its sides). *Reach* sets how far up.
  - **Rust:** blotches spreading from edges and creases, and streaks running down the sides (noise stretched upright). *Spread* sets how far.
  - Each starts broken up more than the plain kinds (more noise, smaller), and takes its seed.
- **Ready-made layers:** with the realism channels on ([0207](0207-realism-channels.md)), the Paint layers toolbar adds **+ Dirt**, **+ Wear** and **+ Rust**, each one undo step:
  - Dirt: a dark brown fill, 75%, roughness 95%;
  - Wear ("Worn metal"): bare steel, roughness 35%, metal 100%;
  - Rust: orange-brown, 90%, roughness 90%.
- **Built on the shape, not on baked occlusion.** Occlusion needs rays for every texel, too slow to remake on every change of the shape; curvature and place give the same corners and creases at once. (Baked occlusion can still be a mask: Bakes → Use as the layer's mask.)
- **Saving:** project format 79 allows smart mask kinds 4 to 6; an older app refuses them.
- **No keys.**

## Consequences

- A realistic prop gets worn, dirty or rusty in a click or two, and the effect follows the shape as it changes.
- The masks stay settings, so the same look applies to any model (and goes into presets).
