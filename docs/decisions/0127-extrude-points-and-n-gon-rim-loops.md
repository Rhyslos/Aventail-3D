# 0127. Extrude from points, and loops round an n-gon's rim

- **Status:** Accepted
- **Date:** 2026-10-22
- **Builds on:** [0038](0038-extrude-and-inset.md) (extruding border edges), [0029](0029-selection.md) (loops and rings)

## Problem

The project lead, testing the rework (2026-10-22), found two things in the way of everyday modelling:

1. A double-click on the top or bottom edge of a standard cylinder selected that one edge, not the rim. The rim's points have three edges (two round the rim, one down the side), so the loop couldn't go "straight on through four quads", and it stopped. He had to inset the cap first.
2. Extruding was impossible from the side. In a side or front view, edges seen end-on are hard to pick, but points are easy. He asked for Extrude on selected points to build new polygons between them, as Blender does, so a profile can be modelled straight from an orthographic view.

## Options

For the loop:
1. **Run round the rim of the one n-gon the edge borders, where the loop can't go straight on (chosen).** This is what Blender does, and it matches how an open border's loop already runs round the border.
2. Leave loops stopping at n-gons and expect an inset first.

For extruding points:
1. **Points mode Extrude grows the open-border edges whose two points are both selected, as the Edges mode does, then selects the new outer points (chosen).** No new key (D, as everywhere), no new kind of edit, and the file format is unchanged: it is recorded as an edge extrude.
2. The new points follow the mouse freely in the view, as in Blender. That would need a new kind of edit with a free direction. It can come later if growing straight out (then moving with W) isn't enough.
3. Lone points making lone edges. Not possible: every edge in the app belongs to a polygon.

## Decision

- `Topology::edge_loop` runs round an n-gon's rim when the loop can't go straight on and the edge borders exactly one n-gon. It still stops where an edge borders two n-gons, and at triangles and poles away from an n-gon.
- In the Points mode, Extrude (D) works on the open edges between selected points. A point on its own, or points with no open edge between them, grow nothing, and the status bar says why.
- In the Points and Edges modes, when the selection goes all the way round polygons (every corner, or every edge), Extrude pushes those polygons out as the Polygons mode does, and selects their new rim. The project lead found this was needed straight away: a closed cap's rim has polygons on both sides, so it had nothing to grow from (the same rule as Blender's).

## Consequences

- Double-clicking a cylinder's cap rim, in the Edges or Points mode, selects the whole rim.
- A shape can be drawn out a strip at a time from points, then shaped with W.
- Edges inside a shape (polygons on both sides) that don't go all the way round a polygon still don't extrude. Growing them as a fin, as Blender can, is left out: it makes a shape that isn't closed.
