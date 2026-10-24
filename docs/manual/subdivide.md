# Subdivide

> Smooth a shape by subdividing it: every polygon is split into smaller ones and the corners are rounded off, level by level, while creased edges stay sharp.

**Subdivide** is a [modifier](modifiers.md) for **subdivision smoothing** (often called *subdivision surfaces* or *subD*, and in some programs *TurboSmooth* or *Subdivision Surface*). You model a simple, low-polygon **cage**, and the modifier shows the smooth, rounded shape that cage stands for. A box becomes a pebble; a rough cage of a mug becomes a smooth mug. Because the cage is what you edit, the shape stays light to work with, and changing it is quick.

Each **level** splits every polygon into four (an n-gon of five corners into five) and moves every point towards the average of its neighbours, the method called **Catmull-Clark**, which most 3D programs use. Each level has four times as many polygons, and is smoother.

> **Not to be confused with Smoothing.** An object's **Smoothing** setting in the Attributes panel ([hard and soft edges](hard-and-soft-edges.md)) only changes how light falls on the surface: the shape stays exactly as it is. **Subdivide** changes the shape itself, adding polygons and rounding it off.

## When to use it

- To make smooth, rounded shapes (characters' props, bottles, handles, cushions, organic shapes) from a cage that is easy to edit.
- For a **high-poly** version of a game asset, to [bake](export-and-import.md) normal maps from in Substance Painter, while the low-poly version is the cage itself.
- For **mid-poly** game models: one level, with creased or hard edges where the shape should stay crisp.

It's not for flat, hard-surface shapes that should keep sharp corners everywhere: there, a [Bevel](bevel.md) on the edges gives a crisp shape with a few polygons. Subdivide and creases work best where most of the shape is meant to be round.

## How to use it

1. Select one or more shapes.
2. Click **Subdivide** on the **modifiers palette** (the dashed square with a rounded shape inside, on the right of the viewport), or choose **Mesh → Modifiers → Subdivide**, or press **Space**, type *subdivide*, and press **Enter**.
3. The shape is shown smoothed, one level in the viewport. In the [Attributes panel](attributes.md), under **Modifiers → Subdivide**, change **Levels** and **Levels for export**.
4. Keep modelling the cage. In the Points, Edges and Polygons modes you edit the cage's own points, drawn over the smoothed result, and the smoothing follows every change.
5. To keep an edge sharp, **crease** it (below). When you're done, **Bake into shape** (**Ctrl + E**) makes the smoothed shape a plain mesh, at its export levels.

### Keeping edges sharp: creases

A **[crease](concepts.md#mesh-basics)** tells Subdivide how sharply to keep an edge, from **0** (smoothed like any other) to **1** (kept fully sharp at every level). A crease in between keeps the edge sharp for part of the levels and then smooths it, so it comes out as a tighter curve than its neighbours: **0.5** keeps it sharp for half the levels.

1. Press **2** for the **Edges** mode and select the edges.
2. Do one of these:
   - **Crease edges**: press **E** and choose **Creases**, then **Crease edges** (in the tool menu it sits just before Hard and Soft edges, in the place Flip normals has in the Polygons mode); or **Mesh → Shading → Crease edges**. The edges are fully creased (1).
   - The **Crease** field: in the Attributes panel, under **Selected edges**, drag or type a crease from 0 to 1. One drag is one undo step.
   - **Uncrease**, from the same places, takes the creases away.
3. Creased edges are drawn **magenta** in the Points, Edges and Polygons modes, so you can see where they are.

Creases are recorded [edits](edit-list.md), like hard edges and seams: they're undone with **Ctrl + Z**, made on both sides with [symmetry](symmetry.md) on, and follow the edges through later edits. They do nothing without a Subdivide modifier, and they don't change the shading.

### Hard edges stay sharp

With **Hard edges stay sharp** on (as standard), edges marked **hard** count as fully creased, so you can mark the crisp edges once for both the shading and the smoothing. This only applies on objects whose **Smoothing** is set to their **marked edges** (untick **By angle** on its Smoothing row in the Attributes panel, or **Mesh → Shading → Smooth by angle**). On an object smoothed **by angle**, the standard for new shapes, every corner sharper than the angle looks hard, so counting those would stop a low-poly cage from rounding at all: there, use creases instead.

Marking an edge hard on an object smoothed by angle switches it to its marked edges (keeping its look), so from then on its hard edges stay sharp too.

## Settings

| Setting | What it does |
|---|---|
| **Levels** | How many times the viewport splits and smooths the shape. **1** as standard. Each level has four times the polygons: a 500-polygon cage is 2 000 at level 1, 8 000 at 2, 32 000 at 3. **0** shows the cage as it is. Up to 5. |
| **Levels for export** | How many times exports and **Bake into shape** split and smooth it. **2** as standard: one more than the viewport, smoother where it counts, while the viewport stays quick. For a game's mid-poly model, 1 is often enough; for a high-poly to bake from, 2 or 3. |
| **Hard edges stay sharp** | On (standard): edges marked hard count as fully creased, on objects shaded by their marked edges (see above). Off: only creases keep edges sharp. |
| **Show in viewport** | Off: the viewport shows the cage without smoothing; exports and Bake into shape still smooth it. |
| **Crease** (Edges mode, *Selected edges*) | The selected edges' crease, 0 to 1. Shows the first edge's when they differ; a new value gives them all the same. |

A result over 4 million polygons is refused, with a note asking for fewer levels.

### What the result keeps

- **Open borders** (the rim of a cup, a plane's edges) stay where they are and are smoothed along their length; the **corners** of an open sheet (a point with only one polygon) stay put.
- **Edges shared by three or more polygons** stay sharp.
- **Materials:** each new polygon keeps its old polygon's material.
- **UVs** are smoothed too, so the texture keeps fitting the smoother shape. Along [seams](seams.md) each side keeps to its own island, smoothed along the seam, and a UV corner with only one polygon stays put (what Blender calls *keep corners*). Seams, hard edges and full creases carry over to the new edges; partial creases are used up by the levels.
- **Shading:** the result is shaded by the object's own [Smoothing](hard-and-soft-edges.md); hard edges stay hard.

## Shortcuts

| Action | Shortcut |
|---|---|
| Find Subdivide, Crease edges or Uncrease | `Space`, then type the name |
| The creases in the tool menu (Edges mode) | `E`, then **Creases** |
| Make it permanent (Bake into shape) | `Ctrl + E` |

Subdivide and the crease commands have no keys of their own.

## Tips and common problems

- **Problem:** the shape shrinks and looks like a blob. **Why:** subdivision pulls every corner inwards; a cage with few polygons rounds off a lot. **Fix:** add edge loops near edges that should stay tighter ([Loop cut](loop-cut.md), F), or crease them.
- **Problem:** a box stays a box. **Why:** its edges are creased, or marked hard on an object shaded by its marked edges, with **Hard edges stay sharp** on. **Fix:** uncrease them, or untick **Hard edges stay sharp**.
- **Problem:** a crease of 0.5 looks fully sharp in the viewport but softer in the export. **Why:** a partial crease keeps the edge sharp for part of the levels, so it shows best with more levels. **Fix:** compare at the export's levels: set **Levels** the same as **Levels for export** for a moment.
- **Problem:** pinches or bumps where many edges meet, or around triangles and n-gons. **Why:** subdivision is smoothest on all-quad cages with four edges at most points. **Fix:** rework the cage into quads where you can ([Connect](connect-bridge-weld.md), [Dissolve](clean-up.md)).
- **Problem:** the smoothed shape lags a moment behind while dragging points. **Why:** a big cage at level 2 or more is worked out on another thread so the window stays smooth. **Fix:** a lower **Levels** while modelling; keep **Levels for export** high.
- **Tip:** with [symmetry](symmetry.md), the mirror comes before the modifiers, so the two halves are smoothed as one, with no crease down the middle.
- **Tip:** Subdivide after [Remesh](remesh.md) (put it below in the list) smooths a remeshed shape further.

## Related

- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Hard and soft edges](hard-and-soft-edges.md): the per-object Smoothing setting, which shades but doesn't change the shape
- [Bevel](bevel.md), for crisp hard-surface edges
- [Concepts: subdivision, creases](concepts.md#mesh-basics)
- Why it works this way: [decision 0143](../decisions/0143-subdivide-and-creases.md)
