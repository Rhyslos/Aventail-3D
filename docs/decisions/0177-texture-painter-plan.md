# 0177. The texture painter: plan, layout and controls

- **Status:** Accepted (the project lead's choices)
- **Date:** 2026-10-28

## Problem

Texture painting (Phase 11) needed its planning talk before it could get milestones (DESIGN.md, "Texturing and a material builder"). The project lead asked to build it next, ahead of the rest of Phase 8.

## Options

The research (what game texture artists use in Substance Painter, 3DCoat, Blender, ArmorPaint, and how World of Warcraft and League of Legends textures are painted) and a written plan were reviewed with the project lead, with four interface mockups. The choices that shaped it:

1. **Where painting lives:** a Paint mode in the left palette (chosen), or a separate workspace.
2. **How close to Substance Painter:** "game-first, not a Painter copy" (the old design line), or **very similar to Substance Painter in most regards** (chosen), built the app's own way, with settings in the Attributes panel.
3. **Order against Phase 8:** finish Phase 8 first, or **painting now, bringing forward only the bakes the painter needs** (chosen); the rest of Phase 8 follows the painter.

## Decision

- **33 milestones in six groups** (the core; paint that follows the model; the bakes the painter needs; the hand-painted kit; export and sharing; realism), hand-painted first. The working plan is [docs/plans/phase-11.md](../plans/phase-11.md).
- **Ideas that set the app apart**, which only an app that owns the modelling and the UVs can do: paint that survives modelling, re-packing or re-unwrapping without losing paint, and changing texture size without blur (strokes kept as dabs). Also: brush size in centimetres, seeing the model as the player will, painting trim sheets and kits live, engine-true previews, vertex colour painting, a value check, automatic warm light and cool shade, and "Why does this look wrong?". The timelapse was left out.
- **Layout (mockup A):** the 3D view and the UV Editor side by side, the brush bar in Coordinates' place, no modifiers palette, Attributes (brush and colour) over Paint layers.
- **Controls:** Substance Painter's, and Substance Painter's whenever in doubt (listed in the plan). In Paint mode, 1 to 6 pick the paint tools instead of switching modes.
- **Colour picker:** built on the project lead's Nodelit picker, expanded for painting; to be made more compact.
- **Pen:** Wacom first (Wintab), other pens through Windows.
- **Painting technique:** each brush dab is rasterised onto the nearby triangles in texture space, so both sides of a UV seam are painted and joins don't show; tiled layers with tile undo.
- **Working:** one milestone per pull request, each merged as soon as its checks pass (the project lead's standing OK), test steps added to the testing document; no pauses for feel checks.

## Consequences

- The design's "not a Painter copy" line is replaced: people coming from Substance Painter find the same controls.
- The baker starts in this phase (from the shape itself; no high-poly bakes yet), and Phase 8's baker grows from it.
- Brush feel is judged only at the end by the project lead; problems found then are fixed afterwards.
- Phase 8 resumes after Phase 11.
