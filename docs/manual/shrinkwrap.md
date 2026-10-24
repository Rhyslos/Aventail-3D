# Shrinkwrap

> Move a shape's points onto another shape's surface: a strap hugging a body, a decal sheet on a curved panel, a road laid on terrain.

**Shrinkwrap** is a [modifier](modifiers.md): each point of the shape (or each point you choose) is moved onto the surface of another shape, the **target**, live. Edit or move the target and the wrapped shape follows; switch the modifier off and the shape is as you modelled it.

You need two objects: the **shape** to wrap, which carries the modifier, and the **target** it wraps onto (its *source*). The target stays an ordinary object of its own, as a [boolean](booleans.md)'s cutter does. Hidden, it still counts, so you can hide it while you look at the result.

## When to use it

- Fitting a piece snugly to a curved surface: a strap or belt round a body, armour plates on a shoulder, a label on a bottle.
- Laying something onto uneven ground: a road, a path or a rug dropped onto terrain along Z.
- A *floater*: a thin sheet of detail (a panel, a vent, a decal) that should sit just above a surface, with **Offset** keeping it off so the two don't flicker through each other.
- Fitting only part of a shape: choose the points that should touch (the bottom of a cloth, the inner side of a strap) and leave the rest.

To bend a shape along a path rather than onto a surface, use [Bend along curve](bend-along-curve.md). To make one shape out of two, use [Booleans](booleans.md).

## How to use it

1. Model the shape to wrap near the target, roughly where and how it should end up (a strip of polygons round a body, a grid above the ground). Give it enough cuts to follow the target's curves: points move, polygons don't.
2. **Select the shape first**, then **Ctrl + click** the target (in the viewport or the [Objects panel](objects.md)).
3. Click **Shrinkwrap onto** on the **modifiers palette** (a sheet pressed onto a hump, on the right of the viewport), or choose **Mesh → Modifiers → Shrinkwrap onto** (under *Deform*), or press **Space**, type *shrinkwrap*, and press **Enter**.
4. Every point moves onto the target's nearest surface. Select the shape to see its settings in the [Attributes panel](attributes.md), under **Modifiers → Shrinkwrap**: choose another **Mode**, or lift it off with **Offset**.
5. When you're happy, **Bake into shape** (**Ctrl + E**) makes it permanent. The target stays.

If the button is greyed out, hover over it: it says what's missing (nothing selected, no target, more than two objects, or a target that can't be used).

On a big target (from about 20 000 polygons with a light shape) the result is worked out in the background; the window keeps working meanwhile, showing the last result.

### Only some points

To wrap only part of the shape (the points that touch the body, say):

1. With the shape selected, press **1** for the Points mode and select the points that should move.
2. In the Attributes panel, under the Shrinkwrap modifier, click **Use selected** beside **Points**. It now says how many points are chosen.

The others stay where they are. **All points** wraps every point again. The chosen points are remembered by their own names, so they stay chosen when you edit the shape elsewhere.

## Settings

| Setting | What it does |
|---|---|
| **Target** | The shape wrapped onto, by name. **Select target** selects it, to change or move it. |
| **Mode** | How each point finds the target's surface. **Nearest surface point** (standard): straight to the nearest place on it, whichever way that is; snug, but points far round a corner can bunch up there. **Along the shape's normals**: along the way the shape faces at that point ([normals](concepts.md#shading-and-normals)), forwards or backwards, whichever meets the target nearer; keeps the points spread as they were, best for a shape already close to the target's form. **Along X**, **Along Y**, **Along Z**: straight along one of the shape's own axes, either way, like dropping a sheet onto the ground (**Along Z**). Along a line, a point that meets nothing stays where it is. |
| **Offset** | How far off the target's surface the points end up, along the way the surface faces there: a few millimetres above keeps a floater from flickering through the surface. Below 0, the points sink inside it. |
| **Points** | **All points** (standard), or the number chosen with **Use selected** (see *Only some points* above). |
| **Show in viewport** | Off: the viewport shows the shape unwrapped (handy while you model it); exports and Bake into shape still wrap it. |

Only the points move: the shape's polygons, [UVs](uvs-and-modelling.md), seams and materials stay as they were, so a texture laid out on it follows it onto the target.

## Shortcuts

| Action | Shortcut |
|---|---|
| Choose the shape, then the target | Click, then `Ctrl + click` |
| Find Shrinkwrap onto | `Space`, then type *shrinkwrap* |
| Choose points for Use selected | `1` (Points mode), then select |
| Make it permanent (Bake into shape) | `Ctrl + E` |

Shrinkwrap has no key of its own.

## Tips and common problems

- **Problem:** the wrapped shape is crumpled, its points bunched together. **Why:** with **Nearest surface point**, points far from the target all go to the same nearest place (a corner, an edge). **Fix:** start the shape closer to the target, or use **Along the shape's normals** or an axis.
- **Problem:** some points didn't move. **Why:** along normals or an axis, a point whose line misses the target stays where it is. **Fix:** move the shape so its lines meet the target, or use **Nearest surface point**.
- **Problem:** the wrapped shape flickers in and out of the target. **Why:** two surfaces in the same place. **Fix:** give a small **Offset** (0.1 to 0.5 cm for a floater).
- **Problem:** the wrapped shape is jagged across the target's curves. **Why:** only its points move, and it has too few. **Fix:** give it more cuts ([Loop cut](loop-cut.md), or more **Segments** on a primitive), or put a [Subdivide](subdivide.md) modifier before Shrinkwrap.
- **Problem:** "Not worked out: its target has no shape". **Why:** the target is a curve that is only a line, or has no polygons. **Fix:** give the curve a sweep ([Curves](curves.md)), or pick another target.
- **Problem:** "Not worked out: none of its chosen points are left". **Why:** the chosen points were deleted, or a modifier before Shrinkwrap (Remesh, Decimate) made new ones. **Fix:** choose them again with **Use selected**, click **All points**, or move Shrinkwrap **Up** above the one that renames them.
- **Problem:** the button says a shape can't be wrapped onto itself. **Why:** a shape can't be wrapped onto itself, nor onto a shape made from it (one wrapped onto the first): each would wait for the other. Groups and lattice cages have no surface.
- **Tip:** to lay a road on terrain, model the road flat above the ground, then wrap it **Along Z** with an Offset of a centimetre or two.

## Related

- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Bend along curve](bend-along-curve.md): the other modifier that lays a shape onto another object
- [Booleans](booleans.md): shapes that work together and stay their own
- [Selecting](selecting.md): choosing points for Use selected
- [Concepts: shrinkwrap, modifier](concepts.md#objects-and-scenes), [normal](concepts.md#shading-and-normals)
- Why it works this way: [decision 0147](../decisions/0147-curve-deform-and-shrinkwrap.md)
