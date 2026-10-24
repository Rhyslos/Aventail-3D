# Struts

> Turn every edge of a shape into a bar of its own, with a joint at each point, live: a cube becomes a box frame, a sphere a cage.

Frames, cages, grilles, scaffolding, lattice towers, a crate's metal edging, a birdcage, a geodesic dome, a fence of crossed bars: all are bars along the edges of a simple shape. Modelling every bar by hand is slow and hard to change. The **Struts** [modifier](modifiers.md) makes them for you from a shape's [edges](concepts.md#mesh-basics): model the layout as an ordinary shape (a cube, a plane cut into a grid, a sphere), add Struts, and each edge becomes a closed bar, a **strut**, with a small block, a **joint**, round each point where the bars meet. Keep editing the shape and the frame follows. When you're happy, **Bake into shape** (**Ctrl + E**) makes it real polygons.

**Not the same as the Wireframe look.** The viewport's [Wireframe](viewport-display.md) look only *draws* edges as thin lines on screen, to see through a shape; nothing changes and nothing exports. Struts makes real polygons with a thickness, which show in the game. That's why it has its own name.

## When to use it

- **Frames and cages**: a box frame, a cage, shelving uprights, a window grille, a crate's edging. Start from a [cube](primitives.md) or a [plane](primitives.md) cut into a grid.
- **Lattice and space-frame structures**: towers, cranes, scaffolding, a dome made of struts (from a [sphere](primitives.md) or a hexasphere).
- **A panel in a frame**: switch **Keep the faces** on to keep the shape's own polygons as well, with the frame round them.

If you only need a thickness for a *surface* (a plate, a cup), use [Shell](shell.md) instead. For a single bar along a path, a [curve](curves.md) swept with a profile gives you more control.

## How to use it

1. Select one or more shapes (in the Model mode).
2. Click **Struts** on the **modifiers palette** (the column on the right of the viewport; a box made of bars), or choose **Mesh → Modifiers → Struts** (under *Generate*), or press **Space**, type *struts* and press **Enter**.
3. Every edge becomes a square bar **2 cm** thick, with a joint at each point. In the [Attributes panel](attributes.md), under **Modifiers → Struts**, drag **Thickness** and the bars follow as you drag; change **Sides** for round bars.
4. [Export](export-and-import.md) as usual, or **Bake into shape** (**Ctrl + E**) to make it permanent.

The shape you modelled keeps being the shape you edit: in the Points, Edges and Polygons modes you edit the plain shape, and the struts follow. Every edge gets a strut, so plan the edges: a [loop cut](loop-cut.md) adds a row of bars, [Dissolve](clean-up.md) takes one away.

## Settings

| Setting | What it does |
|---|---|
| **Thickness** | How wide each strut is, in the project's [units](concepts.md#objects-and-scenes). 2 cm as standard. For game props, think real-world: a thin wire 2–5 mm, a bike frame 3 cm, a scaffold pole about 5 cm, a steel beam 10–30 cm. The joints are a little wider than the struts, to cover their ends. 0 leaves the shape as it is. |
| **Sides** | How many sides each strut has round it, 3 to 24. **4** (standard) is a square bar; 3 a triangle; 6 a hexagon; 8 or more look round. Each joint has as many sides round it, so square bars meet in little cubes and round ones in little balls. More sides cost more polygons: keep it low for games. |
| **Even thickness** | On (standard), the thickness is measured across the struts' flat sides, so every strut is exactly that wide whatever its sides (a square bar 2 cm thick is 2 cm across). Off, it's measured across their corners, so the strut fits inside a round rod that thick (a square one is then about 1.4 cm across its flats): handy when the struts must fit a round hole. |
| **Keep the faces** | Off (standard), only the struts and joints are left. On, the shape's own polygons stay as well, with the struts round their edges: a panel in a frame, a window with its glass. The kept polygons keep their UVs. |
| **Show in viewport** | Off, the viewport skips the struts while exports and Bake into shape still use them. |

### How the struts sit

- Each strut runs from point to point along its edge, with a flat side facing the way the edge's first polygon faces. So a square frame lies flat on a flat shape, and on a box the bars are square with its sides.
- Each **joint** is a closed little block round its point (a cube for square bars, a faceted ball for round ones), just big enough to hide the ends of the struts meeting there.
- Every strut and joint is a closed piece of its own; they overlap where they meet rather than being joined into one surface. That renders, bakes and exports cleanly, and nothing can fold however the edges meet. (To join them into one surface, [Remesh](remesh.md) after the Struts, at the cost of more, even polygons.)

### Hard edges, UVs and materials

- With **6 sides or fewer**, the struts' corners and the joints' edges are marked [hard](hard-and-soft-edges.md), so a square bar shades as four flat sides. With more sides they're smooth, so a round bar looks round. The ends of every strut are hard where they meet its sides.
- Every strut and every joint gets [UVs](concepts.md#uvs-and-texturing) of its own: a strut's sides unrolled flat into a strip (its length up, round it across) with its two ends hung off the strip; a joint unrolled in rows round it. All are at the same texel density, packed into the texture square without overlapping, with [seams](seams.md) round each piece. A texture painted in Substance Painter or a tiling metal texture fits straight away.
- With **Keep the faces** on, the struts' UVs go in the square **above** the texture square, so the shape's own UVs stay as they were. To paint everything in one square, **Bake into shape** and then [Pack](packing.md) the UVs.
- A strut takes the material of the first polygon along its edge, a joint of the first polygon round its point.

## Shortcuts

| Action | Shortcut |
|---|---|
| Add Struts | No key: the modifiers palette, **Mesh → Modifiers → Struts**, or **Space** and *struts* |
| Make it permanent | **Ctrl + E** (Bake into shape) |

## Tips and common problems

- **Problem:** there are diagonal bars across the squares. **Why:** the shape is made of triangles, and every edge gets a strut. **Fix:** add [Quadify](quadify.md) before the Struts (a new Quadify goes before it on its own), or take those edges away with Dissolve.
- **Problem:** two struts sit on top of each other along a seam. **Why:** the shape is split there, so the edge is there twice. **Fix:** add [Weld by distance](weld-by-distance.md) before the Struts (a new one goes before it on its own).
- **Problem:** "the result would have … polygons, and the most is …". **Why:** every edge and point becomes a piece with several polygons. **Fix:** fewer **Sides**, or a simpler shape.
- **Problem:** the frame is too heavy for a game. **Why:** each square strut is 6 polygons and each joint 6 more; round ones many more. **Fix:** keep **Sides** at 3 or 4, and use fewer edges. For far-away props, a texture with holes (alpha) on a plain shape is lighter still.
- **Problem:** I wanted to *see* the edges, not make bars. **Why:** that's the viewport's Wireframe look. **Fix:** remove the Struts and use the **Wireframe** look (the button in the bottom-left corner of the viewport, or the **Display** menu) ([Viewport display](viewport-display.md)).
- **Tip:** modifiers are worked out in order. A new Struts goes **before** any [Live bevel](live-bevel.md), [Subdivide](subdivide.md) or [Weighted normals](weighted-normals.md) already in the list: a Live bevel after it rounds the bars' corners, a Subdivide makes square bars soft and rounded. Change the order with **Up** and **Down**.
- **Tip:** a [mirrored](symmetry.md) half gets struts whole, since the mirror comes before every modifier: model half of a cage, and the struts run across the middle with no doubled bar.
- **Tip:** Struts after an [Array](array.md) frames every copy: one cube arrayed into a row becomes a run of scaffolding.

## Related

- [Modifiers](modifiers.md): how modifiers are added, ordered and baked.
- [Shell](shell.md): a thickness for a surface instead of its edges.
- [Viewport display](viewport-display.md): the Wireframe look, which only draws edges.
- [Quadify](quadify.md) and [Weld by distance](weld-by-distance.md): cleaning the edges before.
- [Seams](seams.md), [Packing](packing.md) and [Hard and soft edges](hard-and-soft-edges.md): what the struts get.
- [Concepts](concepts.md): [struts](concepts.md#objects-and-scenes), [edge](concepts.md#mesh-basics), [UVs](concepts.md#uvs-and-texturing).
