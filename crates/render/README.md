# render

**Layer 3 · Front end**

The viewport renderer: shaded meshes, wireframe overlay, selection highlights, grid, and GPU picking.

## Purpose

Draws the scene with wgpu into each viewport's own image: the calm gradient background, a fading grid, soft studio shading, anti-aliased wireframe, selection and hover highlights, the checker texture in UV mode. Also answers "what is under the cursor?" with an ID buffer and depth readback, which powers picking and orbiting around the point under the mouse. See DESIGN.md §4.6 and §5.

## Must never

- Change the scene: it only reads it
- Re-upload meshes that did not change
- Block the interface thread waiting on the GPU

## Depends on

`scene`, `mesh` (planned).

## Status

Empty. The viewport arrives in Phase 1.
