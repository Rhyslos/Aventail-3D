# 0095. UVs before advanced hard-surface, and how the UV phase works

- **Status:** Accepted
- **Date:** 2026-10-16
- **Amends:** [0034](0034-uv-phase-after-modelling.md) (the UV phase after all the modelling phases)
- **Follows:** [0043](0043-attribute-layers-and-the-builder.md) (attribute layers and the builder), [0091](0091-edit-layers.md) and [0092](0092-the-edit-list.md) (edit layers and the edit list), [0090](0090-stable-element-names.md) (stable names), [0028](0028-ask-about-controls.md) (asking about controls)

## Problem

With Phase 6 done, the project lead asked to build the UV phase next and advanced hard-surface after it, unless the UV phase needs something from it. The UV phase also adds a second working area (the flat UV Editor), so its controls had to be chosen, and the project lead set a rule for every area to come.

## Options

- **Order:**
  - **UVs next (Phase 7), advanced hard-surface after (Phase 8)** (chosen, the project lead's request). The UV phase needs nothing from advanced hard-surface: what "UVs that survive modelling" waited for (edit layers, the edit list) is built, stable names let seams keep their edges, and the builder already carries UVs (corner values) and seams (edge values) through every tool. The other way round helps: the game bevel needs seams, and booleans, generators and the rest can carry UVs from the start;
  - keep the order, which leaves the export pipeline without real UVs longer.
- **Where shapes get their first UVs:**
  - **every primitive comes unwrapped** (chosen): a cube's sides side by side, a cylinder's side as a strip with its ends, a cone as a fan, a sphere like a map, cut along seams, pieces at their real relative sizes. Textures and exports work at once, and the UV tools have something to work on;
  - no UVs until unwrapped by hand, as before, which leaves every export without UVs until the user learns the UV tools.
- **Controls** (asked; the project lead chose the recommended ones):
  - **the UV Editor moves like the viewport**: the same navigation controls, with orbiting turned into panning, since the editor is flat;
  - **the same keys as the viewport**: Q select, W move, R rotate, S scale, T all, 1/2/3 points, edges, polygons;
  - **seams from menus, with live unwrapping**: Mark seam and Clear seam in the E menu and the UV menu, and islands unwrap again whenever a seam changes. No new keys for now;
  - (the quad menu and other optional extras are unchanged).
- **The rule for every area** (the project lead, 2026-10-16): **keys stay the same everywhere, and the E menu is in every part of the app, with the tools for where the mouse is and what is being worked on.** Over the UV Editor, or in UV mode, E shows the UV tools, in the same ring, with fixed places.
- **Pull requests:** the project lead chose to keep the Phase 6 pull requests open for testing and stack the UV work on them.

## Decision

1. **Phase 7 is now UVs, Phase 8 advanced hard-surface.** README, DESIGN and the interface's "arrives in" notes say so; the decision log keeps the old numbers in older entries.
2. **`mesh::uvs`:** the `uv` corner layer (two numbers per polygon corner) and a new `seam` edge layer (`attributes::SEAM`). It provides:
   - `Uvs`, for reading many UVs at once;
   - islands (polygons joined across edges whose UVs match on both sides);
   - `uv_splits` (edges where the UVs part);
   - `seams`, `is_seam` and `seams_from_splits`.
3. **Primitives come unwrapped** (`primitives::uvs`), worked out from the finished shape:
   - **box** (a cube, by each polygon's main direction, each side seen from outside and upright);
   - **flat** (planes and discs, from above);
   - **round** (cylinders, tubes and cones):
     - ends from their own side;
     - the side unrolled from the back (−X), at its widest ring's length;
     - a tapering side unrolled into a fan around its apex, so it lies flat without stretching;
     - a tube's inside as a strip of its own, mirrored to read right from inside;
   - **ball** (spheres, by angle around and up, at the sphere's size).

   The pieces are packed in rows into the square at one scale, centred, with a gap. Seams are marked exactly where the UVs part.
4. **The phase's milestones**, each its own pull request, in order:
   1. ready-made UVs (this one);
   2. the UV Editor panel, the UV mode and the UV Edit layout;
   3. editing UVs by hand in the editor (select, move, rotate, scale);
   4. seams (marking, from hard edges, painting, suggested);
   5. live unwrapping, relax and straighten;
   6. texture size and texel density;
   7. packing;
   8. checker and stretch heatmap;
   9. UV editing on the model;
   10. UVs that survive modelling;
   11. bake readiness for UVs and trim sheets.

## Consequences

- Every shape exports with UVs, and glTF exports with them carry tangents. Shapes are split along their seams in exports, as UVs require.
- Older projects open unchanged: a primitive's UVs are worked out from its settings, never saved. Shapes made editable before this have no UVs.
- Plain meshes made editable from now on keep their UVs and seams (saved since file format 5).
