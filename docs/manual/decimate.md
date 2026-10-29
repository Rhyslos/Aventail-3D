# Decimate

> Lower a shape's polygon count while keeping its shape: flat areas lose the most, curves, corners and the outline the least.

**Decimate** is a [modifier](modifiers.md): it works out a lighter version of your shape live, so you can keep modelling the detailed one underneath and try different amounts any time. A game has a budget of [triangles](concepts.md#mesh-basics) for each model, and a shape that came out of [Subdivide](subdivide.md), [Remesh](remesh.md), a sculpt or a scan usually has far more than it needs: thousands of tiny polygons on flat walls that a few big ones would show just as well. Decimate takes those away first.

It works like this: the shape is cut into triangles, and then, again and again, the edge whose removal would change the shape *least* is collapsed: one of its end points moves onto the other, and the two triangles on the edge disappear. How much an edge would change the shape is measured against the planes of all the triangles its point once touched, so on a flat wall the cost is nothing, and on a sharp curve it is high. It stops when the triangles left reach your target. (Other programs call this *polygon reduction*, *ProOptimizer* or *quadric decimation*.)

The points that are left are some of your shape's own points, never moved: the result is exactly on your surface.

## When to use it

- To bring a detailed shape down to a game's budget: a smoothed, remeshed or scanned prop to a few thousand triangles.
- To make lighter versions of a model for far away ([levels of detail, LODs](concepts.md#the-game-asset-pipeline)): Decimate at 50%, 25% and 12%.
- To lighten a heavy shape you only need as a background piece or a blockout.

It's not the right tool for a model you'll keep editing by hand: the result is triangles laid out by cost, not tidy edge loops. Model low-poly shapes you'll keep working on by hand, and decimate what's too heavy to model by hand. For shapes that must stay exactly as built (a modular wall that tiles), keep **Keep open borders** on and check the result.

## How to use it

1. Select one or more shapes.
2. Click **Decimate** on the **modifiers palette** (the square of small triangles becoming big ones, on the right of the viewport), or choose **Mesh → Modifiers → Decimate** (under *Clean up*), or press **Space**, type *decimate*, and press **Enter**.
3. The shape is shown with half its triangles. In the [Attributes panel](attributes.md), under **Modifiers → Decimate**, change the **Ratio** (or switch **Reduce by** to **Target triangles** and type your budget). **Triangles shown** says how many the viewport's shape has now.
4. To see the new polygons, tick **Edges** in the list of the button beside the view dice ([How shapes are drawn](viewport-display.md)). Keep modelling the shape underneath if you like: the decimation follows. When you're happy, **Bake into shape** (**Ctrl + E**) makes it a plain mesh.

On a big shape (from about 5 000 triangles) the result is worked out in the background; the window keeps working meanwhile, showing the last result. 100 000 triangles take about half a second.

## Settings

| Setting | What it does |
|---|---|
| **Reduce by** | How the new count is given: **Ratio** (a share of the triangles the shape has) or **Target triangles** (a number). |
| **Ratio** | The share of triangles to keep: **50%** as standard halves them; 10% keeps a tenth. Down to 0.1%. |
| **Target triangles** | How many triangles to end with, for a fixed budget (1 000 as standard). The arrows step by 100 (Shift: 1 000). |
| **Keep UV seams** | On (standard): [seams](seams.md), and edges where the UVs are split, stay as lines, so the texture stays where it was along them. Off: they can be collapsed across, and the texture may smear there. |
| **Keep hard edges** | On (standard): the edges the shading shows hard stay as lines: those marked hard on an object shaded by its marks, or those sharper than its [Smoothing](hard-and-soft-edges.md) angle (30° as standard) otherwise. A box stays a crisp box. |
| **Keep material borders** | On (standard): where one [material](materials.md) meets another stays exactly where it is. |
| **Keep open borders** | On (standard): the edges of holes and open sheets stay where they are. Off: an open edge can be worn away like any other. |
| **Quads where flat** | Off (standard): the result is all triangles, which is what a game engine draws anyway. On: pairs of triangles lying flat together (within 1.5°) are joined into four-sided polygons again, which is tidier to look at and to keep editing. The count is still in triangles. To join pairs round curves too, add [Quadify](quadify.md) after Decimate instead. |
| **Show in viewport** | Off: the viewport shows the shape without the decimation; exports and Bake into shape still use it. |

**Triangles shown** (not a setting) is how many triangles the viewport's shape has after all its modifiers.

### Why it counts triangles

A game engine draws triangles: a square polygon is two, a five-sided one three. So the Ratio and the target are counted in triangles, the number a game budget means, and Decimate's result is triangles too.

### What "keep" means

A kept line (a seam, a hard edge, a material border, an open border) isn't frozen, but it stays a line: a point on it can only slide along it, onto its neighbour on the line, and only where that keeps the line's course. Where a kept line turns by more than 30° (a square's corner), or where lines meet or end, the point never moves. So a straight seam can lose points along it, but it stays straight and where it was; a hole keeps its corners.

Because corners never move, a target can't always be reached: a cube's 12 triangles are already all corners, and a sphere's UV seams and poles keep some points. The result then stops where it can; **Triangles shown** says where.

### What the result keeps

- **The shape:** every point left is one of the shape's own, never moved. Collapses that would turn a triangle over, make the same triangle twice, or pinch the surface through a point are never made.
- **UVs:** each corner keeps one of its own UVs; collapses that would stretch the texture cost more, so they come later.
- **Materials:** each triangle keeps its polygon's material.
- **Hard edges, seams and creases:** they stay on the edges that are left, and follow a kept line as it shortens.
- **Not kept:** values on points and other corner values; loose points (points with no polygon) are left out.

### Levels of detail

Decimate at a few ratios (say 50%, 25% and 12%) gives the lighter versions a game shows further away. A coarser level made by the app always has only points the finer ones have, so they swap without the outline jumping about. To have the app make the levels for you, beside the object rather than in its place, use [Levels of detail](levels-of-detail.md).

## Shortcuts

| Action | Shortcut |
|---|---|
| Find Decimate | `Space`, then type *decimate* |
| Make it permanent (Bake into shape) | `Ctrl + E` |

Decimate has no key of its own.

## Tips and common problems

- **Problem:** it won't go below a certain count. **Why:** kept lines' corners never move: UV seams with many pieces, hard edges everywhere (a shape smoothed by a small angle), or many materials. **Fix:** switch off what you can do without (for a far-away LOD, **Keep UV seams** often can go), or raise the Smoothing angle so fewer edges count as hard.
- **Problem:** the texture smears in places. **Why:** **Keep UV seams** is off, or the collapse had to cross a stretch of UVs. **Fix:** switch **Keep UV seams** on, or decimate less.
- **Problem:** a modular piece no longer lines up with its neighbour. **Why:** points along its open edges were taken away (with Keep open borders on, the edge keeps its course, but its points along it can go). **Fix:** decimate the pieces together after joining them, or keep their borders' points by giving the border its own material for a moment.
- **Problem:** long thin triangles. **Why:** that's often the cheapest way to keep a curve with few triangles. **Fix:** decimate less, or turn on **Quads where flat** for tidier flat areas. For even polygons, use [Remesh](remesh.md) first.
- **Problem:** nothing changes on a cube. **Why:** all its points are corners where hard edges meet. That's right: it can't be shown with fewer.
- **Tip:** put Decimate after [Subdivide](subdivide.md) to make a smooth, light version: smooth at a high level, then decimate to your budget.

## Related

- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Subdivide](subdivide.md) and [Remesh](remesh.md), which make the heavy shapes Decimate lightens
- [Quadify](quadify.md), to join Decimate's triangles back into quads
- [Hard and soft edges](hard-and-soft-edges.md), [Seams](seams.md) and [Materials](materials.md): what it keeps
- [Concepts: triangle count, LOD, decimation](concepts.md#objects-and-scenes)
- Why it works this way: [decision 0145](../decisions/0145-decimate.md)
