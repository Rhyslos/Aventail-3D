# 0077. The bake check (model part)

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** [0052](0052-clean-up.md) (clean-up), [0073](0073-mikktspace-tangents.md) (tangents) and [0075](0075-export-presets.md) (presets)

## Problem

A bake in Substance Painter can go wrong for reasons that are hard to see in the viewport:

- a polygon facing the wrong way;
- points doubled on the same spot;
- a bent many-sided polygon.

You only find out after exporting, baking and looking at the normal map. The app should say so before you export, in plain words, with a fix where there is one (DESIGN.md §8, wish W13).

## Options

- **Where:**
  - inside the Export window only;
  - **its own window, from File → Check for baking…, and a button in the Export window when Made for is Substance Painter** (chosen). No new keys, as the project lead chose for exporting.
- **What it checks** (the model part; UV checks come with the UV tools):
  - polygons facing the wrong way;
  - edges shared by three or more polygons;
  - polygons with no area;
  - edges with no length (points on the same spot);
  - loose points;
  - **many-sided polygons (five or more corners) bent out of flat** (a corner more than 5% of the polygon's size out of its plane) **or cut into sliver triangles** (a corner under 2°, with the viewport's cut);
  - `_low` / `_high` objects without their partner, the names Substance Painter pairs by.
  - Quads are left alone: a thin quad is normal (a bevel strip), and a slightly bent one shades predictably.
- **Fixes:**
  - **one click where the fix is certain:** Fix normals for polygons facing the wrong way, delete loose points, weld each group of points on the same spot;
  - **Select** for everything else, since cutting up an n-gon or deciding which of three polygons to remove is the modeller's call.
- **When a fix makes something else worse** (welding can leave three polygons on an edge): undo it silently, or **do it and say so** (chosen), so nothing is hidden and Ctrl + Z is one key away.

## Decision

1. `mesh::bake::BakeReport` lists the problem elements of a shape (`new`, `is_clean`, `short_edge_groups`). `BENT_SHARE` is 5%, `SLIVER_DEGREES` 2°. Every primitive is clean (tested).
2. `ui::workbench::baking`:
   - **the window** lists each problem as *Object: what*, in amber, with the reason on hover, **Select** (the object, the right mode, the elements), and **Fix** where there is one;
   - fixes are recorded edits on the object, one undo step, labelled *Fix …*;
   - the list is worked out again whenever the scene changes;
   - instances are checked through their original.
3. The status bar says what a fix did, and anything it made worse.

## Consequences

- The UV part (hard edges without seams, overlapping UVs, padding) joins the same window in Phase 8.
- Problems are worked out on demand, while the window is open, never in the background.
