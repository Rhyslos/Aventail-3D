# Bend, twist and taper

> Bend a shape into an arc, twist it round an axis, or taper it narrower towards one end, live.

A curved horn, a bent pipe, a leaning tree trunk, a twisted rope or column, a sword blade narrowing to its tip, a table leg that thins towards the floor: all are simple shapes changed along their length. Modelling them bent from the start is fiddly; moving rows of points by hand rarely gives an even curve. The **Bend, twist or taper** [modifier](modifiers.md) does it for you, evenly, and stays live: keep modelling the straight shape and the bend follows; change the angle and it changes; switch it off and the shape is straight again.

It is one modifier with three **kinds**, because they share everything else (the axis, the limits, the centre), and you can switch between them without losing those:

- **Bend** curls the shape into an arc, through an **Angle** in all.
- **Twist** turns each slice of the shape round the axis, a little more along its length, like wringing a cloth.
- **Taper** makes each slice narrower (or wider) along the axis, by an **Amount**, with a **Curve** to bulge or pinch its middle.

It is a [deformer](concepts.md#objects-and-scenes): it only moves points. The shape's polygons, [UVs](uvs-and-modelling.md), seams and materials stay as they were, so a texture laid out on the straight shape still fits the bent one.

## When to use it

- **Bend**: horns, tusks, claws, bent pipes and rails, a leaning lamp post, a banana; a flat plank curled into a ring (360°).
- **Twist**: twisted columns and candles, drill bits, rope and cable, a screw thread's look, a propeller blade.
- **Taper**: blades, spikes, table and chair legs, tree trunks, a vase (with **Curve** for its belly), a bottle's neck.
- Several at once: add one of each, for a horn that tapers, twists and bends.

The modifier moves points, it doesn't add any: a shape bends only where it has points along its length. A plain box has points at its ends only, so it would just lean. Give it **slices** first: more **Height segments** on a [cylinder](primitives.md), [Loop cuts](loop-cut.md) along it, or a [Subdivide](subdivide.md) modifier before it. To bend a shape along a path you draw, use [Bend along curve](bend-along-curve.md); to push and pull a shape freely, use a [Lattice](lattice.md).

## How to use it

1. Select one or more shapes (in the Model mode). The modifier works **from each shape's pivot**: a new primitive's pivot is at its bottom, so it bends over from its foot.
2. Click **Bend, twist or taper** on the **modifiers palette** (the column on the right of the viewport; a bar curling over to the right), or choose **Mesh → Modifiers → Bend, twist or taper** (under *Deform*), or press **Space**, type *bend*, *twist* or *taper* and press **Enter**.
3. The shape bends **45°** along its own **Z** (up), towards its X. In the modifier list it is called **Bend**.
4. While the shape is selected, a light blue **helper** is drawn over it: a line along the axis, following the bend; a cross where the bend starts and ends (the **limits**); and a small cross at the **centre**. It shows you what the modifier works along, even through the shape.
5. In the [Attributes panel](attributes.md), under **Modifiers → Bend**, change the **Kind** to **Twist** or **Taper**, and set the **Angle** (or **Amount**), the **Axis** and, if you like, the limits and the centre.
6. [Export](export-and-import.md) as usual, or **Bake into shape** (**Ctrl + E**) to make it permanent.

On a shape with very many points (from about 100 000) the result is worked out in the background; the window keeps working meanwhile, showing the last result.

### The centre, and where it bends from

The **centre** is the place along the axis that stays still: the shape bends, twists and tapers *away* from it. As standard it is the shape's **pivot**.

- Pivot at the shape's foot (a new primitive): the foot stays, the top bends over.
- Pivot in the middle: both halves bend away from the middle, like a **C** (or a twist turns both ends opposite ways).
- To bend from somewhere else, move the pivot ([Pivots](pivots.md)) or set **Centre X / Y / Z**, which moves the centre away from the pivot.

### Only part of the shape: the limits

**Limit from** and **Limit to** say where along the axis the modifier works, as a share of the shape's length along it: 0% is its lowest point along the axis, 100% its highest. Past the limits the shape carries on **unchanged but moved along**: above a bend's upper limit the rest of the shape goes on straight, at the angle the bend left it, with no tear.

The centre is brought within the limits: a centre below them starts the bend at the lower limit. So to bend only the top half of a pole standing on its pivot, set **Limit from** 50% and **Limit to** 100%: the bottom half stays straight and the bend starts halfway up. The helper's crosses move to the limits.

## Settings

| Setting | What it does |
|---|---|
| **Kind** | **Bend** (standard), **Twist** or **Taper**. Switching keeps every other setting. The modifier list shows the kind's name. |
| **Axis** | Which of the shape's own axes it works along: **X**, **Y** or **Z** (standard; up). |
| **Angle** | Bend and twist. A bend's: how far the shape curls over in all, between the limits: 90° lays the far end on its side, 180° turns it back down, 360° curls it into a ring. A twist's: how far the far end turns round the axis compared with the centre. 45° as standard; below 0 goes the other way. Up to ten turns either way. |
| **Direction** | Bend only. Which way it bends, in degrees round the axis: 0° (standard) bends towards the next axis round (Z bends towards X, X towards Y, Y towards Z); 90° a quarter turn on from there, 180° the opposite way. |
| **Amount** | Taper only. How much wider the shape gets one whole limit's length from the centre: **−50%** (standard) makes it half as wide at the far end, **−100%** brings it to a point, **100%** makes it twice as wide. The centre keeps its size. It never gets thinner than nothing: past a point, it stays a point. |
| **Curve** | Taper only. Bulges the middle of the limits out (above 0%) or pinches it in (below 0%), with the two ends as the Amount leaves them: a barrel, a vase's belly, an hourglass's waist. 0% as standard. |
| **Limit from**, **Limit to** | Where along the axis it works, as shares of the shape's length along it: 0% and 100% (standard) cover the whole shape, whatever its size. Either order works. Past them the shape carries on unchanged. Limits with no length between them do nothing. |
| **Centre X**, **Centre Y**, **Centre Z** | Where the centre is, from the shape's pivot, in the shape's own directions and the project's [units](concepts.md#objects-and-scenes). 0 as standard: the pivot itself. Moving it across the axis (X or Y for a Z bend) also moves the line the shape is bent or twisted round. |
| **Show in viewport** | Off, the viewport skips it (and its helper) while exports and Bake into shape still use it. |

The modifier works on the shape as the modifiers before it leave it, so the **limits** are measured on that shape: a Bend after a Twist bends the twisted shape.

## Shortcuts

| Action | Shortcut |
|---|---|
| Add Bend, twist or taper | No key: the modifiers palette, **Mesh → Modifiers → Bend, twist or taper**, or **Space** and *bend*, *twist* or *taper* |
| Make it permanent | **Ctrl + E** (Bake into shape) |

## Tips and common problems

- **Problem:** the shape leans or shears instead of bending smoothly. **Why:** it has too few points along the axis (a plain box has only its two ends). **Fix:** give it slices: more height segments, [Loop cuts](loop-cut.md), or a [Subdivide](subdivide.md) modifier moved **Up** above the bend.
- **Problem:** it bends the wrong way, or along the wrong length. **Why:** the **Axis** or **Direction** isn't the one you meant. **Fix:** look at the helper line: it runs along the axis. Change **Axis**, or set **Direction** to 90°, 180° or 270°, or make the **Angle** negative.
- **Problem:** the whole shape moves, not just the far end. **Why:** the centre (the pivot) is in the middle or at the far end. **Fix:** move the pivot to where the bend should start, or set **Centre X / Y / Z**.
- **Problem:** a taper made the bottom wider, not the top thinner. **Why:** the centre is in the middle, so the bottom half tapers the other way. **Fix:** move the centre (or the pivot) to the end that should keep its size.
- **Problem:** a bend of a thick shape pinches on its inside. **Why:** with a big angle over a short length, the inside of the curve is squeezed past the centre of the bend. **Fix:** spread the bend over more length (wider limits), or make the angle smaller.
- **Tip:** for a curled shape (a ram's horn), add a **Taper**, then a **Twist**, then a **Bend**, in that order: each works on the result of the one before.
- **Tip:** the helper shows only while the shape is selected; switch the modifier off to compare with the straight shape.

## Related

- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Bend along curve](bend-along-curve.md): bend a shape along a curve you draw
- [Lattice](lattice.md): bend and swell a shape by moving a cage's points
- [Pivots](pivots.md): move the pivot the bend works from
- [Concepts: deformer, modifier, pivot](concepts.md#objects-and-scenes)
- Why it works this way: [decision 0158](../decisions/0158-bend-twist-taper.md)
