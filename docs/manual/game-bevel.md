# Game bevel

> One click readies a hard-surface shape for a game the *mid-poly* way: every sharp edge gets a small rounded bevel, and the shading is set up so the big faces look flat and the bevels look round, with no normal map to bake.

**Game bevel** is a command, not a modifier of its own: it adds two [modifiers](modifiers.md) and sets the shape's shading, all as **one undo step**. Afterwards you see and change everything it did in the [Attributes panel](attributes.md), like any modifier you added by hand.

### Why "mid-poly"?

Game props are usually made one of two ways:

- **High to low:** model a detailed version, model a simple version, and *bake* the detail into a normal map. Accurate, but slow, and every change means baking again.
- **Mid-poly:** model one shape with a few more polygons than a low-poly model: small bevels on its edges, and [weighted normals](concepts.md#shading-and-normals) so it shades cleanly. No baking, quick to change, and it holds up close. Many modern games use it for props, buildings and vehicles, often with tiling textures or [trim sheets](trim-sheets.md).

The game bevel is the mid-poly setup in one step. Doing it by hand takes three things that are easy to get wrong:

1. a [Live bevel](live-bevel.md) on the sharp edges;
2. the bevels shading **round**: no hard edges left along or across them;
3. [Weighted normals](weighted-normals.md) **last** in the list, so the big faces shade flat.

## When to use it

- **Hard-surface game props**: crates, panels, machines, weapons, furniture, buildings. Anything with flat faces and sharp edges that should catch the light.
- **After booleans**: a shape cut with [Subtract](booleans.md) gets its cut edges bevelled too, and they shade round like the rest.
- While the shape is still changing: everything it adds is live, so the bevels follow every edit.

Not for organic shapes (characters, rocks, cloth): they have no sharp edges to bevel. For smoothing those, see [Subdivide](subdivide.md).

## How to use it

1. Select one or more shapes.
2. Click **Game bevel** on the **modifiers palette** (the column on the right of the viewport; a block with rounded top corners, arrows on top and a tick, after Weighted normals), or choose **Mesh → Modifiers → Game bevel** (under *Game ready*), or press **Space**, type *game bevel*, and press **Enter**.
3. Each shape now has, under **Modifiers** in the Attributes panel:
   - a **Live bevel**: every edge sharper than **30°** bevelled **2 cm** wide in **2** segments, with **Soft bevels** on;
   - **Weighted normals**, last in the list.
4. Go on modelling. To change the bevel (wider, more segments, only some edges), change the Live bevel's settings. One **Ctrl + Z** takes the whole game bevel back.
5. [Export](export-and-import.md) as usual: Godot, Substance Painter and other programs get the bevelled shape with its normals, so it looks the same there. Or **Bake into shape** (**Ctrl + E**) to make it permanent.

### What it does, exactly

For each selected shape:

- **Smoothing.** If the shape is smoothed by angle (every new shape is, at 30°), it switches to **shading by its marked edges**, keeping its look: the edges that were sharper than the angle are marked hard first (as when you mark an edge by hand on such a shape). *Why:* smoothed by angle, a 2-segment bevel of a right-angled edge has a 45° step in its middle, sharper than 30°, so each bevel would shade with a crease down it. Shaded by marks, the bevel decides. The Attributes panel's **Last edit** then reads **Game bevel**, with a line saying Smoothing switched from By angle to the marked edges (the marks are part of the game bevel, so there's nothing to open there); the bevel's and the normals' settings are in **Modifiers** below.
- **Live bevel.** If the shape has none, one is added with the standard settings and **Soft bevels** on. If it already has one, it's kept with its own settings, and Soft bevels is switched on (and the bevel switched on, if it was off).
- **Weighted normals.** If the shape has none, one is added at the end of the list. If it already has one, it's moved to the end and switched on.

A shape that already has all this is left as it is. Groups are skipped; an [instance](copies-and-instances.md) can't take it (give it to the original).

### Soft bevels

