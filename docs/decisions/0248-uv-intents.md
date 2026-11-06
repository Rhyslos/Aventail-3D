# 0248. UV intents

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 22. A piece is straightened, put on a trim row or given a texel density, then the model changes (an inset, an extrude, a re-unwrap). Mending (0104) lays the broken UVs out again, but as a plain layout: the strip is no longer straight, the trim piece slides off its row, the density drifts. The piece forgets what it was told.

## Decision

- **An intent is three polygon layers** (`mesh::uv_intents`: `uv_intent`, `uv_intent_a`, `uv_intent_b`), saved as every layer is: **Strip**, **Density** (texture widths per unit), **TrimRow** (bottom and top), **Upright** (the world's up runs up the texture), or none. A piece's intent is its first polygon's (by id) that has one.
- **Set by an edit of its own**, `Operation::SetUvIntent`, recorded in the same undo step as the tool that implies it: **Straighten** (Strip), **Set** and **Match texel density** (Density), **Put on trim row** (TrimRow). Recording it beside rather than inside those edits keeps every existing project's edits making exactly what they made before. **File format 92** stores it (`v92::IntentEditV92`; an unknown kind or values out of range are refused).
- **Made so again** (`uv::intent::reapply`) after a modelling edit that mends (with mending on) and after Unwrap, on each piece holding a polygon the edit changed (in shape or UVs), not locked: a strip is straightened, a density set, a trim piece scaled evenly to fill its row's height (its left side kept, so it doesn't jump back to the row's start), an upright piece turned about its middle.
- **UV → Pieces → Intents**: **Keep this way up** (turns the pieces now and keeps them so) and **Clear intents**. No keys.
- **The UV Editor says what a piece keeps** in the note bottom left when the mouse is over one of its polygons (Polygons and UV modes), unless the Stretch view's note is showing.
- **Mirrors** (the plan's "a mirror") aren't an intent here: mirrored pieces already stay stacked through Pack's Stack matching pieces, and keeping a mirror after an edit needs the mirrored partner found again, which can follow.

## Consequences

- Moving a piece by hand never fights its intent: intents are re-applied only after modelling edits and Unwrap, not after UV moves.
- Keep this way up turns the pieces as it is recorded, and again after later changes.
