# 0219. Fixes from the second review round

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

After the fixes of [0218](0218-review-fixes.md), two reviewers went through the app again from modelling to painting. Most fixes held; they found new problems, a few of which needed a choice.

## Decision

Bugs fixed (no choice involved):
- **Bevels over a seam** (Bevel, Game bevel, Live bevel) blended the UVs of the two pieces either side, so the strip, and the corner polygons where strips meet, stretched across the whole texture. A strip over a seam now takes its UVs from the polygon on its near side only, and a corner polygon from the first polygon round its point, as the seam moves to the strip's far border ([0104](0104-uvs-that-survive-modelling.md)). Without a seam nothing changes: the polygons either side share their UVs there.
- **A release and the next press in one frame** (a busy app) looked like the button held throughout, joining two strokes. The brush now reads the button's own presses and releases.
- **Spacing for a brush measured on the object or the texture** was worked out from its screen size setting, not its size where it lands, leaving a line of dots. Spacing now follows the size of the last dab.
- **Planar** dabs ran down the sides of an edge (the fade now takes the surface's turn into account), the views redrew only when the mouse moved after a change, and an edge highlight was too thin to see at its first width (now 40%).
- **Cancel in the type-a-path window** for an export says nothing was exported, as closing the system's window does; the baked maps wrap onto more rows in a narrow panel; Open recent is written to the settings file at once, not only every 30 seconds and on closing.

Choices made:
- **Switching a rebuilding modifier off again** (Subdivide, say) right after it moved the paint, with no painting in between, gives the earlier paint back exactly and at once, instead of moving it a second time (which took about a minute on a dense shape and softened it). The app keeps, per material, the last move's before and after; any other change to the paint forgets it.
- **The polygon fill tool** shows a small cross under the mouse, not the brush's circle: it fills whole polygons, so a circle promised a size it doesn't paint.

## Consequences

- A bevelled strip over a seam has no room of its own on the texture (as one without a seam never had): paint on it is the colour along its near edge. Painting detail on bevels still means baking them into the shape and unwrapping again, as the manual says.
- A click on the type-a-path window was suspected of painting the model underneath; a test shows it doesn't, and stays to keep it so.
- Still not reproduced: undo and redo bringing back an extrude at a different distance. Left for later: overlapping UV pieces aren't pointed out while painting; a few panel-space and clipping polish points.