**Soft bevels** is a Live bevel setting, the one the game bevel turns on. It makes the edges the bevel takes count as **soft**, even when they're marked hard. Without it, a hard edge's bevel keeps a crisp line along both sides (good for low-poly chamfers, see [Bevel](bevel.md)), and Weighted normals, which keeps hard edges, would shade each bevel flat instead of round.

It works live, every time the bevel is worked out, rather than by un-marking edges once. So it also softens edges you make later, and the edges a [boolean](booleans.md) cuts (the boolean marks those hard after your edits, where nothing else could soften them).

## Settings

The game bevel has no settings of its own: it uses the standard ones, which suit a 1 m prop. Change them afterwards in the Attributes panel.

| Setting | Where | What it does |
|---|---|---|
| **Width** | Live bevel | How wide each bevel is. **2 cm** as standard; 0.5 to 3 cm suits most props. Bigger bevels catch more light but cost the look of a crisp edge. |
| **Segments** | Live bevel | Rows per bevel. **2** as standard looks round from a game's distance; 3 for close-up items; 1 for a flat chamfer. |
| **Edges**, **Angle** | Live bevel | Which edges are bevelled: by angle (**30°**), the hard ones, or the ones you mark. See [Live bevel](live-bevel.md). |
| **Soft bevels** | Live bevel | On: the bevels shade round even where their edges are marked hard (see above). |
| **Face influence**, **Strength** | Weighted normals | How strongly the big faces win. The standard (75%, 100%) keeps them flat to well under a tenth of a degree. See [Weighted normals](weighted-normals.md). |
| **Smoothing** | Object | Switched to *marked edges*. Edges the bevel doesn't take that are marked hard stay crisp. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Find Game bevel | `Space`, then type *game bevel* |
| Take it back | `Ctrl + Z` |
| Make it permanent (Bake into shape) | `Ctrl + E` |

Game bevel has no key of its own.

## Tips and common problems

- **Problem:** a flat sheet or a sphere shows no bevel, and the Live bevel says *no edge is sharper than the angle*. **Why:** nothing is sharp enough to bevel. **Fix:** lower the Live bevel's **Angle**, or mark the edges you want and set **Edges** to *Marked edges*.
- **Problem:** the polygon count went up a lot. **Why:** every bevelled edge adds a strip of polygons, and every corner a small patch: 2 segments on a box turn 6 polygons into 54. **Fix:** fewer **Segments**, or bevel only the edges that catch the light (*Marked edges*). Add [levels of detail](levels-of-detail.md) for distance.
- **Problem:** an edge I marked hard on purpose now shades round. **Why:** it's bevelled, and Soft bevels softens bevelled edges. **Fix:** raise the Live bevel's Angle above that edge's, give it no Bevel weight (with *Marked edges*), or switch Soft bevels off.
- **Problem:** I painted the bevels in Substance Painter and the paint looks stretched across them. **Why:** each bevel takes the UVs along its edge, so a narrow bevel has almost no room in the UV layout. **Fix:** for small bevels this rarely shows. For detail painted on the bevels, **Bake into shape** when you've finished modelling and [unwrap](unwrapping.md) again: the bevels then get UV room of their own.
- **Tip:** the game bevel adds no [seams](seams.md): the bevel keeps the seams the shape had, moving each to the far side of its strip, and with the bevels soft there are no hard edges for the [bake check](bake-check.md) to ask seams for.
- **Tip:** for the cleanest result, model with simple shapes and let the bevel do the edges: a box of 8 points, not one already bevelled by hand.

## Related

- [Live bevel](live-bevel.md), and its Soft bevels setting
- [Weighted normals](weighted-normals.md), always last
- [Modifiers](modifiers.md): switching off, reordering, baking
- [Hard and soft edges](hard-and-soft-edges.md), and the object's Smoothing
- [Booleans](booleans.md), whose cut edges the game bevel bevels too
- [Concepts: bevel, weighted normals, mid-poly](concepts.md#mesh-basics)
- Why it works this way: [decision 0156](../decisions/0156-game-bevel.md)
