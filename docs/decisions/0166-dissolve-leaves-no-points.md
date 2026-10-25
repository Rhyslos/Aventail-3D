# 0166. Dissolve leaves no points behind on curved surfaces

- **Status:** Accepted
- **Date:** 2026-10-25
- **Replaces part of:** [0052](0052-clean-up.md) ("points left in the middle of a straight line by the dissolve go too … corners of a real turn stay")

## Problem

The project lead found that dissolving edges **sometimes left their points behind**: dots on a line with only two edges, which add nothing and get in the way of later edits. It happened on a sphere and not on a flat grid. Decision 0052 removed a leftover point only when its two edges lie in a perfectly straight line. On a curved surface every line bends a little at each point, so nothing there counted as straight.

## Options

1. **Always remove every point a dissolve leaves between two edges**, as Blender does. Simple, but dissolving a cube's edge (after the sharp-corner question) would also take away the cube's corners and wreck its shape.
2. **Remove a leftover point unless it is a corner of the shape**, measured by how sharply the polygons around it meet.

## Decision

Option 2. A point a dissolve leaves between just two edges goes when either:

- its two edges are in a straight line (as before), or
- it is inside the surface (not on an open border) and the polygons around it meet no more sharply than **60°** (`mesh::cleanup::CORNER_ANGLE`).

60° keeps the real corners: a cube's corners and a cylinder's rims, both at 90°. Even a blocky 8-sided ball (45°) loses its leftover points. Points on an open border still go only on a straight run, so a hole's outline keeps its shape. The same applies to points left by dissolving polygons.

## Consequences

- Dissolving a loop or a run of edges on a sphere, a cylinder's side or any rounded shape now leaves clean polygons with no stray points.
- The shape changes very slightly where a point on a curve goes: the polygon's outline cuts straight across where it used to bend through the point. This is what other 3D programs do and what a modeller expects from a dissolve.
- Old projects replay their dissolves with the new rule when opened, so a dissolve made before this may now take away points it used to leave.
