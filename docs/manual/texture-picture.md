# Texture picture and UV layout export

> Lay a picture of your own on the model by its UVs (and behind the UVs in the UV Editor), see it as a game shows it from far away, and export the UV layout as a picture to paint over in another program.

## When to use it

- **You have a texture already** (one painted elsewhere, a photo, a tiling material) and want to see how it sits on your UVs before exporting. The [checker](checker-and-stretch.md) tells you whether the UVs are even; a real picture tells you whether the bricks run the right way, whether a logo lands where you want it and whether a seam cuts through something important.
- **You want to paint the texture in another program** (Krita, GIMP, Photoshop). Export the UV layout: a picture the size of your texture with every UV edge drawn on it. Put it on a layer over your painting, and you can see where each part of the model lies.
- **You want to check details from far away.** A game doesn't draw a texture at full size when the object is far off: it uses a smaller copy, a [mip level](concepts.md#uvs-and-texturing). Thin lines and pieces packed close together can blur into each other there. The **Mip level** shows what the player would see.

To paint the texture here in the app instead, see [Painting textures](painting-textures.md).

## How to use it

1. Select the objects and open the [UV Editor](uv-editor.md).
2. Choose **UV → Texture picture → Choose picture…** and pick an image (PNG, JPEG and the other kinds the app can read). The viewport switches to the **Texture picture** surface: the picture lies on the model by its UVs, and the UV Editor shows it behind the UVs.
3. Move UVs in the UV Editor: the picture on the model follows as you go.
4. To see it from far away, drag **UV → Texture picture → Mip level** up. Each step halves the picture's sharpness. Drag back to 0 to see it at full size.
5. To export the layout, select the objects and choose **UV → Texture picture → Export UV layout as PNG…** (a picture to put on a layer in a paint program) or **…as SVG…** (lines, for programs that edit drawings). Choose where to save it.
6. To go back to your colours, choose **Display → Surface → Colours**, or **Clear picture** in the same menu.

## Settings

| Setting | What it does |
|---|---|
| **Choose picture…** | The picture shown. It's made square (stretched as a game stretches it over the texture square) and kept at most 2048 pixels across. Kept until you close the app; it isn't saved in the project, because it's a check, not part of the model. |
| **Clear picture** | Stops showing it. The Texture picture surface then shows the numbered checker. |
| **Mip level** | 0 (standard) is the picture itself; 1 is half as sharp, 2 a quarter, up to 6. Remembered. In the UV Editor the big pixels show as blocks. |
| **Export UV layout as PNG…** | White lines on a see-through background, at the project's texture size (UV Editor → **Texture** menu), so it lies exactly over a texture of that size. |
| **Export UV layout as SVG…** | The same lines as an SVG drawing, which stays sharp at any size. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Everything here | UV menu → **Texture picture** |

No keys; ask if you'd like one.

## Tips and common problems

- **Tip:** choose a picture with writing on it to see whether any piece is mirrored: the writing reads backwards there.
- **Tip:** at **Mip level** 3 or 4, look at the gaps between pieces in the UV Editor. Where two pieces' colours run together, the texture can bleed across in a game: give the packing more room (raise **Padding** in the UV settings and pack again).
- **Problem:** the picture looks squashed on the model. **Why:** it isn't square, and a texture is always stretched over the whole square. **Fix:** make the picture square in your paint program first.
- **Problem:** the exported layout is empty. **Why:** nothing with UVs was selected. **Fix:** select the objects first.
- **Problem:** the picture is gone after reopening the app. **Why:** it's kept only while the app is open. **Fix:** choose it again.

## Related

- [Checker and Stretch views](checker-and-stretch.md)
- [The UV Editor](uv-editor.md)
- [Texture size and texel density](texture-and-density.md)
- [Concepts: mip level, UV map](concepts.md#uvs-and-texturing)
