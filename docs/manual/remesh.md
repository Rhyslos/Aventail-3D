# Remesh

> Rebuild a closed shape as an even surface of square polygons (quads), whatever its polygons were before.

**Remesh** is a [modifier](modifiers.md): it throws the shape's polygons away and wraps a brand-new, even surface round it. Shapes made with [booleans](booleans.md), [joined](split-and-join.md) from parts, or pushed around a lot end up with long thin triangles, n-gons and uneven detail, which shade badly and are hard to keep editing or to smooth. Remesh gives back the same shape as a clean grid of quads, all about the same size.

It works like this: the app fills a 3D grid of tiny cubes, **[voxels](concepts.md#objects-and-scenes)**, with the shape (each voxel inside or outside), then lays a new surface over the boundary between the inside and outside voxels, one quad per voxel side it crosses. The **Voxel size** is how big those cubes are, and so roughly how big the new polygons are.

## When to use it

- After booleans or joining, to turn a messy result into tidy quads you can keep modelling or smoothing.
- To even out a shape whose detail is all bunched up in some places and spread thin in others.
- As a starting point for a high-poly version to bake from.

It's not for every shape. A remeshed shape's sharp corners are rounded off by about one voxel, its UVs start again (see below), and the new polygons don't follow your edge loops. For a game model that's going to be low-poly, model the clean topology by hand, or lighten a heavy shape with [Decimate](decimate.md); Remesh suits high-poly shapes, sculpt-like shapes and boolean-heavy blockouts.

## How to use it

1. Select one or more **closed** shapes: shapes with no holes in their surface (see the tips if yours isn't).
2. Click **Remesh** on the **modifiers palette** (the ball covered by a grid, on the right of the viewport), or choose **Mesh → Modifiers → Remesh**, or press **Space**, type *remesh*, and press **Enter**.
3. The shape is shown remeshed at the standard voxel size of 2 cm. In the [Attributes panel](attributes.md), under **Modifiers → Remesh**, change **Voxel size** for finer or coarser polygons, and **Smooth** to even out the surface.
4. Keep modelling the original shape if you like: the remesh follows. When you're happy, **Bake into shape** (**Ctrl + E**) makes it a plain mesh to edit point by point.

On a big shape with a small voxel size, the result can take a moment; the window keeps working meanwhile, showing the last result.

## Settings

| Setting | What it does |
|---|---|
| **Voxel size** | How far apart the grid is: about the size of the new polygons. Smaller keeps more detail and makes many more polygons (half the size, about four times as many). **2 cm** as standard: for a 1 m crate, about 15 000 quads. Try 5–10 cm for a quick, light result. The grid can have at most 2 million voxels; a size too small for the shape is refused, and the panel says the smallest size that fits. |
| **Smooth** | Relax passes that even out the new surface without shrinking it: **2** as standard, **0** for none (crisper, a little stepped on gentle slopes), up to 20. |
| **Show in viewport** | Off: the viewport shows the shape without the remesh (quicker while you model); exports and Bake into shape still remesh it. |

### What the result keeps

- **The shape**, within about a voxel. Sharp corners and edges are rounded off a little: the smaller the voxels, the sharper they stay.
- **Materials:** each new polygon takes the material of the original polygon nearest it, so parts keep their materials.
- **UVs:** none of the old ones can be kept: the polygons are all new. The result gets a simple **box projection**: each polygon is laid flat as seen from the side it faces most (front, back, left, right, top or bottom), with [seams](seams.md) where that side changes. It's enough for a quick check or a tiling texture, not for painting: once the shape is baked, [unwrap](unwrapping.md) it properly.
- **Shading:** no edges are marked hard; the shape is shaded by its own [smoothing](hard-and-soft-edges.md) angle (30° as standard), so the rounded corners shade smoothly.

## Shortcuts

| Action | Shortcut |
|---|---|
| Find Remesh | `Space`, then type *remesh* |
| Make it permanent (Bake into shape) | `Ctrl + E` |

Remesh has no key of its own.

## Tips and common problems

- **Problem:** "Not worked out: the shape isn't closed". **Why:** remeshing needs to know what's inside the shape, so it must be a [closed solid](concepts.md#mesh-basics): no holes in its surface (a plane, a cylinder without caps, a box with a deleted side), and no edge shared by three or more polygons. **Fix:** close it: [Fill hole](clean-up.md) (J); for a flat plane, extrude it into a slab. A half shape with [symmetry](symmetry.md) is fine as long as it closes with its mirror: the mirror comes before the modifiers.
- **Problem:** "the voxels are too small for this shape". **Why:** the grid would need more than 2 million voxels, which would be slow and huge. **Fix:** use the size the message suggests or larger; for a very detailed result, remesh parts separately.
- **Problem:** "nothing is left: the shape is thinner than a voxel". **Why:** no voxel's centre is inside the shape (a thin plate, a wire). **Fix:** make the Voxel size smaller than the shape's thinnest part.
- **Problem:** thin parts merge, or small gaps close up. **Why:** anything thinner than about a voxel can't be told apart. **Fix:** a smaller Voxel size.
- **Problem:** the corners look soft. **Why:** remeshing rounds corners at the scale of a voxel. **Fix:** a smaller Voxel size, or Smooth at 0.
- **Tip:** two shapes overlapping or touching are remeshed as one solid, joined where they meet: a quick way to merge a blockout into one clean surface.

## Related

- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Booleans](booleans.md), whose results Remesh tidies
- [Concepts: voxels, closed solids](concepts.md#objects-and-scenes)
- [Unwrap](unwrapping.md) and [Seams](seams.md), for proper UVs after baking
- Why it works this way: [decision 0142](../decisions/0142-modifier-stack-and-remesh.md)
