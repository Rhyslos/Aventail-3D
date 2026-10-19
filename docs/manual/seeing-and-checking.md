# Seeing and checking UVs

> The UV Editor warns you about overlapping and mirrored pieces as you work, can write each piece's size, texel density and stretch on it, and measures lines in texture pixels and real centimetres.

![The UV Editor with a cube's six pieces: two overlapping pieces tinted red, one mirrored piece tinted purple, "2 pieces overlapping · 1 piece mirrored" written top right, each piece labelled with its size (604 × 604 px) and density (604 px/m), and a yellow measuring line across two pieces reading "1235 px · 193.3 cm on the surface".](../screenshots/2026-10-17-seeing-and-checking.png)

## When to use it

**Warnings** are always on. Two problems spoil a texture without being obvious while you work:

- **Overlapping pieces** (tinted red): two pieces lying over each other share the same pixels, so whatever you paint on one shows on the other, and a bake writes both into one spot. A polygon folded over inside its own piece is the same problem in miniature, and is tinted red too.
- **Mirrored pieces** (tinted purple): the piece lies flipped in the texture, so text and arrows painted on it read backwards on the model, and normal maps can light it the wrong way.
- **Pieces outside the square** (tinted amber): part or all of the piece lies outside the texture square (0 to 1). A texture only covers the square, so what's outside repeats the texture from the other side, and a bake can't paint it.

Pieces only overlap if they share a texture: pieces with different materials (when **Pack** gives each material its own texture, see [Pack](packing.md)) may lie in the same spot. Pieces stacked exactly on an identical piece on purpose (to share pixels), and pieces on a [trim sheet](trim-sheets.md)'s rows (which run past the square's sides on purpose), aren't warned about. Mirroring a piece on purpose, for a symmetrical model whose halves share pixels, is fine, as long as nothing you paint there has to read the right way round.

**Numbers on pieces** help when you're judging a layout: is this piece as big as it should be, does it get the texture it needs, is anything stretched?

**Measuring** answers "how big is this in the texture, and on the model?": for example, how many pixels a strip is wide, or how long a seam-to-seam distance really is.

## How to use it

**Warnings** need nothing: the red, purple and amber tints, and a line top right saying how many pieces overlap, are mirrored or stick out of the square, show whenever the UV Editor has something to warn about. Fix it with [Pack](packing.md) (moves pieces apart), **UV → Turn and flip** (flips a mirrored piece back), Pack (brings pieces back inside the square), [Unwrap or Relax](unwrapping.md) (unfolds folded polygons), or by moving pieces by hand.

**Numbers on pieces:**

1. In the UV Editor, open **View** and tick **Numbers on pieces**.
2. Each piece big enough on screen shows its size in texture pixels (width × height of its box), its [texel density](texture-and-density.md) in pixels per metre, and, if any polygon in it is stretched, how much at worst ("stretched 1.4×").
3. Zoom in to see the numbers on small pieces. Untick to hide them.

**Measuring:**

1. In the UV Editor, open **View** and choose **Measure**.
2. Drag a line in the editor. It shows its length in texture pixels and how long it is on the surface of the model, in centimetres (counting the object's size in the scene).
3. Drag again to measure something else. Press **Esc** (or choose **View → Measure** again) to go back to selecting.

While measuring, a drag measures instead of selecting or moving UVs; navigating the view still works.

## Settings

| Setting | What it does |
|---|---|
| **View → Numbers on pieces** | Writes size, texel density and worst stretch on each piece. Off when the app starts. |
| **View → Measure** | Turns measuring on: drag to measure. |

The overlap warnings check pieces against others on the same texture, grouped as **UV → Settings → Packing** groups them (a texture per material, a texture per object).

## Shortcuts

| Action | Shortcut |
|---|---|
| Measure | **View → Measure** in the UV Editor, then drag |
| Stop measuring | `Esc` |
| Numbers on pieces | **View → Numbers on pieces** in the UV Editor |

## Tips and common problems

- **Problem:** a piece is red but I can't see anything over it. **Why:** a polygon is folded over inside the piece (the piece crosses itself), or a small piece hides under it. **Fix:** Relax or Unwrap the piece; Pack moves small pieces out.
- **Problem:** two objects' pieces are red, but they have different materials. **Why:** **UV → Settings → Packing → A texture per material** is off, so they're counted as sharing one texture. **Fix:** turn it on if each material gets its own texture.
- **Problem:** the measured length on the surface is shorter than the line looks. **Why:** only the parts of the line over pieces count; the gaps between pieces (and jumps across a seam) aren't surface. **Fix:** measure within one piece for a single length.
- **Problem:** no numbers on some pieces. **Why:** they're too small on screen to fit. **Fix:** zoom in.
- **Tip:** the numbered checker (**Display → Surface → Numbered checker**) shows mirroring on the model itself: labels read backwards.

## Related

- [The UV Editor](uv-editor.md)
- [Pack](packing.md)
- [Texture size and texel density](texture-and-density.md)
- [Checker and stretch views](checker-and-stretch.md)
- [Check for baking](bake-check.md)
- [Concepts: UVs and texturing](concepts.md#uvs-and-texturing)
