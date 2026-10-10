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

None (no other app crates; `tools` only in tests, to aim the camera). Outside libraries: `wgpu` (GPU), `glam` (maths), `bytemuck` (GPU data).

## Status

The first 3D view, see [decision 0013](../../docs/decisions/0013-viewport-rendering.md):

- **`ViewportRenderer`** draws into a **`ViewportTarget`** (sRGB colour, 4× multisampling where supported, depth). `display_view` is the plain-RGBA view egui must be given.
- **Backdrop:** a studio sky over a slightly darker ground, drawn from each pixel's view direction (the inverse view-projection is in the frame uniforms), so the horizon follows the camera.
- **Grid** on the ground (X–Y plane, Z up), finite: it fades out around `FrameParams::focus` (the point the camera looks at), reaching farther as the camera zooms out (`GRID_MIN_REACH`, `GRID_REACH_PER_DISTANCE`). Drawn per pixel: 10 cm, 1 m and 10 m lines (strengths in `GRID_STRENGTHS`) that fade with distance and density, out to 100 m. `FrameParams::fine_grid` switches the 10 cm level off.
- **Axes:** X red and Y blue on the ground, Z green rising from the origin to 100 m (decision 0012), drawn in 1 m pieces (long lines that pass behind the camera are dropped by some drivers).
- **Shapes** (`Shape`): shaded faces with soft, wrapped studio light (gentle gradient across faces, slightly darker near the ground) and very faint white edges. `Shape::test_cube` (2 m, on the grid at the origin) is shown until objects exist.
- **`read_pixels`** copies a finished image back, for tests.

Tests: the shader is validated without a GPU; `tests/gpu.rs` renders real frames (skipped without a GPU unless `REQUIRE_GPU` is set) and checks the sky above the horizon and the ground below it, the cube's shading order, the axes' colours, grid lines, the fine grid switch, and resizing.
