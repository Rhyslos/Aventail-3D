# Live bevel

> Bevel a shape's sharp edges and keep the bevel live: it follows the shape as you go on modelling, and you can change its width or segments at any time.

**Live bevel** is a [modifier](modifiers.md). It does what the [Bevel](bevel.md) tool does, cutting each chosen edge away and putting a strip of new polygons in its place, but it doesn't change your shape. Instead the bevel is worked out again every time the shape changes and shown on top of it.

Why that matters: with the Bevel tool, you bevel once, and the bevel becomes part of the shape. Move a point afterwards, extrude a face or cut a hole, and the bevel stays where it was or has to be redone. Modelling with the bevel live, you keep working on the plain, simple shape (8 points for a box), and every edge that's sharp enough gets its bevel whatever you do: pull the top of a box up and the bevel runs up with it; extrude a new part and its edges are bevelled too. Only when you're done do you make it permanent, or just export, and the bevel goes out with the shape.

## When to use it

- **Hard-surface props for games**: crates, panels, machines, weapons, furniture. Bevelled edges catch the light, so the prop looks solid instead of like a cardboard box.
- **Together with [Weighted normals](weighted-normals.md)**, the usual way to shade a mid-poly game model without baking a normal map: Live bevel rounds the edges, Weighted normals keeps the big faces flat. Put Live bevel **before** Weighted normals.
- **Before [Subdivide](subdivide.md)**, to keep edges tighter when smoothing: a narrow bevel holds the edge in place, the way supporting loops do.
- While a shape is still changing. For a one-off bevel on a few edges of a finished shape, the [Bevel](bevel.md) tool is quicker and lets you drag the width.

## How to use it

1. Select one or more shapes.
2. Click **Live bevel** on the **modifiers palette** (the column on the right of the viewport; a block with a rounded edge and a circling arrow), or choose **Mesh → Modifiers → Live bevel** (under *Generate*), or press **Space**, type *live bevel*, and press **Enter**.
3. Every edge sharper than 30° is bevelled 2 cm wide in 2 segments. Change its settings in the [Attributes panel](attributes.md) under **Modifiers → Live bevel**.
4. Go on modelling: the bevel follows.
5. [Export](export-and-import.md) as usual, or **Bake into shape** (**Ctrl + E**) to make it permanent.

**Where it goes in the list.** A new Live bevel is put **before** any Subdivide or Weighted normals the shape already has, since that's the order that works: the bevel shapes the edges first, smoothing rounds the result, and Weighted normals shades the final shape (it has to be last). Any other modifier it's added after, as usual. You can move it with **Up** and **Down**.

### Choosing which edges

**Edges** decides which edges are bevelled:

- **By angle** (standard): every edge whose two polygons meet more sharply than the **Angle**. A box's edges meet at 90°, so they're bevelled; the sides of a cylinder meet at a few degrees, so they aren't, but its rims are. Lower the angle to take gentler edges too.
- **Hard edges**: the edges the shape's shading shows hard, following its **Smoothing** row in the Attributes panel: the edges you marked hard if it shades by its marks, or those sharper than its Smoothing angle. See [Hard and soft edges](hard-and-soft-edges.md).
- **Marked edges**: only edges you give a **Bevel weight** (below), each bevelled that share of the width. For when no angle picks the right edges: a bevel on the front of a panel but not the back.

### Marking edges with a bevel weight

1. Select the shape and press **2** for the Edges mode.
2. Select the edges to bevel.
3. In the Attributes panel, under **Selected edges**, drag or type a **Bevel weight** from 0 to 1: **1** bevels them at the modifier's full width, **0.5** at half of it, **0** not at all. One drag is one undo step.
4. Edges with a bevel weight are drawn **lime green** in the Points, Edges and Polygons modes, so you can see where they are.

A bevel weight is a recorded edit, like marking an edge hard: undo takes it back, it's made on both sides when [symmetry](symmetry.md) is on, and it stays with its edges when you go on editing. It only does something with a Live bevel set to **Marked edges**.

## Settings

