# 0221. Viewport lines and points sized in points

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

The viewport drew its lines and points a fixed number of the screen's pixels wide: the selection outline (decision 0220), selected and hovered edges and points, hard edges, seams and the rest, and objects' plain edges, which were the GPU's own lines, always exactly one pixel. On a screen set to 150% or 200% (most 4K and laptop screens), where the rest of the interface is drawn larger, they looked half as thick. The project lead asked for sizes that look the same on any screen.

## Options

1. **A share of the view** (as a percentage), like some web layouts. A line would grow with a big monitor and shrink in each of the four views, and change as a panel is resized: lines in 3D programs are meant to stay a steady width.
2. **Points**, the unit the interface already uses: a point is one pixel at 100%, two at 200%, set by the system's display scaling. Web pages do the same for their borders (a CSS pixel is a point, not a screen pixel).

## Decision

Points. Every line and point the viewport draws is given in points and turned into pixels with the screen's scale (`ViewportView::pixels_per_point`, kept between 1 and 4): objects' edges are 1 point, the selection outline 2 (3 with High contrast), the rest as before. Objects' edges become strips worked out on the GPU from each edge's ends (read from the mesh's vertices and edges as storage), since the GPU's own lines can't be wider than a pixel. A mesh too big for the GPU's storage limit keeps the one-pixel lines. In the player view (decision 0200) lines shrink with the smaller image, never below a pixel.

## Consequences

- A 4K screen at 200% shows the same weight of lines as a 1080p screen at 100%.
- Edges cost a little more to draw (six corners an edge instead of two); the heavy-mesh check is the place to watch it.
