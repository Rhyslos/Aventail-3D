# Scatter

> Spread copies of other shapes over a surface: rocks on the ground, rivets on a plate, studs round a shield's rim.

**Scatter** is a [modifier](modifiers.md): the copies are worked out live from the shapes you scatter, so you can keep editing those shapes, the surface, or the settings, and every copy follows. Placing fifty rocks by hand takes an afternoon, and changing your mind about the rock takes another; with Scatter, you model one rock, and the app places the fifty.

You need two kinds of shape: the **surface**, which the copies stand on and which carries the Scatter modifier, and one or more **shapes to scatter** (its *sources*). The shapes to scatter stay ordinary objects of their own, as [cutters](booleans.md) do: move one and nothing changes (the copies stand where the surface puts them); edit it, and every copy changes with it. When there are several, each copy is one of them, picked at random.

Each copy stands on the surface by its [pivot](concepts.md#objects-and-scenes), the point the shape turns round. A new [primitive](primitives.md) has its pivot at the middle of its base, so it stands on the surface rather than half sunk in it. For your own shapes, put the pivot at the foot ([Pivots and Edit pivot](pivots.md)).

## When to use it

- Scattering many small pieces over a bigger one: stones, grass tufts, bolts, spikes, leaves, debris.
- Putting one copy on every point or every polygon: rivets along a plate's grid, studs on a dome.
- Trying a look quickly: change the count, the seed or the sizes and see the result at once.

Scatter is for many copies spread over a surface. For a few copies set out in a row or a ring, copy the shape ([Copies and instances](copies-and-instances.md)); for copies along a line, an *array* modifier comes later this phase.

## How to use it

1. Make the surface (a plane for ground, a sphere, your own model) and the shape or shapes to scatter (a small rock, say). It doesn't matter where the shapes to scatter are.
2. **Select the surface first**, then **Ctrl + click** each shape to scatter (in the viewport or the [Objects panel](objects.md)). The shape selected first is always the surface.
3. Click **Scatter onto surface** on the **modifiers palette** (little cones standing on a hill, on the right of the viewport), or choose **Mesh → Modifiers → Scatter onto surface** (under *Generate*), or press **Space**, type *scatter*, and press **Enter**.
4. Fifty copies are spread at random over the surface. Select the surface to see its settings in the [Attributes panel](attributes.md), under **Modifiers → Scatter**: change the **Count**, try another **Seed**, give the copies a **Random turn** and sizes.
5. **Hide the shapes you scattered** (click their eye in the Objects panel) if you don't want them in your scene or your export on their own. Hidden, they still scatter.
6. When you're happy, **Bake into shape** (**Ctrl + E**) makes the copies part of the surface's mesh for good. The shapes you scattered stay.

If the button is greyed out, hover over it: it says what's missing (nothing selected, only the surface selected, or a shape that can't be scattered there).

On a big scatter (from about 40 000 new polygons) the result is worked out in the background; the window keeps working meanwhile, showing the last result.

### Only on some polygons

To keep copies to part of the surface (only the top of a wall, only one side of a hill):

1. With the surface selected, press **3** for the Polygons mode and select the polygons the copies should go on.
2. In the Attributes panel, under the Scatter modifier, click **Use selected** beside **On polygons**. It now says how many polygons are chosen.

**Whole surface** puts the copies over all of it again. The chosen polygons are remembered by their own names, so they stay chosen when you edit the surface elsewhere.

## Settings

| Setting | What it does |
|---|---|
| **Shapes** | The shapes scattered, by name. **Select shapes** selects them, to edit or hide them. |
| **Place on** | Where the copies go. **Points**: one on each point of the surface. **Polygon centres**: one on the middle of each polygon. **Random over the surface** (standard): a number of copies spread at random, evenly by area, so a polygon twice the size gets twice as many on average. |
| **Count** | How many copies a random spread makes: **50** as standard, up to 100 000. Only for Random. |
| **Seed** | The number all the random choices start from. Another seed gives another spread, with other turns, sizes and shapes; the same seed always gives the same result, on any computer. |
| **Spacing** | The least distance between two copies, from spot to spot: a copy that would stand closer to one already placed is left out. **0** (standard) lets them overlap. Over a random spread, the app tries again for each copy left out, so a large spacing gives an even spread with fewer copies, not none. |
| **Align to surface** | On (standard): each copy stands on the surface, its up turned to face out from it, as a rock lies on a slope or a spike stands out of a ball. Off: every copy stays upright, the way the shape stands. |
| **Random turn** | Turns each copy round its up direction by a random angle up to this: **0°** (standard) all face the same way; **360°** any way at all, which is what rocks and plants usually want. |
| **Smallest size**, **Largest size** | Each copy gets a random size between the two: **100%** is the shape's own size. Both at 100% (standard), all are the same size; 70% and 130% gives a natural mix. |
| **Offset** | Lifts each copy off the surface along its outward direction, or sinks it in below 0: sink rocks a little so they look bedded in. |
| **On polygons** | **Whole surface** (standard), or the number of polygons chosen with **Use selected** (see *Only on some polygons* above). |
| **Show in viewport** | Off: the viewport shows the surface without the copies (handy while you model a heavy scene); exports and Bake into shape still use them. |

