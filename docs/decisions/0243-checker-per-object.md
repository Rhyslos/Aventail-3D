# 0243. The numbered checker in each object's own colours

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 16. With the numbered checker on, every object shows the same blue-to-green cells, so where two objects meet it's hard to see whose UVs are whose, or that one has a different texel density.

## Decision

- **A hue turn per draw**, not an image per object (the plan's texture per draw item or texture array): `render::SurfaceDraw::NumberedChecker { hue }` passes how far round the colour wheel to turn the one checker image (0 to 1), and the shader turns the sampled colour about the grey axis (`hue_turned`, Rodrigues' rotation), so brightness and the labels stay. No more GPU memory, and the image cache is unchanged.
- **Each object's hue** is `ui::checker_hue(id)`: its id times the golden ratio, the fraction part, so neighbouring objects get far-apart colours and each keeps its own. Choosing a hue per object in the Attributes panel can follow if wanted.
- **Display → Numbered checker per object** (`ViewOptions::checker_per_object`, off as standard, remembered), offered while the numbered checker is shown. No keys.
- The UV Editor keeps one image (it shows the selected objects' UVs over one square).

## Consequences

- An object whose hue happens to be near 0 looks like the plain checker; only the first id is exactly 0.
