# Painting textures

> Each material gets its own **texture set**: layers of paint laid on the shape through its UVs, shown on the model and in the UV Editor.

This is the start of the app's texture painter (Phase 11): you can paint on the model with a round brush in **Paint mode**, and bring in pictures painted elsewhere as paint layers. More brushes, layers you can manage, masks and colour tools follow.

## When to use it

- You want to see a texture on your model while you work, for example a picture of wood planks on a crate, or a hand-painted texture from another program.
- You want your painting kept inside the project file, so it's saved, backed up and undone with everything else.

A texture lands where your [UVs](concepts.md#uvs-and-texturing) put it, so unwrap first (every ready-made shape already has UVs). If the picture looks stretched or in the wrong place, the UVs are what need changing: see [Unwrapping](unwrapping.md) and [Checker and Stretch views](checker-and-stretch.md).

## How texture sets work

A game engine gives each material its own textures. The app does the same: every [material](materials.md) can have a **texture set**, its textures and their layers. It goes by the material's **name**: materials with the same name on different objects are one material in an export, so they share one texture set.

A texture set's **layers** are mixed bottom first, like layers in a paint program: a layer higher up covers the ones under it where it's painted, and lets them show through where it isn't. Where no layer is painted, the material's own colour shows.

A new texture set is made at the project's texture size (UV → Texture, see [Texture size and texel density](texture-and-density.md)), 2048 pixels as standard.

## Painting on the model

1. Click **Paint** on the mode palette (the brush, under UV). The viewport switches to **Display → Surface → Painted**, the modifiers column steps aside, and the **Attributes** panel shows the **Paint brush**'s settings at the top.
2. Move the mouse over a shape: a circle shows the brush's size, with a fainter ring where its full-strength part ends.
3. **Press the left button and drag** to paint. The paint goes onto the texture set of the material under the brush, into its top layer (a layer called *Paint* is made the first time, and the texture set too, at the project's texture size).
4. Let go: the whole stroke is one step to undo (**Ctrl + Z**).
5. Press **Esc** (or click another mode) to leave Paint mode. The viewport goes back to the surface it showed before.

**Why your paint doesn't leak or show seams:** the app paints each spot of the texture where it sits on the shape, seen from the camera. So paint carries across a UV seam onto the matching piece, even when the two pieces lie far apart on the texture, and the join doesn't show. Parts hidden behind others, or facing away, are left alone, so a stroke on the front doesn't come through on the back. (Tick **Paint through** to paint those too.)

### The brush

| Setting | What it does |
|---|---|
| **Size** | The brush's radius, on screen in pixels or, with **On the surface**, in centimetres. Hand-painted textures often start with a big soft brush for colour, then smaller harder ones for detail. |
| **Measured** | **On screen**: the brush stays the same size on screen as you zoom, so zooming in paints finer detail. **On the surface**: the same real size on every object, in centimetres, however far away. A warning shows if the brush is smaller than a texture pixel there, as its detail wouldn't show. |
| **Flow** | How much paint one stroke lays on. Within one stroke paint doesn't pile up where the brush passes twice; stroke again to build it up, as in Photoshop or Krita. |
| **Hardness** | How much of the brush paints at full strength: 100% has a crisp edge, 0% fades from the middle. |
| **Spacing** | The gap between the brush's dabs along a stroke, as a share of its width. Small gaps make smooth lines. |
| **Colour** | The colour it paints. |
| **Paint through** | Paints parts hidden behind others and facing away too. |

Under the settings, the panel says what you're painting on: the material and the layer.

### Brush keys (as in Substance Painter)

| Action | Shortcut |
|---|---|
| Paint | `1`, then left drag |
| Erase (take paint away) | `2`, then left drag |
| Brush smaller / bigger | `[` / `]` |
| Size (sideways) and hardness (up and down) | `Ctrl` + right drag |
| Flow (sideways) | `Ctrl` + left drag |
| Leave Paint mode | `Esc` |

In Paint mode, `1` and `2` pick the brush instead of switching to the Points and Edges modes. The view still turns, pans and zooms as usual (Alt + drag, and the right and middle buttons in your navigation scheme).

## Bringing in a picture

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
| Undo a stroke or an import | `Ctrl + Z` |
| Paint mode's keys | See [Brush keys](#brush-keys-as-in-substance-painter) above |

## Tips and common problems

- **Problem:** nothing happens when I paint. **Why:** the brush is over empty space, or the shape has no UVs. **Fix:** paint on a shape; give it UVs with [Unwrapping](unwrapping.md) (the ready-made shapes have them).
- **Problem:** the paint is blocky. **Why:** the texture is small for how close you look, or the brush is smaller than a texture pixel (Attributes warns). **Fix:** raise the texture size (UV → Texture), or paint with a bigger brush.
- **Problem:** the model doesn't show the picture. **Why:** the viewport shows another surface (Colours, Checker...). **Fix:** **Display → Surface → Painted**.
- **Problem:** the picture is squashed or repeats oddly. **Why:** it was stretched to a square texture, or the UVs don't fill the square the way the picture expects. **Fix:** use a square picture painted for these UVs, or change the UVs.
- **Problem:** two objects show the same texture. **Why:** their materials have the same name, so they share one texture set (as they would in the game). **Fix:** rename one of the materials.
- **Problem:** the picture is blurry. **Why:** it's smaller than the texture size, or the texture size is small for how close you look. **Fix:** use a bigger picture, or raise the texture size.

## Related

- [Materials](materials.md), [Texture size and texel density](texture-and-density.md), [The UV Editor](uv-editor.md)
- [Concepts](concepts.md): [texture set and paint layer](concepts.md#uvs-and-texturing)
