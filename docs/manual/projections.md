# Projections

> Lay UVs on by throwing the texture onto the model from a simple shape — a flat square, a box, a cylinder or a ball — that you place and size with the gizmo, or line the texture up with the world for modular kits.

## When to use it

[Unwrapping](unwrapping.md) cuts the surface along its [seams](seams.md) and lays each piece flat with as little stretch as it can. That's the best choice most of the time. A **projection** works the other way round: like a slide projector, it throws the texture square onto the model from outside, so the texture lands exactly where you aim it. Reach for one when:

- a part is **flat or nearly flat** (a sign, a floor tile, a panel, a decal): **Flat**;
- a part is **boxy** (crates, walls, blocks) and you want each side to get the texture straight on: **Box**;
- a part is **round** (pipes, posts, barrels, wheels): **Cylinder**, like a label wrapped round a can;
- a part is **ball-shaped**: **Ball**, like a globe (squeezed at the top and bottom, as globes are);
- you want the texture to lie **just as you see it now**: **Flat, from the view**;
- you're making a **modular kit** and want a tiling texture to run on across pieces placed side by side: **World-aligned**.

A projection stretches the texture wherever the surface turns away from it (the sides of a flat projection, the top of a cylinder), so it suits shapes close to the projection's own. For anything else, unwrap.

## How to use it

1. Select the objects, or in the Points, Edges or Polygons mode the part you want (nothing selected in those modes means the whole object).
2. Choose **UV → Project → Flat**, **Box**, **Cylinder**, **Ball** or **Flat, from the view**. With the mouse over the UV Editor (or in the UV mode), **E → Unwrap & project → Project** is the same as **Flat, from the view**.
3. The projection is placed round the selection and its shape is drawn over the model in the viewport: a square for Flat, a box, a cylinder (its back line is where the seam goes) or a ball. The UVs are laid on at once; open the UV Editor to watch them.
4. Move, turn and size the projection with the usual gizmo: **W** to move, **R** to turn, **S** to size, **T** for all three. The UVs follow as you drag. Making the projection bigger makes the texture bigger on the model.
5. Press **Enter** or click **Done** (at the top of the window) to keep it, or **Esc** / **Cancel** to take it back. The whole placing is one undo step (**Ctrl + Z**).

Flat projects straight across the selection's thinnest side; the others stand upright (along the world's up). Box, Cylinder and Ball mark [seams](seams.md) where the projected UVs part: where the box's sides meet, and down the cylinder's or ball's back line.

**World-aligned:** select the kit pieces and choose **UV → Project → World-aligned**. Each polygon is projected from the world's axes (like Box), at the project's [texel density](texture-and-density.md) (the pixels per metre in UV → Texture), with no fitting into the square. Two pieces that sit side by side get texture that carries on across the join, wherever they are in the scene. It's recorded at once (no gizmo).

Projections are saved with the project and recorded on the object's [edit list](edit-list.md), like Unwrap.

## Settings

| Setting | What it does |
|---|---|
| **The projection's size** (S) | How big the texture square is on the model: double the size and the texture is half as big on the surface. A Flat, Box or Ball projection keeps the texture square (no stretching one way). |
| **Its place and turn** (W, R) | Where the texture lands, and which way it runs. |
| **Texel density** (UV → Texture) | World-aligned uses it: how many texture pixels per metre. Set it before world-aligning a kit. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Projections | **UV → Project**; **E → Unwrap & project → Project** (flat, from the view) over the UV Editor or in the UV mode |
| Move, turn, size the projection | **W**, **R**, **S** (**T** for all three) |
| Keep it | **Enter**, or **Done** |
| Take it back | **Esc**, or **Cancel** |

## Tips and common problems

- **Problem:** the texture is stretched into stripes on some sides. **Why:** those polygons face away from a Flat projection's direction. **Fix:** use Box, or select only the polygons facing the projection and project the rest separately.
- **Problem:** a cylinder's texture runs the wrong way round the pipe. **Why:** the projection stands along the world's up, but the pipe lies on its side. **Fix:** press **R** and turn the projection to lie along the pipe.
- **Problem:** world-aligned pieces no longer line up after moving them. **Why:** the projection was worked out where they were. **Fix:** place the pieces first, then world-align them again.
- **Problem:** the Unwrap place in the E ring now opens four choices. **Why:** Unwrap shares its place with Unfold across, Unfold up and Project. **Fix:** choose Unwrap in the group, or use **UV → Lay out → Unwrap**.
- **Tip:** pack afterwards (**UV → Lay out → Pack**) if the projected pieces overlap others.

## Related

- [Unwrap, Relax and Straighten](unwrapping.md)
- [Seams](seams.md)
- [Texture size and texel density](texture-and-density.md)
- [Modular kits](modular-kits.md)
- [Transforming](transforming.md) (the gizmo)
- [Concepts: UV map, seam, piece](concepts.md#uvs-and-texturing)
