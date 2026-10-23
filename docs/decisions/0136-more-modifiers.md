# 0136. More modifiers for Phase 10

- **Status:** Accepted
- **Date:** 2026-10-24
- **Builds on:** [0125](0125-modelling-before-the-rest-of-the-uvs.md) (modelling before the rest of the UVs), [0135](0135-live-booleans.md) (live booleans)

## Problem

After booleans, the project lead felt Phase 10 had too few modifiers. It listed weighted normals, a better bevel, array and mirror, shell, sweep and lathe, smoothing, the game bevel and kit helpers.

## Options

The modifiers other 3D programs offer for game assets, offered in four groups (recreated in behaviour only, with our own names and icons):

- **Generators:** live bevel, scatter onto surface, wireframe, radial array.
- **Deformers:** bend, twist and taper; lattice cage; curve deform and shrinkwrap; displace and noise.
- **Game pipeline:** LOD generator, collision hull, normal transfer, triangulate preview.
- **Clean-up:** weld by distance, decimate, remesh.

## Decision

The project lead picked every one except normal transfer, and added **quadify** (triangles joined back into quads). Radial array joins the existing array milestone. The new milestones, in the order they're built:

1. Live bevel modifier (after the improved bevel tool).
2. Wireframe (after shell).
3. Bend, twist and taper.
4. Lattice cage.
5. Curve deform and shrinkwrap (after curves and splines).
6. Displace and noise.
7. Scatter onto surface.
8. Weld by distance, quadify, remesh and decimate (after the smoothing modifier).
9. LOD generator (uses decimate), collision hull, triangulate preview.
10. The game bevel and kit helpers stay last: the game bevel builds on the live bevel and weighted normals.

## Consequences

- Phase 10 goes from 9 to 23 milestones.
- Every one is non-destructive, on the modifiers palette, and baked by **Bake into shape**, like booleans.
- Each milestone still asks the project lead about its controls before it's built ([0028](0028-ask-about-controls.md)).
