# 0220. The Edges switch, and the selection outlined

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

The viewport always drew every shape's edges: faint in Shaded, clear in Shaded with wires, and the selected objects' in the accent colour. There was no way to see a shape without lines over it. The project lead asked for a switch for the edges in the shading button's list, ticked by itself in the Edges mode, replacing the Shaded with wires look ([0063](0063-wireframe.md)).

## Options

For how a selected object shows with edges off (asked of the project lead):

1. **An outline** in the accent colour round what it covers on screen, as Blender and Cinema 4D do. Clean, a new pass in the renderer.
2. **Keep the accent edges** on selected objects only. Quick, but edges still show on what you select.
3. **A tint** of the surface only.

For when the switch ticks itself: only in the Edges mode, or in the Points, Edges and Polygons modes; and whether leaving them unticks it.

## Decision

- **Shading** offers **Shaded** and **Wireframe**; **Edges** (`ViewOptions::show_edges`, off as standard) under them draws every edge over the surfaces, as Shaded with wires did. A saved Shaded with wires reads as Shaded. Wireframe always shows every edge, so the switch is greyed out there.
- **The Points, Edges and Polygons modes tick it** on entering them; going back to a whole-object mode unticks it, unless it was on already or was changed by hand meanwhile. A switch ticked only by a mode isn't saved as on.
- **The selection is outlined**: a selected object (not one being edited) marks its surface in the facing image cavity reads (w = 2), and a pass over the whole view after cavity colours the pixels within 2 pixels (3 with High contrast) of a marked one in the accent colour. With Edges on, the selected objects' edges stay in the accent colour too. Cutters (decision 0135) are edges only and keep them.

## Consequences

- Shapes look as a game will show them until you edit their parts.
- The outline follows what is seen: an object in front cuts it, and in the Wireframe look there is no outline (the accent edges show the selection).
- The outline is worked out on the screen's pixels, so its width doesn't change with distance; a very small object far away is a dot with an outline round it.
