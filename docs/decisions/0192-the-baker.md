# 0192. The baker

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's fourteenth milestone ([0177](0177-texture-painter-plan.md)): the maps a hand painter leans on (where the light can't reach, where edges stick out), baked from the shape itself, as the starting point for guides (milestone 15) and smart masks (milestone 17). Phase 8's baker, for baking a high-detail shape onto a low one, will grow from the same crate later.

## Decision

- **A new core crate, `bake`** (layer 1: `mesh`, `paint`, `glam`; no GPU). Six maps, each the texture set's size, for every texel the shape's polygons cover:
  - **Position:** where on the shape, across its box (X red, Y green, Z blue; Z is up).
  - **Normals:** which way the surface faces, in the shape's own space.
  - **Occlusion** (ambient occlusion): rays spread over the half of the sky above each texel; the share that meet nothing within the reach (a fifth of the shape's size as standard). White open, dark in corners.
  - **Curvature:** from the shape's folded edges (polygons meeting at more than about 9°). Light along outer edges, dark along inner ones, fading over 1.5% of the shape's size; flat surfaces mid-grey. The shape's own polygons can't show finer curvature; Phase 8's high-to-low baker will.
  - **ID:** a flat, clearly different colour per material slot.
  - **Thickness:** rays sent in through the surface, and how far they go before leaving it. Light where thin, dark where thick.
- **Rays** use a bounding volume hierarchy over the triangles, and texels are shared over the processor's cores.
- **In the Paint layers panel**, a **Bakes** section:
  - **Bake maps** bakes all six for the shown material, on another thread, so the window keeps working.
  - **Quality** sets the rays a texel for occlusion and thickness: Draft 8, Good 32 (the standard), Best 128.
  - The maps are shown small, with what each shows on hover.
  - The section says when they are **out of date**: the shape's polygons moved on the texture, or the texture size changed since.
- **Baked from the shapes shown** using the material (their drawn shape, modifiers included), each object from its own polygons: one object's polygons don't shade another's.
- **Not saved and not undone:** the maps are worked out from the shape, so they are kept while the app is open and baked again when needed. Saving them can come with the features that use them, if baking again proves too slow.

## Consequences

- Guides and smart masks have their maps to read.
- Baking a big texture with Best quality takes a while (millions of rays). The window stays usable meanwhile, and Draft is quick for a first look.
