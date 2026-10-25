# Bend along curve

> Bend a shape along a curve: a rope, a rail, a chain of links, a vine round a post, a strap round a barrel.

**Bend along curve** is a [modifier](modifiers.md): the shape is bent live along a [curve](curves.md) object, so you keep editing the shape, the curve or the settings and the bend follows. Model the piece straight, where it is easy (a plank, a row of links, a strip of trim), then draw the path it should take, and the app lays it along the path.

You need two objects: the **shape** to bend, which carries the modifier, and the **curve** it follows (its *source*). The curve stays an ordinary curve: reshape it in the Points mode (press **1**), add points, move it, and the shape follows at once.

## When to use it

- A shape with detail along its length that should follow a path: a rope with twisted strands, a chain, a railing with posts, a carved trim, a cable with clips.
- Wrapping a straight piece round something: a band round a barrel (a closed curve), a strap round a crate.
- Trying a path quickly: draw a rough curve, bend the shape along it, then refine the curve until it sits right.

For a plain tube or a profile along a path (a pipe, a simple cable, a moulding), a **sweep** is simpler: the curve makes the solid itself ([Curves](curves.md)). Bend along curve is for a shape you've modelled yourself, with its own detail. To bend a whole shape smoothly without a path, use a [lattice](lattice.md).

## How to use it

1. Model the shape **straight**, lying along one of its axes (X is easiest), with enough cuts along its length to bend smoothly: a box of one segment bends only at its two ends. Ten to forty cuts along a plank are typical.
2. Draw the path with **Create → Draw curve**, or use a ready-made **Circle**, **Arc** or **Spiral** ([Curves](curves.md)).
3. **Select the shape first**, then **Ctrl + click** the curve (in the viewport or the [Objects panel](objects.md)).
4. Click **Bend along curve** on the **modifiers palette** (a striped bar along a dashed arc, on the right of the viewport), or choose **Mesh → Modifiers → Bend along curve** (under *Deform*), or press **Space**, type *bend*, and press **Enter**.
5. The shape jumps onto the curve, its start at the curve's start. Select it to see its settings in the [Attributes panel](attributes.md), under **Modifiers → Bend along curve**.
6. Reshape the curve: select it (or click **Select curve** in the Attributes panel), press **1**, and move its points and handles. The shape follows.
7. When you're happy, **Bake into shape** (**Ctrl + E**) makes the bend permanent. The curve stays.

If the button is greyed out, hover over it: it says what's missing (nothing selected, no curve, more than two objects, or the second isn't a curve).

### Where the shape sits on the curve

- **Along the curve:** the shape's start (its lowest end along the chosen axis) goes to the curve's start, and each part goes as far along the curve as it was along the axis. **Offset** slides it along; **Stretch to fit** makes it exactly as long as the curve. The curve's start is its first point: **Reverse curve** ([Curves](curves.md)) to run it the other way.
- **Across the curve:** the curve runs through the shape's [pivot](pivots.md). A new cube's pivot is at the middle of its base, so a cube rides **on top of** the curve; a cylinder lying along the curve with its pivot on its middle line is **threaded on** it. To change which, move the shape's pivot ([Pivots and Edit pivot](pivots.md)).
- **Where the shape is in the scene doesn't matter:** moving it changes nothing, as the bent shape always lies on the curve. Its own turn and size count: scale it up and the bent shape is bigger across.
- **Which way is up:** the shape's own up (Z) stays as near to up as the curve allows, turning with the curve as little as possible, the same way a [sweep](curves.md)'s cross-section follows its curve, so it never flips or spins on its own. Add **Twist** to turn it on purpose.

## Settings

| Setting | What it does |
|---|---|
| **Curve** | The curve the shape is bent along, by name. **Select curve** selects it, to reshape or move it. |
| **Axis** | Which of the shape's own axes runs along the curve: **Longest side** (standard: the longest side of its box), or **X**, **Y** or **Z**. Pick one when the shape's longest side isn't its length (a short, wide chain link). |
| **Stretch to fit** | On: the shape is stretched or squashed along its length to run exactly from the curve's start to its end (good for a rope that must reach two points). Off (standard): it keeps its own length. Only the length changes, never the thickness. |
| **Offset** | How far along the curve the shape starts, in your [units](settings.md): slide it along a long path. Below 0 it starts before the curve. |
| **Twist** | Turns the shape round the curve evenly from its start to its end, on top of the way the curve turns it: **360°** is one whole turn (a twisted rope). On a closed curve it is rounded to whole turns, so the ends meet. |
| **Show in viewport** | Off: the viewport shows the shape straight (handy while you model it); exports and Bake into shape still bend it. |

### Past the curve's ends

A shape longer than an open curve (or slid off its end with Offset) carries on **straight**, along the way the curve leaves, so it stays whole. Round a **closed** curve (a circle) it goes **round again**: a bar as long as the circle, with Stretch to fit, closes into a ring (its two ends meet but aren't joined; weld them after baking if you need one piece).

## Shortcuts

| Action | Shortcut |
|---|---|
| Choose the shape, then the curve | Click, then `Ctrl + click` |
| Find Bend along curve | `Space`, then type *bend* |
| Edit the curve's points | Select the curve, `1` |
| Make it permanent (Bake into shape) | `Ctrl + E` |

Bend along curve has no key of its own.

## Tips and common problems

- **Problem:** the shape bends in a few sharp kinks instead of smoothly. **Why:** only points move, and the shape has too few cuts along its length. **Fix:** add cuts (a [Loop cut](loop-cut.md), or more **Segments** on a primitive).
- **Problem:** the shape sits beside the curve, or above it, rather than on it. **Why:** the curve runs through the shape's pivot. **Fix:** move the pivot to where the curve should run ([Pivots and Edit pivot](pivots.md)).
- **Problem:** the shape lies across the curve instead of along it. **Why:** its longest side isn't its length. **Fix:** set **Axis** to the axis the shape runs along.
- **Problem:** the shape folds over itself at a sharp corner of the curve. **Why:** a sharp corner bends everything inside it onto the same spot. **Fix:** make the corner smooth (double-click its point, or **Round corners**, [Curves](curves.md)), or give the shape fewer cuts there.
- **Problem:** "Not worked out: the curve has no length". **Why:** all the curve's points are on one spot. **Fix:** move them apart.
- **Problem:** the button says a shape can't be bent along itself. **Why:** a curve with a sweep is a shape too; a shape can't be bent along itself, or along a curve whose shape is made from it.
- **Problem:** deleting the curve straightened the shape. **Why:** the bend is worked out from the curve; with it gone, the modifier goes too. **Fix:** **Ctrl + Z** brings both back. To keep the bend without the curve, Bake into shape first.
- **Tip:** UVs bend with the shape, so a texture laid out on the straight piece follows the bend: unwrap the shape straight, before bending.

## Related

- [Curves](curves.md): drawing and shaping the path
- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Lattice](lattice.md): bending a shape smoothly with a cage instead
- [Shrinkwrap](shrinkwrap.md): the other modifier that lays a shape onto another object
- [Pivots and Edit pivot](pivots.md): where the curve runs through the shape
- [Concepts: curve, modifier, pivot](concepts.md#objects-and-scenes)
- Why it works this way: [decision 0147](../decisions/0147-curve-deform-and-shrinkwrap.md)
