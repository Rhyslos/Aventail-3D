# Triangles: Show triangles and Triangulate

> See the triangles a game engine will get from your shapes before you export (**Display → Show triangles**), and fix them in place if you want with the **Triangulate** modifier.

A game engine, and your graphics card, only draw [triangles](concepts.md#mesh-basics). Every quad and [n-gon](concepts.md#mesh-basics) you model is cut into triangles on the way out: when you [export](export-and-import.md) to glTF (always) or to OBJ with **Cut polygons into triangles** ticked. That cut is called [triangulation](concepts.md#the-game-asset-pipeline). Most of the time you never need to think about it. But it matters in three cases:

- **A bent polygon** (a quad whose four corners aren't flat) can fold either way along one of its two diagonals, and the two folds look different. If the game folds it one way and your bake the other, the shading or the normal map goes wrong there.
- **A long thin triangle** (a sliver), from an n-gon cut like a fan, can shade with streaks and wastes the graphics card's time.
- **Baking** (in Substance Painter, or the app's own checks) wants exactly the same triangles as the game, so both read the model the same way.

The app always cuts the same polygon the same way, and the viewport draws your shapes with that very cut, so what you see is what the export gets. **Show triangles** makes the cut visible as faint lines. The **Triangulate** [modifier](modifiers.md) goes one step further and makes the triangles part of the shape, so nothing is left to the export: with **Beauty** it also evens out slivers.

## When to use it

- **Show triangles**: before an export, to check how n-gons and bent quads will be cut; while modelling low-poly props, to spot slivers and bad folds early. It changes nothing.
- **Triangulate** (the modifier): when you want the triangles fixed in the shape itself, so a baker, a game engine and any other program get exactly these, whatever they'd do on their own; or to even out the slivers of big flat n-gons (**Beauty**); or to send triangles to a program that cuts polygons its own way (an OBJ to Substance Painter without **Cut polygons into triangles** ticked).

If your shape is made of triangles and you want quads back, that's the opposite: [Quadify](quadify.md).

## How to use it

### Show triangles

1. In the viewport's **Display** menu, tick **Show triangles** (or press **Space**, type *show triangles* and press **Enter**; it's also in **Edit → Settings… → Viewport → Viewport display**).
2. Every shown shape gets faint lines inside its polygons where they'll be cut: one across each quad, a few across each n-gon. Triangles have nothing to add, so a shape that is all triangles shows only its own edges.
3. Untick it to hide the lines again. The choice is remembered.

The lines show what the viewport draws: with modifiers, the shape they make (for a [Subdivide](subdivide.md) with a higher export level, the viewport's level).

### The Triangulate modifier

1. Select one or more shapes (in the Model mode).
2. Click **Triangulate** on the **modifiers palette** (the column on the right of the viewport; a six-sided polygon cut into triangles, near the end), or choose **Mesh → Modifiers → Triangulate** (under *Game ready*), or press **Space**, type *triangulate* and press **Enter**.
3. Every polygon with four or more corners becomes triangles. In the [Attributes panel](attributes.md), under **Modifiers → Triangulate**, choose the **Method** and whether only quads are cut.
4. [Export](export-and-import.md) as usual, or **Bake into shape** (**Ctrl + E**) to make the triangles permanent.

You keep editing the shape underneath as quads and n-gons in the Points, Edges and Polygons modes: the triangles follow each edit.

## Settings

| Setting | What it does |
|---|---|
| **Method** | **Beauty** (standard): starts from the export's cut, then evens out each flat polygon's triangles, turning the line between two triangles to the other diagonal wherever that makes the thinnest corner wider. Fewer slivers, especially in big flat n-gons. Bent polygons keep the cut along their fold, so the shape doesn't change. **Fixed**: exactly the cut the export makes, the one **Show triangles** draws: exporting gives the same file with or without the modifier. |
| **Quads only** | Off (standard): every polygon with four or more corners is cut. On: only four-sided polygons are cut, and n-gons stay as they are. |
| **Show in viewport** | Off, the viewport skips it while exports and Bake into shape still use it. |

### What stays the same

- **Points never move**, so the shape is the same; each polygon's first triangle keeps the polygon's [name](concepts.md#mesh-basics) and the others get new ones.
- **Shading:** on flat polygons the shading is exactly what it was. [Weighted normals](weighted-normals.md) before the Triangulate reach the export unchanged (Triangulate is the one modifier after it that keeps them). A bent polygon can shade a little differently once it's two triangles, as it will in the game anyway.
- **UVs, materials, hard edges, seams, creases**: every triangle keeps its polygon's UVs at each corner and its material; every edge keeps its marks. The new lines inside are plain edges.

## Shortcuts

| Action | Shortcut |
|---|---|
| Show triangles | No key: **Display → Show triangles**, or **Space** and *show triangles* |
| Add Triangulate | No key: the modifiers palette, **Mesh → Modifiers → Triangulate**, or **Space** and *triangulate* |
| Make it permanent | **Ctrl + E** (Bake into shape) |

## Tips and common problems

- **Problem:** the Triangulate modifier says "Not worked out: every polygon is a triangle already". **Why:** there's nothing to cut: the shape (or a modifier before it, such as [Decimate](decimate.md)) is all triangles. **Fix:** nothing to do; remove the modifier if you like.
- **Problem:** "the shape has no four-sided polygons to cut". **Why:** **Quads only** is on and the shape's polygons are triangles and n-gons. **Fix:** untick **Quads only**.
- **Problem:** a big flat n-gon shows a fan of long thin triangles. **Why:** the export cuts flat polygons without dents from their first corner, quick and correct, but thin. **Fix:** add Triangulate with **Beauty**, or cut the n-gon yourself with [Connect](connect-bridge-weld.md) or a [loop cut](loop-cut.md).
- **Problem:** a quad's line goes the "wrong" way and the shape folds badly. **Why:** the quad is bent; the export cuts quads from corner 1 to 3 (counted from 1) unless that runs outside a dented one. **Fix:** cut it yourself the way you want with [Connect](connect-bridge-weld.md) between the two other corners, or flatten the quad.
- **Tip:** modifiers are worked out in order. Every other modifier you add goes **before** a Triangulate already in the list, so it stays the finishing step; change the order with **Up** and **Down**.
- **Tip:** you don't need the modifier for a glTF export to come out right: the export already uses the cut Show triangles draws. The modifier is for fixing the cut in the shape, for Beauty, or for programs that cut polygons their own way.
- **Tip:** the [triangle count](concepts.md#mesh-basics) in the Attributes panel is the same with or without the modifier: a quad is two triangles either way.

## Related

- [Modifiers](modifiers.md): how modifiers are added, ordered and baked.
- [Quadify](quadify.md): the opposite, triangles joined back into quads.
- [Export and import](export-and-import.md): glTF always triangulates; OBJ when **Cut polygons into triangles** is ticked.
- [How shapes are drawn](viewport-display.md): the Display menu's other switches.
- [Check for baking](bake-check.md): bent n-gons and other things that spoil a bake.
- [Concepts](concepts.md): [triangulation](concepts.md#the-game-asset-pipeline), [triangle count](concepts.md#mesh-basics), [n-gon](concepts.md#mesh-basics).
