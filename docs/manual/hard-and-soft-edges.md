# Hard and soft edges, and normals

> Choose where the shading breaks into a crisp edge and where it flows smoothly, and see which way polygons face.

A shape is made of flat polygons, but it doesn't have to look faceted. Across a **soft** edge the light flows smoothly, so a cylinder made of 16 flat sides looks round. Across a **hard** edge the light breaks, so a box's corner looks crisp. Getting this right is a big part of making a game model look good, and it changes nothing about the shape itself: only how it's lit. See [hard and soft edges](concepts.md#shading-and-normals).

## When to use it

- **Soft edges:** on curves made of several polygons: pipes, wheels, rounded corners after a [bevel](bevel.md) with several segments.
- **Hard edges:** on real corners: the edges of a box, a panel, where a flat face meets a curve.
- **Hard edges by angle:** to set a whole object at once. Usually a good start, then fix the odd edge by hand.
- **Normals display:** to check which way polygons face, and to spot [flipped](concepts.md#shading-and-normals) ones.

The primitives already come with sensible edges: a cube's corners are hard, and a cylinder's sides are soft with hard rims.

## How to use it

### By hand

1. In the **Edges** mode, select edges. Or in the **Polygons** mode, select polygons: their edges are used.
2. Press **E** and pick **Hard edges** or **Soft edges** (or use the **Mesh** menu).

Softening a group of polygons makes them shade as one smooth surface. Hardening them gives them a crisp outline.

### By angle

1. Select the objects.
2. Pick **Mesh → Hard edges by angle**.

Every edge whose two polygons meet more sharply than the **Angle** becomes hard, and every other one soft. Change the **Angle** under **Last edit** in the [Attributes panel](attributes.md), and the shape follows.

### Seeing them

In the viewport's **Display** menu:

- **Hard edges** (on as standard): while editing points, edges or polygons, hard edges are drawn in light cyan.
- **Normals**: each polygon of the selected objects shows a short violet line from its middle, the way it faces. A line pointing into a shape means that polygon is flipped: see [Clean-up](clean-up.md).

**Select → Select hard edges** selects every hard edge, to check them or to change them all at once.

![A cylinder in the Edges mode: its two hard rims drawn in light cyan, and short violet lines showing which way each polygon faces.](../screenshots/2026-10-14-hard-edges-normals.png)

## Settings

| Setting | What it does |
|---|---|
| **Angle** (Hard edges by angle, Last edit) | Edges whose polygons meet more sharply than this become hard. 0° is flat. **30°** (the start) makes a box's corners hard and curves of 12 or more sides soft. Lower it for curves with fewer sides. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Hard edges, Soft edges | Tool menu `E`, or the Mesh menu |
| Hard edges by angle | Mesh menu |
| Select hard edges | Select menu |
| Show hard edges, normals | The viewport's Display menu |

## Tips and common problems

- **Tip:** a cylinder with few sides (say 8) has sides meeting at 45°. With the 30° start they come out hard and it looks faceted: set the Angle to 50° to make it round again.
- **Tip:** soft edges between polygons at a sharp angle look smeared and dark. If a corner looks muddy, make that edge hard, or bevel it.
- **Problem:** Hard edges did nothing, and the status bar says they already are. **Why:** every selected edge is already hard. **Fix:** nothing to do, or pick Soft edges.
- **Problem:** the hard edges or normals don't show on a very detailed shape. **Why:** they're drawn only for shapes of up to 50,000 polygons, to keep the view quick.

## Related

- [Bevel](bevel.md): a small bevel on a hard edge catches the light.
- [Clean-up](clean-up.md): flip polygons and fix facing.
- [Concepts](concepts.md): [hard and soft edges, normals](concepts.md#shading-and-normals).
