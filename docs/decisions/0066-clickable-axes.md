# 0066. Clickable corner axes, and Cavity beside Hard edges

- **Status:** Replaced by [0084](0084-view-dice.md) for the clickable axes
- **Date:** 2026-10-14
- **Follows:** [0063](0063-wireframe.md) (the project lead's choice: swing to look along an axis, again for the other side), [0065](0065-cavity.md)

## Problem

The project lead asked for "snapping by clicking on the gizmo" in the corner, as in Blender. In 0063 they chose that a click swings the view to look along the axis, staying in perspective, and a second click looks from the other side.

After Cavity was merged, they asked to move it "to the display dropdown". It was first read as the Shading button's dropdown; they meant the viewport's Display menu, "where hard edges and such is".

## Decision

1. **Clicking a letter:** in a perspective view, the corner marker's X, Y and Z letters are buttons (`Look along X`, …).
   - A click glides the view round its pivot, at the same distance, to look along the axis from its own side. **Z** looks down from above, **X** from +X (the right), and **Y** from +Y (the back).
   - Clicking the same letter while already looking that way looks from the other side.
   - The glide is the same short, eased move as framing, using `tools::Navigator::looking_along`.
2. **Straight down or up:** this stops just short, as orbiting does, and lines up like the Top view, with X to the right and Y up the screen.
3. **Flat views:** their letters stay plain markers, because a flat view keeps its direction.
4. **The marker:**
   - It grows a little: 22-point lines, with each letter in a soft round button of its colour (8-point radius).
   - A hovered letter gets a ring and a pointing hand.
   - The grid spacing text and the look row move to make room.
5. **Cavity** loses its own button in the corner and sits in the Display menu's group with **Hard edges** and **Normals**, the things drawn over the surfaces, first in that group. The bottom-left row keeps only the Shading button.
6. **Heavy shapes**, found by the phase-end heavy-mesh check:
   - Shaded by angle, a move can turn edges sharp or soft. A shape of 50,000 polygons or more then follows the move at once, with its old shading groups, and is reshaded on another thread. Lighter shapes are reshaded at once.
   - Shading groups and face facings are worked out on every core.

## Consequences

- Checking a model square-on takes one click and no change of view.
- The heavy-mesh check's geometry step was 3.1–3.5 s against its 3 s target on the 2-core test machine (2.7–3.2 s shaded by marks). It runs in the background, and about 1 s of it is the mesh's connection table. Making that table cheaper is the way to bring the step back under the target, if needed.
