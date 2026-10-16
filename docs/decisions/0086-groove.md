# 0086. Groove: a thin line cut in or raised out along edges

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** the plan in [0083](0083-groove-and-bevel-control.md), building on [0085](0085-bevel-control.md) (bevel control) and [0040](0040-bevel.md) (bevel)

## Problem

To cut a panel line the project lead had to bevel edges into a thin strip, select the strip's polygons and extrude them inward: three steps, two modes, and the result was two edits that couldn't be changed together. He asked for one tool (0083): select edges, drag in for a groove or out for a ridge.

## Options

- **How it's made:**
  - **a bevel at half the width on each side (one segment, or two for sloped sides), then the strip pushed in or out** (chosen). It reuses two tested edits, so ids, materials, hard edges and stretched objects behave as they already do, and Stop at overlaps and Weld points keep crossing lines tidy;
  - a new cut that builds the strip and walls directly, which would mean re-solving everything the bevel already handles (corners where lines meet, ends, overlaps).
- **Straight sides:**
  - **the strip extruded together along its facing** (chosen): walls square to the surface, a flat bottom;
  - each polygon pushed along its own facing, which splits the strip apart on curved surfaces.
- **Sloped sides:**
  - **a two-segment flat bevel whose middle line is moved by the depth along its facing** (chosen): a V, with no extra walls;
  - a straight groove with its bottom narrowed, which needs a third setting.
- **Ends inside a surface:** the bevel leaves a triangle with no area there (the old point and the two points that slid from it along a straight edge).
  - **The old point joins the strip's end polygon and the triangle goes** (chosen), so the groove ends in a clean wall;
  - leaving the triangle, which shades badly and breaks the extrude's walls.
- **Where it sits in the tool menu:**
  - **the last place, at the top left beside Loop cut** (chosen), so Delete keeps the place it has in every mode's ring;
  - beside Bevel, which would move Delete in the modelling ring away from its place in the Model ring.

## Decision

1. **`mesh::groove`** (re-exported from `edit`): `groove_edges(mesh, edges, GrooveShape)` with `GrooveShape { width, depth, sides: Straight | Sloped }`, the width up to `MAX_GROOVE_WIDTH` (1 m). It bevels at half the width with clamp and weld on, closes flat ends (`close_flat_ends`), then extrudes the strip together (Straight) or folds its middle (`fold`, Sloped). Edges without a polygon on each side are skipped, as bevels skip them.
2. **`Operation::Groove { edges, shape, stretch, anchors }`** in `scene`: described as "Groove along N edges" (or "Ridge" when the depth is above 0), worked out as seen on stretched objects, mirrored as one edit together with its mirror (like a bevel), with Depth, Groove width and Sides changeable under Last edit (`EditSettings::groove`).
3. **The tool** `Tool::Groove` on **K** (the project lead's choice), in the Edges mode, in the Mesh menu, the Tools menu and the tool menu. The drag measures along the surface's facing: into it cuts a groove, out of it raises a ridge. The Attributes panel shows Groove width and Sides before the drag (standard 1 cm, straight) and under Last edit with the Depth.
4. **File format 24** stores a groove (on its own or mirrored) beside everything version 23 stores. Older versions refuse a groove rather than drop it.

## Consequences

- A panel line is one edit and one undo step, and its width, depth and sides can be changed afterwards.
- Where a groove reaches a face's edge, its end wall sits flush with the side; carrying it round a corner means selecting the edges there too.
- A rounded groove is a groove followed by a bevel of its rim; a profile setting can come later if wanted.
- The tool menu has 15 places in every mode.
