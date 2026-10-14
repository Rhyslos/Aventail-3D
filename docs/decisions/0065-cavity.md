# 0065. Cavity: ridges and creases picked out on screen

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0063](0063-wireframe.md) (the look row), [0064](0064-automatic-hard-edges.md) (automatic hard edges)

## Problem

The project lead loves Blender's **Cavity**: "I don't know what it really does, but it makes the models nice and easy to work with". It brightens ridges and darkens creases, so a single-grey hard-surface model reads clearly.

They also asked to move Automatic hard edges (0064) from the viewport's Display menu to the main Mesh menu: "it doesn't make sense for auto-hard edges by angle to be in display. It should be in mesh."

## Options

1. **Per-point curvature, worked out on the shape:** soft and slow to update, and it smears across the large flat polygons of hard-surface models.
2. **On screen, from the surfaces' facing** (chosen, like Blender's screen-space cavity): read how the surface's facing changes from pixel to pixel. It catches every hard edge crisply, costs the same whatever the shape, and needs no work on the mesh.

## Decision

1. **How it's drawn:**
   - While the surfaces are drawn, the renderer also writes each pixel's facing across the screen (right, up, towards the camera) into a facing image (`FACING_FORMAT`, half floats; multisampled and smoothed like the colour).
   - Then one triangle over the view (`cavity_fs`) compares the facing a pixel and two pixels either side, across and down. Where it spreads apart (a ridge), a light line is laid over the surface; where it closes in (a crease), a shadow. The outline of a shape, against the backdrop, is neither.
   - Only then are the grid, edges and overlay drawn, so they aren't shaded.
   - `ViewportRenderer::set_show_cavity`. The frame uniforms gain the screen's right and up directions.
2. **On as standard** (`ViewOptions::hide_cavity`, false as standard).
   - Switched by a **Cavity** button beside Shading in the bottom-left row (its dot lit while on), and by **Display → Cavity**. *The button was taken out at the project lead's request, and Cavity sits with Hard edges in the Display menu, [0066](0066-clickable-axes.md).*
   - Remembered.
   - Nothing in Wireframe.
3. **Strength:** light lines at up to 45%, shadows at up to 70%, tuned by eye on a panelled box.
4. **Automatic hard edges moves to the main Mesh menu**, just above Hard edges by angle. It works the same and is still remembered. The Display menu keeps only how things are shown.

## Consequences

- Hard-surface details read at any distance, at a fixed cost per pixel.
- Every frame draws into one more image and runs one more short stage. That stays small next to the surfaces themselves.
- Low-poly curves show a line at every edge, because they really do turn there.
- A softer, wider "world" cavity (Blender's other kind) could be added later as a strength or radius setting, if wanted.
