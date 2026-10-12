# 0034. UVs as one phase after modelling, aiming to be the best UV tools

- **Status:** Accepted
- **Date:** 2026-10-12

## Problem

The project lead wants the app to have the best UV tools available. Until now the plan had UVs as Phase 6, between export and the workflow tools (edit layers, modifiers), with a modest list: seams, unwrap, packing, texel density.

Two questions:
- what would make the UV tools stand out;
- where in the plan they belong.

Unwrapping itself is a mature field, with RizomUV as the specialists' choice. What still hurts in every program is the work *around* unwrapping:
- UVs break when the model changes, so people leave them to the end;
- seam placement is guesswork;
- texture size, padding and texel density are only checked after baking;
- the flat UV view and the model are separate worlds.

## Options

1. **Keep UVs as Phase 6 with the basic list**, and add the bigger ideas later. The basics arrive sooner, but the important idea (UVs that survive modelling) needs edit layers, which come after Phase 6.
2. **One large UV phase after the modelling phases**, with the basics and the standout ideas together.

## Decision

Option 2, as the project lead asked: model first, then UVs. The project lead practises modelling in the meantime and has the UV tools by the time they're good at it.

New phase order:
1. Phase 4: core modelling tools.
2. Phase 5: export.
3. Phase 6: workflow tools, edit layers and the modifiers palette (was 7).
4. Phase 7: advanced hard-surface (was 8).
5. Phase 8: UVs (was 6).
6. Phases 9 and 10 (scripting, rigging) are unchanged.

The UV phase (DESIGN.md §7) holds the professional-standard basics plus seven standout features:
1. UVs that survive modelling;
2. seams suggested for hard surfaces, in hidden places;
3. bake-aware texture settings (size, texel density, padding in real pixels, warnings about mipmaps);
4. UV editing directly on the model;
5. problems explained, with a fix;
6. straight by default;
7. packing that rivals the specialists, including stacking identical parts.

Bake readiness for UVs and trim sheets join them.

## Consequences

- Export (Phase 5) can no longer be checked with a full bake, which needs UVs. Its gate becomes "the prop looks right in Godot and Substance Painter (shape, hard edges, scale)". The bake round trip moves to the UV phase's gate.
- Until Phase 8, exported models have no useful UVs. Primitives' simple built-in UVs can be exported where they exist, but they aren't a goal.
- UVs that survive modelling build on edit layers (Phase 6) and on the anchors of [0032](0032-edits-find-their-points-again.md). Putting UVs after both makes that possible.
- Earlier decisions mention the old phase numbers (Edit Layers as Phase 7, UVs as Phase 6). They are records of their time and stay as written. DESIGN.md and the README tracker carry the current numbers.
- Matching the best packers means writing our own packing; xatlas (MIT) is the starting point.
