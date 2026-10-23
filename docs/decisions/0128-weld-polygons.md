# 0128. Weld polygons

- **Status:** Accepted
- **Date:** 2026-10-22
- **Builds on:** [0051](0051-connect-bridge-weld.md) (Weld, M and Ctrl + M)

## Problem

The project lead wanted an easy way to make a cylinder's cap into even triangles: inset the cap, select the middle polygon, and weld its points into one at the centre. Weld only worked in the Points mode.

## Options

For **M** with several polygons selected:
1. **Each group of touching polygons melts at its own middle (chosen, the project lead's choice).** Polygons sharing a point are one group, so separate details each collapse in place, like Blender's Collapse.
2. Every selected polygon's points melt into a single point.

For **Ctrl + M**, the project lead asked for the same as with points: press on a polygon and drag it onto a point.

## Decision

- Weld works in the Points and Polygons modes. In the Polygons mode, M records one weld at the middle per group of selected polygons sharing a point, all in one undo step.
- Ctrl + M in the Polygons mode makes Weld the tool: a drag from a polygon onto a point welds that polygon's points (every selected polygon's, if it was selected) onto the point, which stays put.
- No new keys, and no new kind of edit: both are recorded as the existing weld (the points, and a target or none), so the file format doesn't change.

## Consequences

- An inset cap's middle polygon welds into a fan of even triangles.
- M on a polygon with nothing round it to close in (a plain cube's top) makes a point, like a pyramid's tip; Ctrl + Z takes it back.
