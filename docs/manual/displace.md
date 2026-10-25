# Displace

> Push a shape's points in and out by a noise, live: rough rock, lumpy ground, dented metal, cobbles, bark.

Nature is rarely smooth, and worn things aren't either. Moving hundreds of points by hand to rough up a boulder or a patch of ground takes ages and looks hand-made. The **Displace** [modifier](modifiers.md) does it with a [noise](concepts.md#objects-and-scenes): a pattern of random-looking bumps that is the same every time for the same settings. Each point is pushed out or in by the noise's value where it is. Like every modifier it stays live: keep modelling the shape and the bumps follow; change a setting and they change; switch it off and the shape is as you modelled it.

Displace is a [deformer](concepts.md#objects-and-scenes): it only moves points. The shape's polygons, [UVs](uvs-and-modelling.md), seams and materials stay as they were, so a texture laid out on the smooth shape still fits the bumpy one.

## When to use it

- **Rocks and cliffs**: a sphere or a box with enough cuts, given a **Smooth** noise, then perhaps a second Displace with **Ridged** for cracks and crests.
- **Ground and terrain**: a [plane](primitives.md) with plenty of segments, pushed **Along Z**.
- **Cobbles, scales, hammered metal**: **Cells** gives rounded cells with grooves between them.
- **Damage on part of a model**: choose the points round a dent (see *Only some points*) and push them in with a negative **Strength**.
- **Breaking up a perfect shape** a little: a low Strength makes a crate, a wall or a tree trunk look less computer-made.

Displace moves points, it doesn't add any: bumps smaller than the distance between the shape's points can't show. Give it enough points first: more **Segments** on a primitive, [Loop cuts](loop-cut.md), or a [Subdivide](subdivide.md) modifier before it. For a single dent or bulge you shape yourself, a [Lattice](lattice.md) or moving points by hand is the better tool.

## How to use it

1. Select one or more shapes (in the Model mode).
2. Click **Displace** on the **modifiers palette** (the column on the right of the viewport; a block with a bumpy top and arrows up and down), or choose **Mesh → Modifiers → Displace** (under *Deform*), or press **Space**, type *displace* and press **Enter**.
3. The shape gets soft bumps up to **5 cm** high and about **25 cm** across. In the [Attributes panel](attributes.md), under **Modifiers → Displace**, drag **Strength** and **Scale** until it looks right, and pick the **Noise**.
4. Not the bumps you wanted? Change the **Seed**: another number, other bumps of the same kind.
5. [Export](export-and-import.md) as usual, or **Bake into shape** (**Ctrl + E**) to make it permanent.

On a shape with many points (from about 40 000 at the standard settings) the bumps are worked out in the background; the window keeps working meanwhile, showing the last result.

### Only some points

To roughen or dent only part of the shape:

1. With the shape selected, press **1** for the Points mode and select the points to push.
2. In the Attributes panel, under the Displace modifier, click **Use selected** beside **Points**. It now says how many points are chosen.
3. Set **Falloff**: how far round the chosen points the push fades out, so there's no step at the edge. 0 pushes only the chosen points.

The falloff is measured **along the surface**, so it never jumps across a gap to a part of the shape that only lies near (the other side of a thin wall). **All points** pushes every point again. The chosen points are remembered by their own names, so they stay chosen when you edit the shape elsewhere.

## Settings

| Setting | What it does |
|---|---|
| **Noise** | The bumps' look. **Smooth** (standard): soft rolling lumps, pushing both in and out, so the shape keeps about its size; for rock, ground and anything weathered. **Ridged**: sharp crests where the smooth noise would cross the surface; for craggy stone, bark and crumpled sheet. **Cells**: rounded cells with sharp grooves between them; for cobbles, scales, hammered or pitted metal. Ridged and Cells push outward only (inward with a negative Strength). |
| **Strength** | How far the highest bumps push the points, in the project's [units](concepts.md#objects-and-scenes). 5 cm as standard. Below 0, the bumps go the other way: ridges and cells become grooves and dents. For a rock 1 m across, try 5 to 15 cm; for subtle wear on a prop, a few millimetres. |
| **Scale** | How big the bumps are, about from one to the next. 25 cm as standard. Keep it a few times the distance between the shape's points: much smaller and the bumps fall between the points and the result looks like random noise. |
| **Detail** | How many layers of ever smaller bumps sit on the big ones, each half the size and half the height of the one before (*octaves*). 3 as standard, 1 to 8. 1 is the plainest; more layers look more natural but need more points to show. |
| **Seed** | Another number gives other bumps of the same kind. The same number always gives the same bumps, on any computer, so a saved project opens looking exactly as it did. |
| **Direction** | Which way points are pushed. **Along the normals** (standard): out from the surface, the way it faces at each point ([normals](concepts.md#shading-and-normals)); for rocks and anything round. **Along X**, **Along Y**, **Along Z**: straight along one of the shape's own axes; **Along Z** for ground and terrain, so the bumps go up and down only. |
| **Points** | **All points** (standard), or the number chosen with **Use selected** (see *Only some points* above). |
| **Falloff** | With chosen points, how far from them (along the surface) the push fades out. 20 cm as standard; 0 gives a sharp edge. Greyed out when all points are pushed. |
| **Show in viewport** | Off, the viewport skips the bumps while exports and Bake into shape still use them. |

The noise is read where each point is (in the shape's own space, after any modifiers before Displace). So moving or turning the whole object doesn't change its bumps, but moving the shape's points by modelling does.

## Shortcuts

| Action | Shortcut |
|---|---|
| Add a Displace | No key: the modifiers palette, **Mesh → Modifiers → Displace**, or **Space** and *displace* |
| Choose points for Use selected | `1` (Points mode), then select |
| Make it permanent | **Ctrl + E** (Bake into shape) |

## Tips and common problems

- **Problem:** the shape looks spiky or crumpled, not bumpy. **Why:** **Scale** is about as small as the distance between points (or smaller), so neighbouring points get unrelated values. **Fix:** raise **Scale**, lower **Detail**, or give the shape more points.
- **Problem:** hardly anything happens. **Why:** the shape has too few points (a plain cube has eight), or **Strength** is small next to the shape. **Fix:** add points first: more **Segments**, or a [Subdivide](subdivide.md) modifier moved **Up** above Displace. Then raise **Strength**.
- **Problem:** the bumps show hard creases. **Why:** the object is smoothed by angle (30° as standard), and steep bumps are sharper than that. **Fix:** raise the object's **Smoothing** angle in the Attributes panel, or put a Subdivide after Displace to soften them.
- **Problem:** the shape got bigger with Ridged or Cells. **Why:** those push outward only. **Fix:** give a negative Strength for grooves instead, or switch to Smooth, which goes both ways.
- **Problem:** "Not worked out: none of its chosen points are left". **Why:** the chosen points were deleted, or a modifier before Displace (Remesh, Decimate) made new ones. **Fix:** choose them again with **Use selected**, click **All points**, or move Displace **Up** above the one that renames them.
- **Tip:** stack two Displace modifiers: a big, strong **Smooth** one for the overall shape, then a small, weak **Ridged** or **Cells** one for detail.
- **Tip:** for a game model, displace a dense copy for baking normal maps, and keep the low-poly version for the game, or **Bake into shape** and then [Decimate](decimate.md).

## Related

- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Shrinkwrap](shrinkwrap.md): the other modifier with **Use selected** points
- [Subdivide](subdivide.md): more points for the bumps to show on
- [Lattice](lattice.md): bend and swell a shape by hand instead
- [Concepts: noise, deformer, modifier](concepts.md#objects-and-scenes), [normal](concepts.md#shading-and-normals)
- Why it works this way: [decision 0157](../decisions/0157-displace.md)
