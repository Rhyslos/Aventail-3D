# Array

> Repeat a shape in a line, a grid or a ring, live: edit the one shape and every copy follows.

Game props are full of repeats: fence posts, railings, stair steps, floor tiles, a row of bolts, the spokes of a wheel, the teeth of a gear, rivets round a hatch. Modelling each copy by hand is slow, and changing them afterwards means changing every one. The **Array** [modifier](modifiers.md) makes the copies for you from the one shape you model. The copies are worked out live: change the shape, the count or the spacing at any time, and the copies follow. When you're happy, **Bake into shape** (**Ctrl + E**) makes them real polygons.

The copies are all part of the same object and export as one [mesh](concepts.md#mesh-basics), which is what a game engine wants for a fence or a staircase.

## When to use it

- **A row of the same thing**: posts, planks, windows, pipes. Use the **Line** layout.
- **A field or a wall of the same thing**: tiles, bricks, crates stacked in rows and columns. Use the **Grid** layout.
- **Copies round a middle**: spokes, bolts round a wheel, the blades of a fan, the teeth of a gear, columns round a tower. Use the **Radial** layout.
- **Stairs**: a Line whose step goes up and along at once (Offset X 1 and Offset Z 1, with a step-shaped box).

To mirror a shape (a left half and a right half), use [Symmetry](symmetry.md) instead: it is the app's mirror generator, works live the same way, and welds the middle. To spread copies over a surface at random (rocks on the ground), use [Scatter](scatter.md).

## How to use it

1. Select one or more shapes (in the Model mode).
2. Click **Array** on the **modifiers palette** (the column on the right of the viewport; three boxes in a row), or choose **Mesh → Modifiers → Array** (under *Generate*), or press **Space**, type *array* and press **Enter**.
3. The viewport shows **three copies in a line**, touching, along the shape's X axis. In the [Attributes panel](attributes.md), under **Modifiers → Array**, pick the **Layout** and change its settings: the copies follow as you drag.
4. [Export](export-and-import.md) as usual, or **Bake into shape** (**Ctrl + E**) to make the copies permanent.

The first copy is your shape itself: you keep editing it (in the Points, Edges and Polygons modes) and the others follow.

### A ring round another object

A **Radial** array turns round the shape's own [pivot](pivots.md) as standard. To turn it round something else (the middle of a wheel), pick that object from the **Centre** list. It stays live: move that object and the ring follows. Any object can be the centre, even an empty group placed where you want the middle. Delete it and the ring goes back to turning round the pivot.

If all the copies seem to sit on top of each other, the shape is on the ring's centre: either move the shape away from its pivot (model the spoke or the tooth where it belongs), or raise **Radius**, which moves each copy out from the centre before it is turned.

## Settings

| Setting | What it does |
|---|---|
| **Layout** | **Line** (one after another), **Grid** (along X, Y and Z) or **Radial** (turned round a centre). |
| **Count** (Line, Radial) | How many copies, the shape itself included. 3 in a line and 8 in a ring as standard, up to 1 000. |
| **Count X / Y / Z** (Grid) | How many copies along each axis. 3 × 3 × 1 as standard: a square of nine. |
| **Relative to size** | On (standard), offsets are shares of the shape's size: **1 places copies touching**, 1.5 leaves a gap of half a shape, 0.5 overlaps them. Off, offsets are distances (in the project's [units](concepts.md#objects-and-scenes)). Relative is handy while the shape's size still changes: the copies stay touching. |
| **Offset X / Y / Z** (Line) | The step from one copy to the next. 1, 0, 0 as standard: along X, touching. Up and along at once makes stairs; a negative step goes the other way. |
| **Spacing X / Y / Z** (Grid) | The step between copies along each axis. 1, 1, 1 as standard: touching. |
| **Angle** (Radial) | How far round the ring goes. **360°** (standard) is a whole turn, the copies spread evenly with none on top of the first: 8 copies are 45° apart. Less than a whole turn makes a **fan** with copies at both ends: 3 copies over 90° stand at 0°, 45° and 90°. Below 0 it turns the other way. |
| **Axis** (Radial) | Which of the shape's own axes the ring turns round. **Z** (up, standard) lays the ring flat on the ground, as for bolts on a lid; X or Y stands it up, as for a wheel. |
| **Centre** (Radial) | **Its pivot** (standard), or another object's pivot. |
| **Radius** (Radial) | How far each copy is moved out from the centre before it is turned (along X; along Y when the axis is X). 0 as standard: the copies turn where the shape is. |
| **Merge touching copies** | Off as standard. On, the points where copies touch melt into one (with [Weld by distance](weld-by-distance.md)), so open copies (a strip of fence, a pipe section, a stair's profile) join into one surface without cracks, which smoothing and loop cuts need. Closed copies (boxes) stay closed shapes side by side: only points on [open borders](concepts.md#mesh-basics) weld. The first copy's points stay put. |
| **Merge distance** | Shown while merging: how close points of different copies must be to melt. 0.1 mm as standard, in millimetres. Raise it if the copies don't quite touch. |
| **UVs side by side** | Off as standard: every copy shares the shape's [UVs](concepts.md#uvs-and-texturing), so they all show the same part of the texture (cheap, and right for repeats that look the same). On, each copy's UVs are shrunk into **their own square of the texture**, side by side, so each copy can be painted differently in Substance Painter or get its own lightmap. Each copy then has less of the texture: use it for a handful of copies, not hundreds. |
| **Show in viewport** | Off, the viewport skips the array while exports and Bake into shape still use it. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Add an Array | No key: the modifiers palette, **Mesh → Modifiers → Array**, or **Space** and *array* |
| Make the copies permanent | **Ctrl + E** (Bake into shape) |

## Tips and common problems

- **Problem:** in a ring, the copies all overlap. **Why:** the shape sits on the ring's centre (its pivot). **Fix:** move the shape away from its pivot, raise **Radius**, or pick another object as the **Centre**.
- **Problem:** with *Relative to size* on, a flat shape (a plane) stacks its copies on top of each other when stepping up. **Why:** it has no height, so 1 × its height is nothing. **Fix:** turn *Relative to size* off and use a distance.
- **Problem:** after switching *Relative to size* off, the copies bunch up. **Why:** the numbers stay as they were, so an offset of 1 (one shape's width) becomes 1 cm. **Fix:** type the distance you want, for example the shape's width from the Attributes panel's size.
- **Problem:** copies that touch still shade with a line between them, or smoothing pulls them apart. **Why:** they aren't joined. **Fix:** tick **Merge touching copies**; if they still don't join, raise **Merge distance** a little.
- **Problem:** "it would make … copies, and the most is 100 000". **Why:** a grid's counts multiply (1 000 × 1 000 is a million). **Fix:** lower the counts.
- **Problem:** the viewport is slow with many copies of a detailed shape. **Why:** every copy is real polygons. **Fix:** untick **Show in viewport** while you model, or array a simpler shape and add detail after.
- **Tip:** modifiers are worked out in order. An Array *after* a [Subdivide](subdivide.md) repeats the smoothed shape; *before* it, each copy is smoothed (and with merged copies, smoothed as one surface). Change the order with **Up** and **Down**.
- **Tip:** the [mirror](symmetry.md) comes before every modifier, so arraying a mirrored shape repeats the whole shape. A ring of **Count 2** gives a copy turned half way round (like a propeller's two blades), not a mirrored one: for a mirror, use Symmetry, and **Move the mirror to** to put the mirror where you want it.
- **Tip:** to lay copies along a curve, use a Line array, then [Bend along curve](bend-along-curve.md) after it.

## Related

- [Modifiers](modifiers.md): how modifiers are added, ordered and baked.
- [Symmetry](symmetry.md): the mirror generator.
- [Weld by distance](weld-by-distance.md): what merging uses.
- [Scatter](scatter.md): copies spread over a surface.
- [Concepts](concepts.md): [array](concepts.md#objects-and-scenes), [modifier](concepts.md#objects-and-scenes), [pivot](concepts.md#objects-and-scenes), [UVs](concepts.md#uvs-and-texturing).
