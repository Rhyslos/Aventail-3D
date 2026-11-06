# 0252. The live bake preview and its problems, explained

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 27. A bake (0250, 0251) shows what it made only once the files are written and opened elsewhere; trying cage settings means baking again and again. And where it goes wrong (the cage missing the high model, reaching through to another part, detail smeared on a steep slant) has to be guessed from the result.

## Decision

- **Preview on the model** in the Bake window (off as standard), with a choice of what: any map, or **Problems**. It bakes the first object at most 256 pixels across, on another thread, and shows the result on the model and behind the UVs as the **Texture picture** surface (0244), so the preview reuses what is already drawn; switching the preview off (or closing the window) puts the surface back. It bakes again whenever the shapes (any edit, including cage painting), their placement or the settings change, stopping a bake that has become out of date. Not baked in tiles bit by bit as the plan said: at 256 pixels a preview takes well under a second.
- **Problems** (`bake::maps::problem_map`), with a high-detail object, per texel: **missed** (red: the ray from the cage met nothing), **probably another part** (purple: met within the last tenth of its reach, behind the part meant), **skewed** (yellow: the high surface faces more than about 70° away from the low one, so its detail smears); grey where all is well. The window lists each problem over half a percent with its share and what to do (raise or lower Cage, paint the cage out or in there, add an edge loop or a bevel, or make the edge hard and a seam), as the bake check explains and fixes (0077, 0105).
- No keys.

## Consequences

- The preview's small size hides problems smaller than a few of its pixels; the full bake's files are the final word.
- Problems need a high-detail object; baked from itself there is nothing to check, and the list says nothing.
