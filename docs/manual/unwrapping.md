# Unwrap, Relax and Straighten

> Lay the shape's pieces flat along their seams, live as you place seams, and even them out or straighten them afterwards.

![A sphere after Suggest seams: one seam from bottom to top at the back, and the UV Editor showing the whole ball unwrapped in one piece, its poles at the top and bottom middle.](../screenshots/2026-10-16-unwrap.png)

## When to use it

**Unwrapping** is flattening the surface along its [seams](seams.md), like unfolding a cardboard box, so a texture can be painted on it. A curved surface can't lie flat perfectly, so something has to give: the shapes of its parts, or their sizes. The standard way (**Keep shapes**, what most UV tools do) keeps every little triangle's shape, so a checker texture's squares stay square, at the cost of some change in size across very curved pieces. **UV → Unwrap method** offers three more ways (see [Settings](#settings)).

- **Unwrap** to lay a piece out fresh, or after moving points stretched it. (Polygons that extrudes, bevels and other edits make get fitting UVs by themselves: see [UVs while you keep modelling](uvs-and-modelling.md).)
- You often don't need to: **marking seams unwraps the pieces around them at once** (the switch **UV → Unwrapping → Unwrap as seams change**, on as standard).
- **Relax** evens out UVs you've pulled about by hand, or a piece that stretches.
- **Straighten** lays a piece made of quads out as a perfectly straight grid: the cleanest result for strips and panels in hard-surface work.
- **Unfold across only** and **Unfold up only** unwrap, but move the UVs one way only: every point keeps its height (or its place across). Use them to even out a strip along its length without changing its height.
- To hold some UVs still while the rest is laid out around them, [pin](pins.md) them first.

## How to use it

1. Select an object. To work on part of it, pick the UV mode or press **1**, **2** or **3** and select the pieces (or polygons, edges or points in them). With nothing selected inside it, the whole object is used.
2. Press **E** with the mouse over the UV Editor (or in the UV mode) and choose **Unwrap**, **Relax** or **Straighten**. They're in the **UV** menu too, with **Unfold across only** and **Unfold up only**.
3. Each is one undo step (**Ctrl + Z**), saved with the project and recorded on the object's [edit list](edit-list.md).

What **Unwrap** does with each piece holding the selection:
- lays it flat along its seams, the way **UV → Unwrapping → Method** says. A piece that can't lie flat in one go (a tube, or a closed ball with no seam) is first cut open in a hidden place (where **UV → Seam suggestions → Hide cuts** says), and the cut is marked as a seam;
- a piece of **square-cornered quads** (a cylinder's side, a bevel's strip, a flat panel) comes out as a **straight grid**, when **Straighten strips when unwrapping** is on;
- turns it so its sides line up with the square, keeping the way it lay before (a quarter turn at a time);
- sizes it to match the rest of the object, so every piece keeps its **real size** against the others (the same [texel density](texture-and-density.md));
- puts it where it was. Pieces may overlap; [Pack](packing.md) arranges them.

A piece with [pinned UVs](pins.md) is different: with two or more pins it's laid out around them and keeps its place, size and turn; with one, it's laid out as above and moved so the pin is back. Pinned pieces aren't laid out as straight grids.

**Unfold across only** (or **up only**) unwraps the piece as above, fits the result over where the piece was (moved, turned and sized to lie on it as closely as it can), then gives every point back its old height (or its old place across). A piece with no UVs yet is simply unwrapped.

**Relax** moves only the corners of the selected polygons: each side is pulled towards its real length, so bunched or stretched UVs even out. The other corners, and pinned ones, stay where they are, so you can relax one area of a piece. **UV → Relaxing → Strength** sets how far it goes.

**Nothing is left to chance.** Every choice these tools make is a setting (below), and each edit keeps the settings it was made with: the same shape and settings always give exactly the same UVs, and changing a setting later doesn't change edits you've already made. See [Settings of the automatic UV tools](uv-settings.md) for all of them in one place.

**Straighten** works on pieces made only of quads; others are left as they were (the status bar says so).

## Settings

In **UV → Unwrapping** (and **UV → Relaxing**):

