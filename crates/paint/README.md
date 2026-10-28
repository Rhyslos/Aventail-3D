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
- Tests in each module: sizes, copy-on-write, partial edge tiles, smooth stretching, mixing order and opacity, hidden layers, only changed tiles mixed again.
