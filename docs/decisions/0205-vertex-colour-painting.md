# 0205. Vertex colour painting

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's twenty-sixth milestone ([0177](0177-texture-painter-plan.md)). Vertex colours are colours kept on the shape itself, with no texture or UVs: cheap in a game, common for low-poly and stylised props, and used by shaders as masks. The project lead put them after the texture tools (DESIGN.md, texturing choices).

## Decision

- **Stored per polygon corner** (`mesh::colours`, the corner attribute layer `colour`: four numbers, sRGB and alpha; white where never painted). Corners sharing a point and a colour make a soft blend across polygons; different colours make a crisp change, as low-poly art wants.
- **Painted as a recorded edit** (`Operation::PaintColours`: each corner by polygon and point, with its colour), so it replays with the shape and works with undo. A stroke records the edit at its first dab and replaces it as it grows (`Command::ReplaceOperation`, one merge key), so the model shows the colours while you paint and the stroke is one undo step.
- **In Paint mode, Attributes → Paint on: Texture | Vertex colours.** With Vertex colours:
  - the brush paints the corners within its radius on the model (facing the camera unless Paint through), each mixed toward the brush colour by the brush's hardness falloff and flow (and the pen's pressure); the eraser mixes toward white; other brush kinds paint like the brush;
  - the polygon fill tool (4) fills the corners of a polygon, a UV piece or a material, by Fill by;
  - the views switch to **Display → Surface → Vertex colours** (vertex colours over the material colour by their alpha), and back to Painted when Texture is chosen again;
  - **From the texture** turns the selected objects' painting into vertex colours: each corner takes its material's mixed texture at its UV, over the material's colour, as one undo step;
  - the UV Editor doesn't take strokes (vertex colours belong to the shape); a note says so.
- **Drawing:** `MeshVertex::colour`, split where colours differ like UVs; the shader's surface mode 5.
- **Export:** glTF `COLOR_0` (linear, with alpha) when a shape has vertex colours. OBJ has no standard way to hold them, so they're left out there.
- **Saving:** project format 77 stores the edit (`v37::OperationV37::PaintColours`); the layer itself is saved with any frozen shape like every layer.
- **No keys:** the Paint on choice is in Attributes.

## Consequences

- Vertex colours paint with the same brush, colours, pen and fill tool as textures.
- Mirror painting, smudge and blur don't apply to vertex colours yet. Outline width by area ([0201](0201-outlines.md)) can now be read from vertex colours later.
- A very dense shape takes longer per dab (every corner in reach is looked at); fine for the props vertex colours are used for.
