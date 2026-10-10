# 0013. Viewport rendering

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

The viewport needs real 3D drawing on the GPU, sharing the window's wgpu device, in the calm look of the approved mockups. It will grow a lot (meshes, selection, gizmos, UV checker, several viewports), so the first version sets the pattern. Halberd solved the same problem two days earlier.

## Options

1. **Draw inside egui's own render pass** with a paint callback: no extra image, but no depth buffer or multisampling of our choosing.
2. **Draw into an offscreen image per viewport** and show it in egui as a texture: one extra copy per viewport, but full control over depth, multisampling and formats, and the renderer knows nothing about egui. This is what Halberd does.

## Decision

Option 2, following Halberd:

- **`render` draws into a `ViewportTarget`** (sRGB colour, 4× multisampling where supported, depth). `app` registers the image with egui; `ui` shows it through the `ViewportRenderer` trait, so the panel and its tests need no GPU, and a machine without a usable GPU shows the background and a plain-language reason.
- **egui reads a plain-RGBA view of the sRGB image**, as Halberd found: handing egui the sRGB view would decode colours twice and darken everything.
- **The camera is ours** (`tools::Camera`): right-handed, Z up (decision [0012](0012-z-up-axes.md)), centimetres, depth 0 to 1 as wgpu expects, 45° field of view. It opens looking at the origin from the front, right and above, as Cinema 4D opens a new scene.
- **The look follows the mockups:** the theme's grey gradient as the background; a grid on the ground drawn per pixel with three levels (10 cm, 1 m, 10 m) that fade with distance and density, never ending in a hard edge or shimmering; the X axis in red and Y in green across the ground; shapes with soft studio shading (a sky-and-ground fill and one key light, so top, front and side each get their own calm shade) and faint white edges.
- **The ground axes are drawn in 1 m pieces.** A single line from one side of the grid to the other often passes behind the camera, and some graphics drivers (found with Mesa's software Vulkan driver) then drop it entirely; short pieces always draw and fade smoothly.
- **A test cube** (2 m, standing on the grid at the origin) shows until objects exist in Phase 2.
- **Tested on a real GPU in CI**, with the software Vulkan driver: the gradient's colours, the cube's shading order (top lightest, then front, then side), the axes' colours, and grid lines being lighter than the ground between them.

## Consequences

- One extra viewport-sized image per viewport on the GPU; negligible.
- Navigation (orbit, pan, zoom, cursor-point pivot) is the next milestone; until then the camera stays at its opening view.
- Shading is a fixed studio light for now; a matcap option and selection highlights come with later milestones.
