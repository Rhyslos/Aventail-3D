# 0058. Closed slits: cutting a shape into a polygon

- **Status:** Accepted
- **Date:** 2026-10-14

## Problem

While testing Slit, the project lead tried to cut a hole into a polygon, not from its edge. A slit's line has to go into a polygon and out again to cut it, so a line drawn wholly inside one polygon cut nothing. He wants this, and called it very important: a window in a wall, a button on a panel, a vent on a box.

## Options

How to close the line:

1. **Click the first point again** (with three or more points). Common in 3D programs' knife tools and in drawing programs; no new key.
2. A key (Enter with a modifier, say) to close the line. One more key to learn.

How to cut a shape that lies wholly inside a polygon, as a polygon can't have a hole:

1. **Cut the ring round the shape in two**, with two new edges from the polygon's corners to the shape, on opposite sides. Every polygon stays a simple polygon, the shape is its own polygon, and the result draws, picks and exports like any other.
2. **One bridging edge** from a corner to the shape, making a polygon that touches itself along that edge. Fewer polygons, but such a polygon breaks many tools and exports.
3. **Fan the ring** into triangles or quads round the shape. Tidier for some uses, but many more polygons the user didn't ask for.

How to save a closed line:

1. **The first point again at the end of the line.** Old files never have it, and nothing new needs saving, so the file format stays at version 13.
2. A new `closed` switch on the slit, needing file format version 14.

## Decision

- **Click the first point to close**, chosen by the project lead. A click on a line point that doesn't move the mouse (within 3 screen points) is a click, not a drag: on the first point, with three or more points, it closes the line and finishes it. Hovering the first point shows the closing stretch.
- **A loop wholly inside one polygon** cuts its shape in as a polygon of its own, and the ring round it in two halves. One new edge goes from the polygon's corner nearest the shape to the shape's point facing it. The other goes from the corner most nearly opposite to the point facing that. If either half would cross itself (a star-shaped loop), other corners and points are tried, nearest first, a limited number of times; if none fits, that loop isn't cut. The shape keeps the polygon's way round, whichever way the loop was drawn.
- **A loop crossing polygons** cuts like an open line, the stretch back to the start included. Where the loop starts inside a polygon, the cut there runs from where the loop last comes in, through the start, to where it first leaves.
- **Saved as the first point again at the end** (`SlitLine::is_closed`), so no format change.

## Consequences

- Shapes can be cut into polygons and then extruded, inset or deleted, without building them edge by edge.
- The two new edges across the ring are placed by the app; a later "connect to" choice could let the user pick them if needed.
- A project with a closed slit, opened in an app from before this change, cuts the loop as an open line ending at its start: the shape inside one polygon isn't cut there. Projects open in older versions only on a best-effort basis anyway.
- A click on a line point no longer nudges it to where the mouse was.