### What the copies are like

- **Each copy is the shape as you see it**, with its own [modifiers](modifiers.md), its turn and its size (rotate or scale the shape you scatter, and every copy turns or grows), moved so its pivot stands on the surface. Where it is in the scene doesn't matter.
- **Materials** are matched by name: a rock with a "Stone" material is "Stone" on the surface too if the surface has a "Stone" material (add one in its [Materials](materials.md) section); if it hasn't, the copies take the surface's first material.
- **UVs, seams, hard edges and creases** come along with each copy, so a textured rock is textured the same way on the surface. All copies share the same UV space (they overlap in the UV square), which is what you want for repeated pieces that use one texture.

### What goes out in an export

The surface goes out with all its copies **as one mesh**, exactly as the viewport shows it. The shapes you scattered go out as well, as objects of their own, unless you hide them.

glTF has a way to store repeated pieces as one shape placed many times (*GPU instancing*), which keeps big scatters small, but Godot doesn't read it yet (it would show only one rock), so the app always writes the copies out in full. A scene with 10 000 rocks of 100 polygons is a million polygons: keep the rock light, or use fewer.

## Shortcuts

| Action | Shortcut |
|---|---|
| Choose the surface, then the shapes to scatter | Click, then `Ctrl + click` |
| Find Scatter onto surface | `Space`, then type *scatter* |
| Choose polygons for Use selected | `3` (Polygons mode), then select |
| Make it permanent (Bake into shape) | `Ctrl + E` |

Scatter has no key of its own.

## Tips and common problems

- **Problem:** the copies are half sunk into the surface, or float above it. **Why:** they stand on their pivot, and your shape's pivot isn't at its foot. **Fix:** move the shape's pivot to its base ([Pivots and Edit pivot](pivots.md)), or use **Offset**.
- **Problem:** every copy faces the same way and the result looks artificial. **Fix:** set **Random turn** to 360° and the sizes to something like 80% and 120%; try a few **Seeds**.
- **Problem:** copies stand sideways or upside down. **Why:** with **Align to surface** on, the shape's up (Z) points out of the surface; a shape modelled lying down stands on its side. **Fix:** rotate the shape you scatter so it stands upright, or switch Align to surface off.
- **Problem:** fewer copies than the Count. **Why:** **Spacing** is too large for that many to fit. **Fix:** lower Spacing or accept the even spread you have.
- **Problem:** "Not worked out: none of the polygons it was limited to are on the shape any more". **Why:** the chosen polygons were deleted, or a modifier before Scatter (Remesh, Decimate) made new ones. **Fix:** choose them again with **Use selected**, click **Whole surface**, or move the Scatter modifier **Up** above the one that renames them.
- **Problem:** the button is greyed out and says a shape can't be scattered. **Why:** a shape can't be scattered onto itself, nor onto a shape that is made from it (a shape scattered over the first, or an instance of it): each would wait for the other. Groups and lattice cages have no shape to scatter.
- **Problem:** deleting a shape you scattered made its copies vanish. **Why:** the copies are worked out from it; with the last shape gone, the modifier goes too. **Fix:** **Ctrl + Z** brings both back. To keep the copies without the shape, Bake into shape first.
- **Tip:** scatter on **Points** of a low-polygon sphere for studs on a dome, or on **Polygon centres** of a plane cut into a grid for an even pattern of rivets.
- **Tip:** put Scatter after [Subdivide](subdivide.md) in the list for copies on the smooth surface, or before it to have the copies smoothed too.

## Related

- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Booleans](booleans.md): another way shapes work together and stay their own
- [Pivots and Edit pivot](pivots.md): where a copy stands
- [Materials](materials.md): matched by name
- [Concepts: scatter, pivot, instance](concepts.md#objects-and-scenes)
- Why it works this way: [decision 0146](../decisions/0146-scatter.md)
