# 0108. UV powerhouse and baking, before advanced modelling

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0095](0095-uvs-before-advanced-modelling.md) (UVs before advanced hard-surface), [0034](0034-uv-phase-after-modelling.md) (the UV phase), [0077](0077-bake-check.md) and [0105](0105-bake-readiness-for-uvs.md) (the bake checks)

## Problem

With the UV phase built, the project lead asked for more:

- everything the other 3D programs do well for unwrapping;
- new baking features;
- something for UVs that no other app has.

He chose:

- all of the first two;
- all four new ideas offered;
- baking inside the app rather than leaving it to Substance Painter;
- all of it before advanced modelling.

## Options

- **When:** **a new Phase 8 before advanced hard-surface** (chosen by the project lead; it keeps his momentum on UVs). The alternatives were after Phase 9, or the small tools now and the rest later.
- **Baking:** **in the app** (chosen by the project lead): normal, ambient occlusion, curvature, thickness, colour ID, position and height maps. The rest of the bake moves here too: high-to-low with a cage, live preview and explained problems. Substance Painter becomes optional for bakes. The alternative was keeping Substance Painter as the baker with only readiness tools; it would leave the bake check unable to show real bake problems.
- **The new ideas** (all chosen):
  - player-view UVs;
  - UV intents;
  - "why is this here?" for every automatic choice;
  - paint-aware detail with a seam preview before baking.

## Decision

Phase 8 is "UV powerhouse and baking", with 19 milestones listed in DESIGN.md and the README. The order is:

1. the familiar UV tools;
2. the new ideas, which build on them (pins, projections, packing controls);
3. the baker, which builds on both (live preview needs fast unwrapping; the seam preview, player views and paint-aware detail feed it).

Advanced hard-surface becomes Phase 9, scripting Phase 10, rigging Phase 11. Baking moves out of the texturing plan into this phase. Texturing still follows Phase 9 and starts from the maps baked here.

Each milestone asks the project lead about its controls before it adds keys or mouse actions (decision 0028). Each copies behaviour only, never names or assets, from the programs it learns from.

## Consequences

- Advanced hard-surface waits one more phase. Its tools will carry UVs and work with UV intents from the start.
- The baker is the largest new piece of the app (ray casting against high-poly meshes). It gets its own crate, kept free of the GPU so it can run in tests and on another thread.
