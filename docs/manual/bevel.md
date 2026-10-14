# Bevel

> Bevel cuts a sharp edge or corner away and puts a strip of new polygons in its place: a flat chamfer, a rounded edge, or a hollow curve (a cove) with more segments.

Real objects almost never have perfectly sharp edges. A slightly bevelled edge catches the light, which is one of the quickest ways to make a game prop look solid and well made instead of like a box. Bevels also make baking and shading behave better on hard-surface models.

![A cube with all twelve edges bevelled with three segments. The Attributes panel shows the Last edit, with its Width, Segments and Profile.](../screenshots/2026-10-13-bevel.png)

## When to use it

- **Soften a hard edge** so it catches the light, like the rim of a crate or the edge of a panel: bevel its edges with one segment (a [chamfer](concepts.md#mesh-basics)) or a few (rounded).
- **Round a corner off**, like a rounded box or a worn edge: more segments, Profile at 100%.
- **Carve a hollow curve** along an edge, like a cove moulding on furniture or a scooped edge on a stone block: more segments, Profile at -100%. The bevel curves inward instead of bulging out.
- **Cut a corner off** with a single flat face: bevel a point in Points mode.

To push polygons out or make frames, use [Extrude and inset](extrude-and-inset.md) instead.

## How to use it

### Bevel edges

1. Select a shape and press **2** for the [Edges mode](modes.md).
2. [Select](selecting.md) the edges to bevel. Double-click selects a whole loop, and **Ctrl + A** selects every edge.
3. Press **V** (or pick **Bevel** from the [tool menu](tool-menu.md) **E**, or the **Mesh** menu).
4. Drag **away from the selected edges** anywhere in the viewport: the further you pull away from the nearest one, the wider the bevel. Pull back towards them to narrow it. A label by the mouse says how wide. This follows the app's rule that [drags follow the pull](interface-tour.md#drags-follow-the-pull).
5. Let go. The whole drag is one undo step.

### Bevel points (cut a corner off)

1. Press **1** for the Points mode and select the corner points.
2. Press **V** and drag away from them. Each point is replaced by a flat polygon cutting the corner off, as far along each edge as the width.

### Segments and profile

Before you drag, the top of the **Attributes panel** shows the tool's **Segments** and **Profile** for the next bevel. After a bevel, the same settings and its **Width** show under **Last edit**, and changing them reshapes the bevel you just made. This works until you make another edit on top.

**Ctrl + mouse wheel** changes Segments without leaving the viewport: up for more, down for fewer. It does what the arrows beside Segments do. Before a drag it sets the next bevel. While dragging it changes the bevel you're making. Afterwards it reshapes the bevel you just made, one undo step per notch.

### While you drag

- Hold **Shift** for ten times finer steps.
- Press **Esc** or **right-click** to cancel.
- Let go where you started, or closer to the edges than where you started, and nothing is recorded.
- A **click** without dragging still selects.

### Rounded or hollow

With 2 or more segments, the **Profile** decides which way the bevel curves. At **100%** it bulges out, like a worn edge. At **0%** the rows lie flat, like a wide chamfer. Below 0 it curves the other way, into the shape: at **-100%** each edge is scooped out along a quarter circle, a *cove*, the kind of hollow edge you see on furniture mouldings and carved stone. You can change it on the bevel you just made, under **Last edit**, and watch the edge go from round through flat to hollow.

![The same cube with its edges bevelled at Profile -100%: each edge is scooped inward, so the top overhangs a hollow groove.](../screenshots/2026-10-13-bevel-hollow.png)

## Settings

| Setting | What it does |
|---|---|
| **Width** | How far the bevel reaches into each polygon next to the edge, in the object's own size. A typical bevel on a game prop is a few millimetres to a couple of centimetres. |
| **Segments** | How many rows the bevel has (**Ctrl + wheel** in the viewport changes it too). **1** is a flat cut (a chamfer), cheap and very common in game art. **2–4** make a rounded edge. More look smoother but cost polygons. |
| **Profile** | The shape across the bevel, from **100%** (round, bulging out like a worn edge) through **0%** (flat) to **-100%** (hollow, curving inward like a [cove](concepts.md#mesh-basics)). Values in between are gentler curves. It only shows with 2 or more segments. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Bevel | `V` |
| Set the width | Left drag away from the selected edges or points |
| Finer steps | Hold `Shift` while dragging |
| More or fewer segments | `Ctrl` + wheel up / down |
| Cancel the drag | `Esc` or a right-click |
| Back to selecting | `Q` |

## Tips and common problems

- **Tip:** while Bevel is in use, Ctrl + wheel changes Segments instead of growing the selection. To grow or shrink the selection, use the **Select** menu, or press **Q** first.
- **Tip:** a single-segment chamfer on every hard edge of a prop is the classic look of mid-poly game art: it shades nicely and costs few polygons.
- **Tip:** the cube's hard edges carry over. A chamfered cube stays crisp; a rounded bevel's inner rows are smooth so it looks round.
- **Where several bevelled edges meet** (a cube's corner), a corner polygon closes the gap. With several segments it is one polygon for now; a properly rounded corner comes with the improved bevel in Phase 7.
- **Problem:** pressing V does nothing but the status bar talks about modes. **Why:** Bevel works in the Edges and Points modes. **Fix:** press **2** (or **1**) and try again.
- **Problem:** the drag does nothing. **Why:** you started away from the edges and pulled towards them, or nothing is selected. **Fix:** start near the edges and pull away.
- **Problem:** an edge on a plane's border wasn't bevelled. **Why:** a bevel needs a polygon on each side of the edge. Border edges are skipped.
- **Problem:** a wide bevel folds over itself. **Why:** it reached further than the polygons next to it are wide. **Fix:** lower the Width in the Attributes panel. Keeping bevels from overlapping automatically comes with the improved bevel.

## Related

- [Extrude and inset](extrude-and-inset.md): the other tools for building shape.
- [Selecting](selecting.md): loops and rings for picking edges quickly.
- [Concepts](concepts.md): [bevel and chamfer](concepts.md#mesh-basics), [hard edges and normals](concepts.md#shading-and-normals).
