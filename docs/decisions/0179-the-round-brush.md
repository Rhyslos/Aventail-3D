# 0179. Paint mode and the round brush

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's second milestone ([0177](0177-texture-painter-plan.md)): painting on the model. It decides how a brush stroke lands on a texture, which is what makes or breaks painting across UV seams, and how painting fits the app's modes, undo and controls.

## Options

1. **Paint the screen, then copy into the texture** (Blender's projection painting): render which texel is under each screen pixel and write the brush there. Seams need special handling, and big textures get slow.
2. **Paint each dab onto the nearby triangles in texture space** (chosen, from the research): every texel a nearby triangle covers is placed on the surface, seen from the camera, and weighed by its distance from the dab's middle on screen. Both sides of a seam sit at the same place on the surface, so both are painted. Hidden texels are found against a depth map of the shape made when the stroke begins.

## Decision

- **Paint mode**, a sixth button on the mode palette (after UV, a brush), as chosen in the plan. Entering it shows the Painted surface (put back on leaving) and hides the modifiers palette; Attributes shows the **Paint brush** above the selection's settings.
- **The brush** (`paint::Brush`): radius on screen, or in centimetres on the surface (the same real size on every object, with a warning when smaller than a texture pixel there); flow, hardness, spacing, colour; paint through. Falloff is full inside the hard part, then smooth to the edge.
- **Strokes** (`paint::stroke`): dabs every *spacing × diameter* along the mouse's path; within a stroke each texel keeps its largest coverage, laid on the layer as it was when the stroke began with the brush's flow (no piling up within a stroke, building up across strokes). The eraser takes alpha away the same way. Texels within 0.75 of a triangle's edge are painted too, so pieces' edges are covered. Hidden texels (`DepthMap`, the shape drawn from the camera on the CPU at up to 1024 wide, compared with a little slack) and those facing away are skipped unless painting through; surfaces seen edge-on take less.
- **Where it paints:** the texture set of the material under the brush when the stroke begins (every slot of the object with that material's name), its top layer; a "Paint" layer and the set itself are made if missing. Only the shape under the brush is painted.
- **Undo:** the layer is given to the document every frame of the stroke with `Command::SetTextureSet`, merged under the stroke's key into one step.
- **Kept strokes:** each stroke's brush and dabs (place, way towards the camera and radius, in the shape's space) are kept with its layer (`Layer::strokes`) and saved (project format 65), so later milestones can paint them again at another texture size or on a changed shape.
- **Controls** (Substance Painter's, the project lead's choice): left drag paints; 1 paint, 2 erase (3 to 6 note what's coming); [ and ] size; Ctrl + right drag size (sideways) and hardness (up and down); Ctrl + left drag flow (sideways); Esc leaves Paint mode. Ctrl + drags don't navigate in Paint mode. Navigation keeps its own buttons.

## Consequences

- No seam joins on the model, even where pieces lie far apart on the texture.
- The depth map is the painted shape only: another object in front doesn't block the brush yet.
- Painting gives the scene a new texture set every frame of a stroke; tiles are shared, so this is cheap, but very large textures are checked in milestone 10.
- Paint mode's keys aren't in the Keys window yet; they come with the brush bar (milestone 8).
