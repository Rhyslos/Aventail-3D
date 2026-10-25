# Weld by distance

> Melt points that sit on top of each other (closer together than a tiny distance) into one, closing the cracks in a shape, live or once.

A shape can look whole and still be in pieces. Two points can sit in the same spot without being joined: then the polygons either side of them don't share an edge, and there is a **crack** in the surface, even if you can't see it. Cracks are common:

- a model [imported](export-and-import.md) from another program or a game is usually split along every [UV seam](seams.md) and hard edge, because game files store it that way;
- pieces placed edge to edge, by snapping or by hand, touch but don't join;
- a scale to zero, a bevel or a weld by hand can leave points almost on one spot.

You notice a crack when an [edge loop](concepts.md#mesh-basics) or a [loop cut](loop-cut.md) stops at it, when [Subdivide](subdivide.md) pulls the surface open there, when the shading shows a hard line, or when the **Mesh** section of the [Attributes panel](attributes.md) says its **Surface** is **Open** where it should be closed. **Weld by distance** finds every pair of points closer together than its **Distance** (a tenth of a millimetre as standard) and melts each pair into one point, so the polygons either side share their edge again.

It is two things, like [Quadify](quadify.md):

- the **Weld by distance modifier**: a [modifier](modifiers.md), worked out live, so the shape underneath stays as it is;
- **Weld by distance now**: a one-off clean-up, recorded as an [edit](edit-list.md), on whole shapes or on the selected points. Its settings stay changeable under **Last edit**.

It isn't the same as **Weld** (**M**, see [Connect, Bridge and Weld](connect-bridge-weld.md)): Weld melts the points *you choose* into one, however far apart; Weld by distance melts only points that are *already on top of each other*, all over the shape at once.

## When to use it

- **An imported model that falls apart** when you loop cut or smooth it: weld it once with **Weld by distance now** in the Model mode.
- **Pieces built separately and placed edge to edge** (a wall made of panels, a frame round a door): select their points and weld them, or join the objects first (**Mesh → Shape and object → Join objects**) and weld the whole shape.
- **Live, under a smoothing modifier**, when the shape underneath is split and you keep changing it. A new Weld by distance modifier goes before a Subdivide already there, so the smoothing sees one closed surface.

## How to use it

### Once: Weld by distance now

1. Select one or more shapes.
2. Either stay in the **Model mode** to weld the whole shapes, or switch to the **Points**, **Edges** or **Polygons mode** (**1**, **2** or **3**) and select what to weld: the points, or the edges or polygons whose points should weld. Only selected points weld, and only with each other.
3. Choose **Mesh → Clean up → Weld by distance now**, or press **Space**, type *weld by distance*, and press **Enter**.
4. The status bar says how many points went. In the [Attributes panel](attributes.md), **Last edit** shows its settings: change the **Distance** and the shape follows. **Ctrl + Z** takes it back.

The settings you choose are remembered for the next Weld by distance now. If nothing is close enough, it's still recorded (and the status bar says so), so you can raise the Distance under Last edit.

With [symmetry](symmetry.md) on, the mirrors of the selected points weld too.

### Live: the Weld by distance modifier

1. Select one or more shapes.
2. Click **Weld by distance** on the **modifiers palette** (the column on the right of the viewport; two blocks side by side with the points between them ringed), or choose **Mesh → Modifiers → Weld by distance** (under *Clean up*), or press **Space**, type *weld by distance*, and press **Enter**.
3. The viewport shows the shape welded. Change the settings in the Attributes panel under **Modifiers → Weld by distance**.
4. [Export](export-and-import.md) as usual, or **Bake into shape** (**Ctrl + E**) to make it permanent.

**Where it goes in the list.** A new Weld by distance is put **before** any [Decimate](decimate.md), [Quadify](quadify.md), [Live bevel](live-bevel.md), [Subdivide](subdivide.md), [Weighted normals](weighted-normals.md) or [Shell](shell.md) the shape already has: each of those reads how polygons join, and a crack spoils them (a Shell would grow a rim along it). Anything else, it's added after. Move it with **Up** and **Down**.

## How it works

The points are taken in order of their numbers. The first one stays where it is, and every point within the Distance of it melts onto it. Then the next point not yet taken does the same, and so on. A point never travels further than the Distance, even along a row of close points, and the same shape and settings always give the same result.

When points melt:

- the polygons keep their corners' [UVs](concepts.md#uvs-and-texturing), so the texture doesn't move;
- a corner that ends up twice in a row in a polygon goes (an edge of no length disappears), and a polygon left with fewer than three corners goes;
- hard edges, seams and [creases](subdivide.md#keeping-edges-sharp-creases) on the edges that join come along.

## Settings

| Setting | What it does |
|---|---|
| **Distance** | Points closer together than this melt into one. Shown in millimetres whatever your units, since it's usually tiny. **0.1 mm** as standard: only points that are really on top of each other. Raise it (1 to 5 mm) to join pieces placed roughly edge to edge. Keep it smaller than your smallest detail, or the detail melts too. From 0 (only points exactly on one spot) to 1 m. |
| **Only at open borders** | Off (standard): any close points weld. On: only points on an **open border** (an edge with a polygon on one side only, where cracks are), so small details elsewhere, such as tiny bevels, never melt even with a larger Distance. |
| **Keep UV seams** | On (standard): where two pieces are joined and their UVs don't meet there, the join is marked as a [UV seam](seams.md), so a later [Unwrap](unwrapping.md) keeps the pieces as they are laid out. Off: the join is left unmarked, and a later Unwrap may join the pieces into one. (A shape with no seams marked at all gets none either way: Unwrap already cuts it where its UVs split.) |
| **Show in viewport** (modifier only) | Off: the viewport shows the shape without it; exports and Bake into shape still use it. |

When the modifier has nothing to weld (no two points within the Distance, or no open borders with *Only at open borders* on), the shape is shown as it is and the modifier says why in amber: *Not worked out: …*.

### Speed

90 000 points take well under a tenth of a second. From about 25 000 points the modifier is worked out in the background while you keep working, showing the last result meanwhile.

## Shortcuts

| Action | Shortcut |
|---|---|
| Find Weld by distance or Weld by distance now | `Space`, then type *weld by distance* |
| Points mode, to weld chosen points | `1` |
| Make the modifier permanent (Bake into shape) | `Ctrl + E` |
| Undo Weld by distance now | `Ctrl + Z` |

Weld by distance has no key of its own.

## Tips and common problems

- **Problem:** the shape still has cracks after welding. **Why:** the two sides are further apart than the Distance. **Fix:** raise the Distance a little at a time (0.5 mm, 1 mm) until the **Mesh** section of the Attributes panel says the **Surface** is **Closed**.
- **Problem:** small details melted away or the shape caved in. **Why:** the Distance is larger than the details. **Fix:** lower it, or tick **Only at open borders** so only cracks are welded.
- **Problem:** after welding, Unwrap joined two UV pieces you wanted apart. **Why:** *Keep UV seams* was off. **Fix:** tick it (or mark the seam by hand, see [Seams](seams.md)).
- **Problem:** the shading shows a soft blend where an imported model had a sharp edge. **Why:** the model was split along its hard edges, and once welded the edge is shaded by the shape's **Smoothing**. **Fix:** use *Smooth by angle*, or mark the edge hard (see [Hard and soft edges](hard-and-soft-edges.md)).
- **Tip:** two pieces that overlap (one pushed into the other) can't be welded: their points aren't on top of each other. Join them with a [boolean](booleans.md) instead.

## Related

- [Connect, Bridge and Weld](connect-bridge-weld.md): Weld (M) melts the points you choose
- [Clean-up](clean-up.md): delete, dissolve, fill holes and fix normals
- [Quadify](quadify.md), the other Clean up tool, often used after welding an import
- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Concepts: weld by distance, open border, crack](concepts.md#mesh-basics)
- Why it works this way: [decision 0152](../decisions/0152-weld-by-distance.md)
