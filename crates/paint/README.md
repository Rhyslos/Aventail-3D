# paint

**Layer 1 · Core**

Texture painting: texture sets by material, their layers, tiled images, and mixing the layers into what a surface shows.

## Purpose

Holds what a texture painter paints (DESIGN.md, Phase 11; [decision 0177](../../docs/decisions/0177-texture-painter-plan.md), [decision 0178](../../docs/decisions/0178-textures-on-materials.md)): every material's **texture set**, its **layers**, and the mixed image the views show. Everything here can be tested without a window or a GPU.

## Must never

- Know about scenes, undo, files, the GPU or UI
- Allocate an image of a size outside `image::SIZES`, or panic on a tile of the wrong length
- Copy a whole image to change part of it: tiles are shared and copied only when changed

## Depends on

`mesh`, `words` (the language layer, [decision 0134](../../docs/decisions/0134-shared-language-crate.md)), `glam`.

## Status

- **Images** (`image`): `Image`, square (16 to 16384 pixels), RGBA bytes (sRGB, straight alpha) in `TILE` (64) pixel square tiles, each an `Arc<Vec<u8>>` or none (transparent); `pixel`, `set_pixel` (copies a shared tile first), `tile`, `set_tile`, `tile_area`, `to_rgba`, `from_rgba` (stretched smoothly to the size; fully transparent tiles left out), `memory_size`. Row 0 is the top of the texture (V = 1).
- **Texture sets** (`set`): `Layer` (name, visible, opacity, `Blend::Normal`, `Content::Paint(Image)`), `TextureSet` (size, layers bottom first; `is_valid`, `free_name`), `PaintStore` (sets by material name, each `Arc` so unchanged ones are shared).
- **Mixing** (`composite`): `Composite::of(set)` and `update(set)`: the layers mixed over each other (straight-alpha "over", with opacity), tile by tile; a tile is mixed again only when one of its layers' tiles or settings changed, and its index returned. One opaque layer's tiles are used as they are.
- **Brushes and strokes** ([decision 0179](../../docs/decisions/0179-the-round-brush.md)): `Brush` (kind, radius, hardness, flow, spacing, colour, through; `falloff`, `step`, `clamped`); `PaintSurface` (a shape's triangles with UVs, slots and normals, a grid to find those near a point, `hit` along a ray); `stroke`: `View` (the camera), `DepthMap` (the shape seen from the camera, for hidden texels), `StrokeCanvas` (a layer image during a stroke, largest coverage per texel), `paint_dab` (a dab painted onto the nearby triangles in texture space, texels near their edges included), `dabs_along`. `StrokeRecord` and `DabRecord` keep strokes with their layer.
- **Layers** ([decision 0181](../../docs/decisions/0181-paint-layers.md)): `Content::Fill` and `Content::Group` beside paint; `Layer::clipped` and `expanded`; `blend` (`Blend`: eight modes by the W3C formulas, `over`); `stack` (`LayerPath`; `TextureSet::layer`, `layer_mut`, `listed`, `top_paint_layer`, `insert_layer`, `add_paint_layer`, `remove_layer`, `duplicate_layer`, `move_layer`, `group_layer`, `ungroup`, `merge_down`); `composite` mixes groups, fills and clipping, and `flatten` mixes layers into one image.
- **Painting flat** (`flat`, [decision 0180](../../docs/decisions/0180-painting-in-the-uv-editor.md)): `paint_flat_dab` (a dab as a circle on the texture, painting only texels inside the given slots' UV pieces and just outside their edges), `triangle_at` and `slot_at` (the piece under a place on the texture). `DabRecord::flat` and `flat_uv` keep a flat dab as a place on the texture.
- Tests in each module: sizes, copy-on-write, partial edge tiles, smooth stretching, mixing order and opacity, hidden layers, only changed tiles mixed again.
