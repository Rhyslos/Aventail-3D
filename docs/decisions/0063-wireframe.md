# 0063. Wireframe, and the viewport's bottom-left corner

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0028](0028-ask-about-controls.md) (ask about controls), [0053](0053-hard-and-soft-edges.md) (the Display menu)

## Problem

The project lead asked for four more Phase 4 milestones, backfilled after the keyboard shortcut viewer:

1. a wireframe view;
2. cavity, as in Blender;
3. clicking the corner axes to turn the view;
4. automatic hard edges by angle.

They were also unsure where the corner axes should sit, and wanted the display settings near them. This record covers the first of these and the choices made for all four.

## Options

- **Where the axes and display settings go:** keep the axes top right with the settings only in the Display menu; move the axes to the bottom left alone; or **move them to the bottom left with a small row of display buttons beside them** (chosen).
- **What clicking an axis does:** **swing the view to look along it, staying in perspective, with a second click looking from the other side** (chosen, like Blender's); switch to the matching flat view; or both, by click and double-click.
- **A key for the look:** **none for now** (chosen); `4` to cycle; or `Shift + Z` as in Blender, which would move the Z lock.
- **Automatic hard edges:** **per object, on by default, with an angle (30°), saved with the project** (chosen, like Blender's Auto Smooth); one switch for the whole project in the Display menu; or a display-only switch that exports ignore.

## Decision

1. **Three looks**, in the Display menu and from a button in the viewport's bottom-left corner:
   - **Shaded** (the standard): as before.
   - **Shaded with wires**: every edge drawn clearly in light grey over the surfaces.
   - **Wireframe**: only the edges, seen through the shape; the renderer leaves out the faces (`ViewportRenderer::set_show_faces`).

   The choice is `ViewOptions::shading`, for every view, and is remembered. Selected objects keep their accent edges in every look. In Wireframe, dense shapes keep a faint outline from afar instead of vanishing, and clicking still picks by the faces.
2. **The bottom-left corner** holds the axis marker, with the grid spacing above it and the look row beside it. The navigation picker stays bottom right. Cavity's button joins the row in its own milestone.
3. **No key** for the looks.
4. The **other three milestones** follow in this order, each in its own pull request:
   - **Cavity:** a Display switch and a button in the row.
   - **Clickable corner axes:** a click swings the view to look along that axis, and a second click looks from the other side.
   - **Automatic hard edges by angle:** per object in the Attributes panel, on by default at 30°, saved with the project, with a Display menu entry for the selected objects.

   The hard edge highlight they asked for already exists: **Display → Hard edges**, on as standard.

## Consequences

- The top right of the view is clear.
- Wireframe costs nothing extra to draw: it is the edges the viewport already draws, without the faces.
- Picking in Wireframe still hits faces, so clicking inside a shape selects it. Selecting through, the standard, behaves the same in every look.
