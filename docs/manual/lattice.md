# Lattice

> Put a cage of points round one or more shapes, then move the cage's points: the shapes inside bend, squash and swell smoothly with them.

A **[lattice](concepts.md#objects-and-scenes)** (also called a *lattice deformer* or *free-form deformation*, *FFD* in some programs) is a box of points round a shape. The box is an object of its own, the **cage**, drawn as lines. Each shape inside gets a **Lattice** [modifier](modifiers.md) that follows the cage: pull a corner of the cage up and that corner of the shape rises with it, the rest of the shape stretching smoothly to keep up. Your shape's own points never change, so you can reshape it as broadly as you like, switch the bend off, or put the cage back, at any time.

## When to use it

- **Broad changes of shape** that would be slow point by point: taper a tower towards its top, swell the middle of a barrel, lean a door frame, bulge a pillow, bend a sword blade a little.
- On **dense shapes** (a remeshed or subdivided shape, an imported scan) where moving hundreds of points by hand isn't possible.
- To shape **several objects together**: one cage round a group of props bends them all as one.

It's not for exact, local changes to a few points: there, moving the points themselves in the [Points mode](modes.md) is quicker. To bend a shape along a drawn line, [curves](curves.md) are better.

## How to use it

1. Select one or more shapes.
2. Click **Add lattice** on the **modifiers palette** (the cage of nine dots, on the right of the viewport), or choose **Mesh → Modifiers → Add lattice**, or press **Space**, type *lattice*, and press **Enter**.
3. A cage of 2 × 2 × 2 points appears fitted round the shapes, and becomes the selection. Round **one** shape it lines up with the shape, turned as it is; round **several** it lines up with the world. Each shape gets a **Lattice** modifier naming the cage, above any Weighted normals or Triangulate (they finish the shape). If you later resize the shape with [Size to kit grid](modular-kits.md), a cage you haven't changed is fitted round it again.
4. Press **1** for the **Points** mode. The cage's points show as dots. Select them as you would a mesh's (click, box, Ctrl adds, Shift removes) and move, rotate or scale them with the gizmo, or type their place in the [Coordinates panel](transforming.md#typing-exact-values). The shapes follow as you drag.
5. For more places to bend, raise the cage's **Divisions** in the [Attributes panel](attributes.md) (below).
6. When you're happy, select the shape and **Bake into shape** (**Ctrl + E**) to make the bend part of it. The cage stays, bending nothing after that; delete it when you no longer need it.

### Selecting the cage again

- Click one of its **lines** in the Model mode. A cage hugs its shape, so its lines lie along the shape's own edges: a click on a line in front picks the cage; a click on the shape's surface picks the shape.
- Or click its name (**Lattice**) in the [Objects panel](objects.md).
- Or select a bent shape and click **Select cage** beside the Lattice modifier in the Attributes panel.

### Moving the cage, or the shape

Moving, turning or scaling the **cage as a whole** (in the Model mode) doesn't bend anything by itself: a cage whose points are where they started changes nothing. It moves *where* the bend is: the bend is worked out in the cage's own box. Moving the **shape** through a bent cage changes which part of it is bent.

Parts of a shape **outside** the cage's box move as the nearest side of the box does, so the shape carries on with no tear: raise the cage's top and anything above the cage rises with it, unbent.

## Settings

The **cage's** settings, in the Attributes panel's **Lattice** section when the cage is selected:

| Setting | What it does |
|---|---|
| **Divisions X / Y / Z** | How many points the cage has along each side, **2** to **16** (**2 × 2 × 2** as standard: a point at each corner). More points bend the shape in more places: 3 along a side gives a middle row to swell or pinch; 4 or more for S-bends and waves. Changing the divisions keeps the cage's shape as closely as the new points can. |
| **Interpolation** | How the cage's points pull the shape. **Smooth** (standard) blends them as a curve (a *B-spline*): no creases, but the shape follows the points a little less closely, more so with more divisions. **Linear** follows straight between neighbouring points: the shape goes exactly through each point's pull, with a crease along the cage's lines. With 2 points along a side, both are the same along it. |
| **Reset points** | Puts every point back where it started, so the cage bends nothing. |

The section also lists the shapes the cage bends.

The **Lattice modifier's** row, on each bent shape (Attributes panel, **Modifiers**):

| Setting | What it does |
|---|---|
| **Cage** | The cage it follows, with **Select cage** to select it. |
| **On / off**, **Show in viewport**, its place in the list | As for every [modifier](modifiers.md). The modifier's place in the list matters: after a [Subdivide](subdivide.md) it bends the smoothed shape; before it, it bends the cage of the smoothing, which is then smoothed. |

### What the result keeps

The Lattice modifier only **moves points**. Polygons, their order, [UVs](concepts.md#uvs-and-texturing), [seams](seams.md), hard edges, creases and [materials](materials.md) all stay as they were, so a bent shape is textured exactly as before (stretched with the shape). The shading follows the new surface.

## Shortcuts

| Action | Shortcut |
|---|---|
| Find Add lattice | `Space`, then type *lattice* |
| Edit the cage's points | Select the cage, then `1` |
| Make the bend permanent (Bake into shape) | `Ctrl + E` (with the shape selected) |

Add lattice has no key of its own.

## Tips and common problems

- **Problem:** moving a cage point does nothing. **Why:** the shape isn't inside that part of the cage, or the shape's Lattice modifier is switched off or names another cage. **Fix:** check the shape's **Modifiers** section; **Select cage** shows which cage it follows.
- **Problem:** with many divisions, the shape doesn't reach the points. **Why:** **Smooth** blends each point with its neighbours. **Fix:** move the points further, or choose **Linear**.
- **Problem:** a crease runs across the bend. **Why:** **Linear** interpolation follows straight lines between points. **Fix:** choose **Smooth**.
- **Problem:** "Not worked out: the cage is flattened". **Why:** the cage object is scaled to 0 along an axis. **Fix:** give it a size again (or undo).
- **Problem:** Delete does nothing to a cage point. **Why:** a cage always has a point at each place of its grid. **Fix:** lower the **Divisions** for fewer points, or **Reset points**.
- **Tip:** deleting the cage removes the Lattice modifiers that follow it, in the same undo step: the shapes go back to unbent. **Ctrl + Z** brings both back.
- **Tip:** copying a shape *with* its cage gives the copy its own cage; copying the shape alone makes the copy follow the same cage.
- **Tip:** exports leave the cage out: only the bent shapes go into the file, as you see them.

## Related

- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Points mode, and moving points with the gizmo](modes.md), [Move, rotate and scale](transforming.md)
- [Subdivide](subdivide.md), [Remesh](remesh.md)
- [Concepts: lattice, deformer](concepts.md#objects-and-scenes)
- Why it works this way: [decision 0144](../decisions/0144-lattice-cage.md)
