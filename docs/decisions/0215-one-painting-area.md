# 0215. The viewport and the UV Editor as one painting area

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

The floating brush bar and the paint tools (decision 0213) were drawn over the viewport alone. In the Paint layout the viewport and the UV Editor sit side by side and both are painted in, so they are one place to paint, and the mockup the project lead chose showed the bar across both. Tied to the viewport, the bar wrapped into two or three rows and the tools moved up when the viewport was made narrow, while the room over the UV Editor went unused. The project lead found this on 2026-10-29.

## Decision

- Each view says where it was drawn every frame (`paint_area::PaintPanes`). When the UV Editor sits beside the viewport (touching it, sharing at least half its height) the two are one painting area; otherwise (UV Editor closed, below, or elsewhere) the area is the viewport alone.
- The brush bar lies over the area's top left, under both views' headers, and only wraps when the whole area is too narrow.
- The paint tools lie along the area's bottom, level with the viewport's Shading button, in the free stretch nearest the area's middle that doesn't cover the Shading and navigation buttons or the UV Editor's notes; with no stretch wide enough, just above the Shading button.
- Both are drawn after the views, on their own layer above both, so a click on them never paints in either view.
- No keys or mouse actions.

## Consequences

- Resizing the views no longer squeezes the bar and the tools while there is room over the UV Editor.
- The paint tools in the left bar (the setting) are unchanged.
