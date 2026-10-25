# Quadify

> Join pairs of triangles back into four-sided polygons (quads), the squarest first, so a shape made of triangles is easy to edit and smooth again.

Shapes often reach you as nothing but [triangles](concepts.md#mesh-basics): exported from another program or a game, scanned, lightened with [Decimate](decimate.md), or cut by a [boolean](booleans.md). Triangles are awkward to keep modelling: an [edge loop](concepts.md#mesh-basics) or a [loop cut](loop-cut.md) stops at the first triangle it meets, and [Subdivide](subdivide.md) smooths a triangle into a lump. Most of those triangles come in pairs that were once a quad, cut corner to corner. **Quadify** finds those pairs and takes away the edge between them, so each pair is one quad again.

It is two things:

- the **Quadify modifier**: a [modifier](modifiers.md), worked out live, so the shape underneath stays as it is and you can change the settings or switch it off at any time;
- **Quadify now**: a one-off clean-up, recorded as an [edit](edit-list.md) like a bevel or a dissolve, on the selected polygons or on whole shapes. Its settings stay changeable under **Last edit**.

## When to use it

- **A triangulated model you want to keep editing**: an imported OBJ or a shape from a game. Quadify it once with **Quadify now**, and loop cuts and edge loops run through again.
- **After Decimate**, to tidy its result into quads where it can (add Quadify after Decimate in the list). Decimate's own *Quads where flat* only joins pairs lying flat; Quadify also joins them round curves.
- **Before Subdivide**: smoothing quads gives an even, round result; smoothing triangles makes lumps. A new Quadify modifier goes before any Subdivide already there.
- **Before Struts**: every edge becomes a bar, so a triangulated shape gets a diagonal bar across each square. A new Quadify goes before any [Struts](struts.md) already there.
- When you'll keep changing the shape underneath (the modifier), or when you just want it done and then go on editing by hand (Quadify now).

It can't make quads where the triangles don't pair up well, such as a fan of triangles round a point, the end of a cylinder, or a mesh of long thin slivers: those stay triangles. For a whole new, even layout of quads whatever the polygons were, use [Remesh](remesh.md) instead (at the price of new UVs).

## How to use it

### Once: Quadify now

1. Select one or more shapes.
2. Either stay in the **Model mode** to quadify the whole shapes, or press **3** for the **Polygons mode** and select the polygons to quadify (a pair is joined only when both of its triangles are selected).
3. Choose **Mesh → Clean up → Quadify now**, or press **Space**, type *quadify now*, and press **Enter**.
4. The status bar says how many pairs were joined. In the [Attributes panel](attributes.md), **Last edit** shows its settings: change them and the shape follows. **Ctrl + Z** takes it back.

The settings you choose are remembered for the next Quadify now. If your settings join nothing, it's still recorded (and the status bar says so), so you can loosen them under Last edit.

In the Points and Edges modes Quadify now is greyed out: rest the mouse on it to see why.

### Live: the Quadify modifier

1. Select one or more shapes.
2. Click **Quadify** on the **modifiers palette** (the column on the right of the viewport; a square with dots on its corners and a dashed line corner to corner), or choose **Mesh → Modifiers → Quadify** (under *Clean up*), or press **Space**, type *quadify*, and press **Enter**.
3. The viewport shows the shape with its triangles paired into quads. Change the settings in the Attributes panel under **Modifiers → Quadify**.
4. [Export](export-and-import.md) as usual, or **Bake into shape** (**Ctrl + E**) to make it permanent.

**Where it goes in the list.** A new Quadify is put **before** any [Live bevel](live-bevel.md), [Subdivide](subdivide.md) or [Weighted normals](weighted-normals.md) the shape already has: bevelling and smoothing work best on quads. Anything else, it's added after. Move it with **Up** and **Down**.

## How it chooses

Every edge between two triangles is a candidate. Taking it away makes a four-sided polygon, so Quadify looks at that quad:

- **Is it flat enough?** The two triangles may lean from each other by no more than **Max angle**. On a flat floor they don't lean at all; round a ball they lean a little; across a box's corner they lean 90°.
- **Is it square enough?** No corner of the quad may be further from a right angle (90°) than **Max shape distortion**. A square cut in two scores 0°; two equal-sided triangles make a diamond with 60° and 120° corners, 30° off.
- **Is it allowed?** Edges you want to keep are never taken away (see the *Keep* settings). Neither is an edge with a [crease](subdivide.md#keeping-edges-sharp-creases) or a [bevel weight](live-bevel.md#marking-edges-with-a-bevel-weight), since those mean something to a modifier, nor a quad with a dent (a corner pointing inward).

Then the best candidates go first: the squarest and flattest quads, each triangle joining only one pair. So in a grid of squares cut corner to corner, each square's own two triangles are joined, not two halves of neighbouring squares, which would make a slanted shape. The same shape and settings always give the same result.

A joined quad keeps the [UVs](concepts.md#uvs-and-texturing) of its corners and the material of its triangles. Points never move.

## Settings

| Setting | What it does |
|---|---|
| **Max angle** | How far the two triangles may lean from each other and still be joined. **40°** as standard: flat and gently curved areas join, sharp folds don't. Lower (5° to 10°) for quads that stay flat; higher to join round tight curves. |
| **Max shape distortion** | How far any corner of a new quad may be from a right angle. **40°** as standard: corners from 50° to 130°. Lower for only near-square quads; up to 90° takes any quad without a dent. |
| **Keep UV seams** | On (standard): never join across a [UV seam](seams.md), marked or where the UVs split, so the texture stays exactly as it was. |
| **Keep hard edges** | On (standard): never join across an edge the shading shows hard, following the shape's **Smoothing** (the edges marked hard, or those sharper than its Smoothing angle; see [Hard and soft edges](hard-and-soft-edges.md)). For Quadify now, it's the Smoothing the shape had when you ran it. |
| **Keep material borders** | On (standard): never join two triangles with different [materials](materials.md). |
| **Show in viewport** (modifier only) | Off: the viewport shows the shape without it; exports and Bake into shape still use it. |

When the modifier has nothing to join (the shape has no triangles, or none may be joined with these settings), the shape is shown as it is and the modifier says why in a grey note: *Nothing to do: …*. That isn't a problem: there is nothing to join on this shape with these settings.

### Speed

50 000 triangles take about a tenth of a second. From about 10 000 triangles the modifier is worked out in the background while you keep working, showing the last result meanwhile.

## Shortcuts

| Action | Shortcut |
|---|---|
| Find Quadify or Quadify now | `Space`, then type *quadify* |
| Polygons mode, to quadify chosen polygons | `3` |
| Make the modifier permanent (Bake into shape) | `Ctrl + E` |
| Undo Quadify now | `Ctrl + Z` |

Quadify has no key of its own.

## Tips and common problems

- **Problem:** some triangles stay triangles. **Why:** they have no partner that makes a good enough quad: a fan round a point, a pole of a ball, a triangle beside a seam or a hard edge, or a partner already taken by a squarer pair. **Fix:** raise **Max shape distortion** or **Max angle**, or untick a **Keep** setting; some triangles simply can't pair up, and that's fine for a game model (the engine cuts everything into triangles anyway).
- **Problem:** the new quads are bent or slanted, and shade oddly. **Why:** the limits are high. **Fix:** lower **Max angle** (flatter quads) or **Max shape distortion** (squarer quads).
- **Problem:** nothing happens on a box made of triangles. **Why:** each square's diagonal is marked as a seam or hard edge, or the two halves have different materials. **Fix:** untick the matching **Keep** setting.
- **Problem:** Quadify now joined only part of what you selected. **Why:** in the Polygons mode, a pair is joined only when both triangles are selected. **Fix:** grow the selection, or quadify the whole shape in the Model mode.
- **Tip:** Quadify doesn't change how heavy a model is in a game: a quad is still two triangles to the engine. It's about editing and smoothing, not polygon count.

## Related

- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Decimate](decimate.md), which makes triangles Quadify can tidy up, and its *Quads where flat*
- [Clean-up](clean-up.md): delete, dissolve, fill holes and fix normals
- [Remesh](remesh.md), for a new even layout of quads from scratch
- [Subdivide](subdivide.md), which smooths quads best
- [Triangles](triangulate.md): the opposite, Show triangles and the Triangulate modifier
- [Concepts: quad, triangle, quadify, modifier](concepts.md#mesh-basics)
- Why it works this way: [decision 0150](../decisions/0150-quadify.md)