| Setting | What it does |
|---|---|
| **Edges** | Which edges are bevelled: **By angle** (standard), **Hard edges** or **Marked edges** (see above). |
| **Angle** | With *By angle*: edges whose polygons meet more sharply than this are bevelled. **30°** as standard takes a box's edges but not a cylinder's sides. |
| **Width** | How far the bevel reaches into the polygons on each side of an edge, measured on a right-angled edge (other angles are evened out so every bevel looks as wide). **2 cm** as standard suits a 1 m prop; game props usually use 0.5 to 3 cm. Marked edges get their Bevel weight's share. |
| **Segments** | How many rows each bevel has: **1** is a flat cut (a [chamfer](concepts.md#mesh-basics)), **2** (standard) or **3** look round from a game's distance. Where three or more bevelled edges meet, the corner is rounded with a patch of square polygons. |
| **Profile** | The bevel's curve: **100%** (standard) round, **0%** a straight slope, **-100%** hollow, scooped into the shape like a cove. |
| **Stop at neighbours** | On (standard): where bevels would run into each other or past the end of an edge, only those points are narrowed so they meet cleanly; the rest stays at full width. Off: every bevel is full width, and a wide one can fold over itself. |
| **Show in viewport** | Off: the viewport shows the shape unbevelled while you work; exports and Bake into shape still bevel it. |
| **Bevel weight** (Edges mode, *Selected edges*) | The selected edges' bevel weight, 0 to 1. Shows the first edge's when they differ; a new value gives them all the same. |

When there's nothing to bevel (no edge is sharp enough, none is hard, or none is marked), the shape is shown as it is and the modifier says why in amber: *Not worked out: …*.

### Live bevel and Weighted normals

The two are made for each other. With all the edges soft, a bevelled box usually shades badly: each big face's corners average in the thin bevel strips, so the face shades in a gradient. [Weighted normals](weighted-normals.md) after the Live bevel lets the big faces win, so they shade perfectly flat while the bevels catch the light like rounded edges. For this, make the shape's edges soft (or set its Smoothing to a large angle) and keep **Weighted normals last** in the list. The game bevel, coming later in this phase, will set all of this up in one go.

### Speed

The bevel is worked out each time the shape changes. A few hundred bevelled edges take a few milliseconds; from about 1 200 bevelled edges the bevel is worked out in the background while you keep working, showing the last result meanwhile. Bevelling all 20 000 edges of a 10 000-polygon sphere takes about a third of a second.

## Shortcuts

| Action | Shortcut |
|---|---|
| Find Live bevel | `Space`, then type *live bevel* |
| Edges mode, to mark bevel weights | `2` |
| Make it permanent (Bake into shape) | `Ctrl + E` |

Live bevel has no key of its own.

## Tips and common problems

- **Problem:** nothing is bevelled, and the panel says *no edge is sharper than the angle*. **Why:** the shape has no edges sharper than the Angle (a sphere, a flat sheet). **Fix:** lower the **Angle**, or switch **Edges** to *Marked edges* and give the edges you want a Bevel weight.
- **Problem:** the cylinder's sides got bevelled into stripes. **Why:** the Angle is lower than the angle between its sides (360° ÷ sides). **Fix:** raise the Angle above it (30° suits 12 sides or more).
- **Problem:** some bevels are narrower than the Width. **Why:** *Stop at neighbours* narrowed them where they'd otherwise run into each other or past a short edge. **Fix:** that's intended; make the Width smaller for an even bevel, or switch it off if you're sure they won't cross.
- **Problem:** the bevels look faceted. **Why:** the shape's shading shows the bevel's own edges hard (its Smoothing angle is smaller than the step between rows, or its edges are marked hard). **Fix:** more Segments, a larger Smoothing angle, or soften the edges; add [Weighted normals](weighted-normals.md) last to keep the big faces flat.
- **Problem:** an open edge (the rim of a cup) isn't bevelled. **Why:** an edge needs a polygon on each side to be bevelled. **Fix:** give the shape thickness first, or bevel the edges beside it.
- **Tip:** a Live bevel shows in the polygon count of your export. Two segments on every edge of a box turns 6 polygons into 54; keep game props' bevels to the edges that catch the light.

## Related

- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Bevel](bevel.md), the tool, for bevelling chosen edges once
- [Weighted normals](weighted-normals.md), best last, after the Live bevel
- [Subdivide](subdivide.md), whose creases are set in the same *Selected edges* section
- [Hard and soft edges](hard-and-soft-edges.md) and the object's Smoothing
- [Concepts: bevel, bevel weight, modifier](concepts.md#mesh-basics)
- Why it works this way: [decision 0149](../decisions/0149-live-bevel.md)