| Setting | What it does |
|---|---|
| **Method** | How Unwrap (and unwrapping as seams change) lays pieces flat. **Keep shapes** (standard): every small part keeps its angles, so a texture's squares stay square; sizes may drift across a strongly curved piece. The fastest. **Keep shapes, smoother**: the same, but the angles are first evened out over the whole piece, so curved pieces (a head, a dome) drift less in size. A little slower. **Keep sizes**: every part keeps its real size, so the texture's pixels are the same size all over the piece, at the cost of a little bending of the squares. Best for game textures with fine detail. About six times slower than Keep shapes. **Automatic**: each piece is laid flat all three ways and keeps the one that lies most evenly; the same piece always gets the same choice. The slowest. |
| **Unwrap as seams change** | On (standard): marking or clearing seams unwraps the pieces on either side at once, in the same undo step. Off: only the seams change until you choose Unwrap. |
| **Straighten strips when unwrapping** | On (standard): pieces of square-cornered quads unwrap as straight grids. Off: they're unwrapped like any other piece (a little curved, if the shape is). |
| **Strip corners within** | How far (degrees) a quad's corners may be from square for its piece to count as a strip: 25° standard, up to 60°. Raise it for strips round tight curves or leaning panels; lower it to straighten only truly square ones. |
| **Relaxing → Strength** | How many steps of evening out Relax takes: 60 standard, 1 to 1,000. A few smooth a little; hundreds even a badly bunched piece right out. |

All are remembered between sessions. Where closed pieces are cut open is in **UV → Seam suggestions → Hide cuts** ([Seams](seams.md#settings)).

## Shortcuts

| Action | Shortcut |
|---|---|
| Unwrap, Relax, Straighten | **E** over the UV Editor (or in the UV mode), or the UV menu |
| Unfold across only, Unfold up only | the **UV** menu |
| Their settings | **UV → Unwrapping**, **UV → Relaxing** |
| Select pieces | UV mode, then click (or **1**, **2**, **3** for parts) |
| Undo | **Ctrl + Z** |

No keys of their own; ask if you'd like some.

## Tips and common problems

- **Problem:** after unwrapping, pieces overlap. **Why:** each piece is put back where it was, and a fresh piece can be bigger. **Fix:** [Pack](packing.md) them, or move them apart with **W** in the UV Editor.
- **Problem:** a ball or rounded piece looks bigger in the middle than at the edges. **Why:** a curved surface can't lie flat without some change in size; Keep shapes keeps the shapes and lets the size vary. **Fix:** choose **UV → Unwrapping → Keep sizes** (or Keep shapes, smoother) and Unwrap again, or add a seam or two to split it (Suggest seams helps).
- **Problem:** with Keep sizes, the checker's squares lean a little. **Why:** keeping every part's size means bending some angles. **Fix:** that's the trade; choose Keep shapes if square squares matter more, or Automatic to let each piece decide.
- **Problem:** a strip came out curved. **Why:** some of its corners aren't square (more than **Strip corners within**, 25° standard, off), or it has a triangle, so it isn't treated as a strip. **Fix:** raise **UV → Unwrapping → Strip corners within**, choose Straighten, or fix the polygons.
- **Problem:** Unwrap marked a seam I didn't place. **Why:** the piece couldn't lie flat in one go (a tube, a closed shape), so it was cut open where it shows least. **Fix:** Ctrl + Z and place the seam where you want it, or keep it.
- **Problem:** unwrapping a big piece takes a moment. **Why:** a piece of tens of thousands of polygons takes seconds to solve. **Fix:** add seams to split it into smaller pieces.
- **Tip:** for a hard-surface prop, a good start is: **UV → Suggest seams** (which unwraps as it goes), then fix seams you don't like; each fix unwraps the pieces around it.

## Related

- [Pinning UVs](pins.md): hold some UVs still while the rest is laid out
- [Settings of the automatic UV tools](uv-settings.md)
- [Seams](seams.md)
- [Trim sheets](trim-sheets.md): lay strips along a shared texture's rows instead
- [UVs while you keep modelling](uvs-and-modelling.md)
- [Texture size and texel density](texture-and-density.md)
- [Checker and Stretch views](checker-and-stretch.md): see where to Relax or add seams
- [The UV Editor and the UV mode](uv-editor.md)
- [Editing UVs by hand](editing-uvs.md)
- [Concepts: unwrapping, pins, stretching, texel density](concepts.md#uvs-and-texturing)
