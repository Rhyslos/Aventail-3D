# bake

**Layer 1 · Core**

Maps baked from a shape itself, for the texture painter: position, normals, occlusion, curvature, ID and thickness.

## Purpose

Maps baked from a shape itself, for the texture painter ([decision 0192](../../docs/decisions/0192-the-baker.md)): position, normals, occlusion (ambient occlusion), curvature, ID and thickness, each a `paint::Image` the size of the texture set, worked out for every texel the shape's polygons cover. No high-detail shape is needed: everything comes from the shape's own polygons.

## Must never

- Know about scenes, undo, files, the GPU or UI
- Bake outside the polygons of the slots asked for, or panic on a shape with no triangles
- Hold the window up: callers bake on another thread, and the work here is shared over the cores

## Depends on

`mesh`, `paint`, `glam`.

## Status

- `bake(sources, kind, settings)`: `Source` (a `paint::PaintSurface`, its slots on the texture set, the size), `BakeKind` (`ALL`), `BakeSettings` (rays a texel, reach and edge width as shares of the shape's size). Texels are shared out over the cores (`mesh::parallel::in_parallel`); uncovered texels stay see-through.
- `texels` finds the covered texels (as painting does, 0.75 texel past each edge so seams stay covered) with their place, facing and slot.
- `Rays` (`rays.rs`): a bounding volume hierarchy over the triangles; `hit`, `openness` (cosine-spread rays out over the surface; the share that meet nothing within the reach) and `thinness` (rays in through the surface; how far before they leave).
- `curvature.rs`: folded edges (two triangles meeting at more than about 9°), light along outer ones and dark along inner ones, fading over the edge width; flat and gently curved surfaces mid-grey.
- `gradient` (decision 0194): a gradient fill's pixels, `paint::Gradient` laid along each object's own box on the texels it covers, or each part's or UV piece's (`groups`, decision 0199).
- `smart_mask` (decision 0195): a smart mask's pixels from the shape (curvature for edges and cavities, facing, height), with 3D value noise and a smoothstep border; see-through off the polygons.
- `lighting` (decision 0196): a light-and-shade layer's pixels: each texel lit by how it faces the sun, shadows by rays toward it (`Rays`), light to shade colours.
- `id_colour`: a clear colour per material slot (hues a golden-ratio turn apart).

Tests in `src/tests.rs`: position and normals, occlusion open on a lone cube and dark where two boxes meet, curvature on edges and grey on faces, ID colours, thin against thick, rays, uncovered texels and an empty shape.
