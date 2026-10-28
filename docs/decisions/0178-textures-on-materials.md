# 0178. Textures on materials

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's first milestone ([0177](0177-texture-painter-plan.md)): somewhere for painting to live, saved with the project and undone like everything else, shown on the model and behind the UVs, before any brush exists.

## Options

1. **A texture set per object.** Simple, but materials shared across objects (one crate kit, one texture) would each paint separately, unlike the export and the engine.
2. **A texture set per material, by name** (chosen). Export already treats materials of the same name as one material across objects, each becoming a texture set in Substance Painter and an engine; painting follows the same rule.
3. **One texture array for the whole project.** Fast to draw, but every set would need the same size.

## Decision

- A new core crate, `paint` (layer 1, no files, GPU or UI): `Image` (square, 64-pixel tiles shared by `Arc`, a tile nobody painted left out), `Layer` (name, shown, opacity, blend, its image), `TextureSet` (size, layers bottom first), `PaintStore` (sets by material name) and `Composite` (the layers mixed, tile by tile, keeping which tiles changed).
- The scene holds the store; `Command::SetTextureSet { material, set }` replaces or removes one set as one undo step (`Edit::SetTextureSet`). Undo memory counts only tiles one side has and the other doesn't.
- Project format 64 saves it: each set's layers and their painted tiles as byte blocks, checked on opening (tile in range and once, size, blend, opacity, names).
- Images in: **Attributes → Materials → Image…** imports a PNG, JPEG, TGA or BMP (`fileio::image_file`, the `image` crate, MIT or Apache-2.0, limited to 16384 pixels a side) as a paint layer on top, stretched smoothly to the set's size, named after the file. A set is made at the project's texture size.
- Seeing it: **Display → Surface → Painted**. The viewport draws each material slot's part of a shape with its own texture (triangles grouped by slot; `SurfaceDraw::Painted`), the paint over the material's colour; a slot without a set shows its colour. Only changed tiles are uploaded, with their smaller copies for far away. The UV Editor shows the texture of the first selected shape's first polygon's material behind the UVs, undimmed.

## Consequences

- Renaming a material moves it away from its texture set (the set stays under the old name); a later milestone can carry sets along with renames.
- Painting tools (milestone 2 on) change a set through the same command, so undo, saving and backups need no more work.
- The standard material is called "Default" in English; its texture set is found by that name.
