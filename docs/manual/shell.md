# Shell

> Give a surface thickness, live: a sheet becomes a plate, a bowl's surface becomes its wall.

Plenty of game props are thin things with a real thickness: sheet-metal panels, armour plates, a bucket's or a cup's wall, a helmet, a lampshade, a sign, a cloth's hem, a pipe cut open along its length. It's easiest to model them as a single surface (a [plane](primitives.md) bent into shape, a [cylinder](primitives.md) without ends) and give that surface its thickness afterwards. The **Shell** [modifier](modifiers.md) does exactly that, and keeps doing it live: keep shaping the surface, or change the thickness, and the wall follows. When you're happy, **Bake into shape** (**Ctrl + E**) makes it real polygons.

A single surface has no inside, so in a game it vanishes when seen from behind (engines usually don't draw the back of a polygon) and light can leak through it. A shelled surface is a proper [closed solid](concepts.md#mesh-basics): two sides and rims joining them.

## When to use it

- **Thin parts**: panels, plates, fins, blades, signs, flags, shelves. Model a plane to the right outline, then shell it.
- **Containers and shells**: cups, bowls, buckets, helmets, lampshades, a car's body. Model the outside, then shell it **Inward** so the outside keeps its size.
- **Open pipes and tubes**: a cylinder without ends, shelled, becomes a pipe with a wall.
- **A thicker version of something already solid**, for a hollow box or a room's walls: shell a closed shape and you get it with a second, smaller (or bigger) copy inside, facing in.

To add depth to only *some* polygons, use [Extrude](extrude-and-inset.md) instead. To round off the shell's sharp rims, add a [Live bevel](live-bevel.md) or [Subdivide](subdivide.md) after it: a new Shell already goes before those (see the tips).

## How to use it

1. Select one or more shapes (in the Model mode).
2. Click **Shell** on the **modifiers palette** (the column on the right of the viewport; a bowl's wall cut through), or choose **Mesh → Modifiers → Shell** (under *Generate*), or press **Space**, type *shell* and press **Enter**.
3. The shape gets **1 cm** of thickness, behind its surface. In the [Attributes panel](attributes.md), under **Modifiers → Shell**, drag **Thickness** and the wall follows as you drag; choose the **Direction** it grows.
4. [Export](export-and-import.md) as usual, or **Bake into shape** (**Ctrl + E**) to make it permanent.

The surface you modelled keeps being the shape you edit: in the Points, Edges and Polygons modes you edit the thin surface, and the shell follows.

## Settings

| Setting | What it does |
|---|---|
| **Thickness** | How thick the surface becomes: the distance between its two sides, in the project's [units](concepts.md#objects-and-scenes). 1 cm as standard. For game props, think real-world: sheet metal 2–5 mm, a plastic cup 2–3 mm, a wooden plank 2–3 cm, a wall 10–20 cm. 0 leaves the surface as it is. |
| **Direction** | Which way the thickness grows from the surface. **Inward** (standard): behind the surface, which stays the outside, so a shape modelled to size stays that size outside. **Outward**: in front of the surface, which stays the inside (for a room modelled from inside, or a sleeve round something). **Both sides**: half each way, the surface in the middle of the wall. "Behind" and "in front" go by the way the polygons face (their [normals](concepts.md#shading-and-normals)). |
| **Even thickness** | On (standard), points at corners move further, by the angle there, so the walls keep the full thickness right into sharp corners: a shelled box has walls of the same thickness everywhere. Off, every point moves the thickness straight out, and corners come out thinner (at a box's corner, about 58% as thick). Very sharp spikes are held back to 5 times the thickness so they can't shoot out. |
| **Fill the rims** | On (standard), where the surface has open [borders](concepts.md#mesh-basics) (a plane's edges, a cup's lip, a pipe's ends) a strip of polygons, the **rim**, joins the two sides, so the result is a closed solid. Off, you get two separate surfaces, the second facing the other way. A closed shape has no borders, so it gets no rims either way. |
| **Show in viewport** | Off, the viewport skips the shell while exports and Bake into shape still use it. |

### Rims, hard edges and UVs

- The rim's edges along both sides are marked [hard](hard-and-soft-edges.md), so the rim shades as a face of its own instead of smearing the light round the edge.
- If the shape has [UVs](concepts.md#uvs-and-texturing), the back side gets the **same UVs** as the front (the two sides of a sheet usually look the same), and each rim strip gets **UVs of its own**, laid flat in a row just **above the texture square**, at the same texel density as the shape, so its texture is as sharp as the rest and never lands on top of the shape's UVs. The rims' edges along both sides are marked as [seams](seams.md), and a closed rim (a cup's lip) is cut once where its row starts.
- Because the rims sit outside the texture square, a texture that tiles (wood, metal) shows on them as it is. To paint the rims on their own in Substance Painter, or to bake, **Bake into shape** first and then [Pack](packing.md) the UVs (or [Unwrap](unwrapping.md)), so everything fits in the square.
- The back side and the rims take the material of the polygons they come from.

## Shortcuts

| Action | Shortcut |
|---|---|
| Add a Shell | No key: the modifiers palette, **Mesh → Modifiers → Shell**, or **Space** and *shell* |
| Make it permanent | **Ctrl + E** (Bake into shape) |

## Tips and common problems

- **Problem:** the two sides poke through each other in places. **Why:** the shell isn't fixed where the thickness is more than the shape is wide there: a closed shape shelled inward by more than half its width, a narrow slot, a tight inside bend, or a thin spike. **Fix:** a smaller **Thickness**, **Outward** instead of Inward, or give that part more room. (The app doesn't untangle it for you.)
- **Problem:** the shell grows the wrong way (outside instead of inside). **Why:** "inward" means behind the polygons' fronts; if they face the wrong way, so does the shell. **Fix:** pick the other **Direction**, or turn the polygons round with [Flip normals or Fix normals](clean-up.md).
- **Problem:** a cut or crack in the surface grows a rim along it. **Why:** the two sides of a crack aren't joined, so it's an open border. **Fix:** add [Weld by distance](weld-by-distance.md) before the Shell (a new Weld by distance goes before it on its own).
- **Problem:** "the result would have … polygons, and the most is …". **Why:** a shell has twice the polygons, plus its rims. **Fix:** a simpler shape, or [Decimate](decimate.md) it first.
- **Problem:** the corners of the shell look pinched. **Why:** **Even thickness** is off. **Fix:** switch it on.
- **Tip:** modifiers are worked out in order. A new Shell goes **before** any [Live bevel](live-bevel.md), [Subdivide](subdivide.md) or [Weighted normals](weighted-normals.md) already in the list, so the rims are bevelled and smoothed with the rest, and Weighted normals stays last (a Shell after it would drop its normals). Change the order with **Up** and **Down**.
- **Tip:** a [mirrored](symmetry.md) half is shelled whole, since the mirror comes before every modifier: model half a helmet, shell it, and both halves get the wall with no seam in the middle.
- **Tip:** a Shell after an [Array](array.md) shells the copies; with **Merge touching copies** on they're shelled as one surface.

## Related

- [Modifiers](modifiers.md): how modifiers are added, ordered and baked.
- [Extrude and inset](extrude-and-inset.md): depth for some polygons only.
- [Live bevel](live-bevel.md) and [Subdivide](subdivide.md): rounding the rims.
- [Seams](seams.md), [Packing](packing.md) and [Hard and soft edges](hard-and-soft-edges.md): what the rims get.
- [Concepts](concepts.md): [shell](concepts.md#objects-and-scenes), [border](concepts.md#mesh-basics), [closed solid](concepts.md#mesh-basics), [normals](concepts.md#shading-and-normals), [UVs](concepts.md#uvs-and-texturing).
