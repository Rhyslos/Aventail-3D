# Painting textures

> Each material gets its own **texture set**: layers of paint laid on the shape through its UVs, shown on the model and in the UV Editor.

This is the start of the app's texture painter (Phase 11). For now you can bring pictures painted elsewhere in as paint layers and see them on the model; brushes for painting on the model come next.

## When to use it

- You want to see a texture on your model while you work, for example a picture of wood planks on a crate, or a hand-painted texture from another program.
- You want your painting kept inside the project file, so it's saved, backed up and undone with everything else.

A texture lands where your [UVs](concepts.md#uvs-and-texturing) put it, so unwrap first (every ready-made shape already has UVs). If the picture looks stretched or in the wrong place, the UVs are what need changing: see [Unwrapping](unwrapping.md) and [Checker and Stretch views](checker-and-stretch.md).

## How texture sets work

A game engine gives each material its own textures. The app does the same: every [material](materials.md) can have a **texture set**, its textures and their layers. It goes by the material's **name**: materials with the same name on different objects are one material in an export, so they share one texture set.

A texture set's **layers** are mixed bottom first, like layers in a paint program: a layer higher up covers the ones under it where it's painted, and lets them show through where it isn't. Where no layer is painted, the material's own colour shows.

A new texture set is made at the project's texture size (UV → Texture, see [Texture size and texel density](texture-and-density.md)), 2048 pixels as standard.

## How to use it

1. Select an object. In the **Attributes** panel, under **Materials**, each material has an **Image…** button.
2. Click **Image…** beside the material you want to texture, and choose a picture: PNG, JPEG, TGA or BMP.
3. The picture becomes a paint layer on that material's texture set, named after the file, stretched to fill the texture square. The viewport switches to **Display → Surface → Painted**, so you see it on the model at once.
4. Open the **UV Edit** layout (top right) to see the same texture behind the UVs in the UV Editor.

Importing again onto the same material adds another layer on top (named *grass 2* if *grass* is taken). Each import is one step to undo (**Ctrl + Z**).

## Settings

| Setting | What it does |
|---|---|
| **Image…** (Attributes → Materials) | Imports a picture as a new paint layer on top of that material's texture set. Pictures of any size are stretched to the texture size. |
| **Display → Surface → Painted** | Shows each material's painted texture on the model, over the material's colour. The UV Editor shows the texture of the first selected shape's material behind the UVs. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Undo an import | `Ctrl + Z` |

## Tips and common problems

- **Problem:** the model doesn't show the picture. **Why:** the viewport shows another surface (Colours, Checker...). **Fix:** **Display → Surface → Painted**.
- **Problem:** the picture is squashed or repeats oddly. **Why:** it was stretched to a square texture, or the UVs don't fill the square the way the picture expects. **Fix:** use a square picture painted for these UVs, or change the UVs.
- **Problem:** two objects show the same texture. **Why:** their materials have the same name, so they share one texture set (as they would in the game). **Fix:** rename one of the materials.
- **Problem:** the picture is blurry. **Why:** it's smaller than the texture size, or the texture size is small for how close you look. **Fix:** use a bigger picture, or raise the texture size.

## Related

- [Materials](materials.md), [Texture size and texel density](texture-and-density.md), [The UV Editor](uv-editor.md)
- [Concepts](concepts.md): [texture set and paint layer](concepts.md#uvs-and-texturing)
