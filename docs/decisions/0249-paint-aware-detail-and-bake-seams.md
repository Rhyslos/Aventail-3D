# 0249. Paint-aware detail and the bake seam preview

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 24. Wear, chips and edge highlights are painted on a model's sharp outer edges and corners; pieces made mostly of those edges deserve more texture than flat panels, but packing gives every piece the same density. And a normal-map bake shows where seams and shading disagree (a thin line along a seam on a smooth edge, smeared shading across a hard edge with no seam), but only after baking.

## Decision

- **Pack → More texture where wear is painted** (off as standard, remembered): each piece's *wear* (`uv::density::wear`: the length of its outward edges sharper than 30°, inside it or on its border, over the square root of its area, so pieces of any size compare) against the average of the pieces packed: `sqrt((1 + wear) / (1 + average))`, from three quarters to one and a half times as much texture across, on top of its own texture scale and the player view (0247).
- **Display → Bake seams** (`ViewOptions::show_bake_seams`, off, remembered), on the selected objects in every mode: **red** on seams along smoothly shaded edges (the bake's tangents part there and a line shows), **yellow** on hard edges with no seam (the bake smears the shading across). Where the UVs part is the marked seams, or the UV splits on a shape without marks (`uv::unwrap::cuts_of`); the shading is the object's smoothing. Drawn as overlay lines (`OverlayStyle::BakeSeam`, `BakeUnsplit`), not as a surface as the plan said: lines on the model show the very edges to fix, and the surface stays free for the checker or the painting.
- No keys.

## Consequences

- The wear weight looks at the shape, not at the painting: a flat panel you paint heavily gets no more texture. A per-piece texture scale (0240) covers that.
- The bake seam preview is about tangent-space normal maps; occlusion and curvature bakes show no such lines.
