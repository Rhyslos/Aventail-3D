# Extrude and inset

> Extrude pushes polygons out (or in) and builds new sides behind them; Inset shrinks polygons inward and leaves a ring of new polygons around them.

These two are the bread and butter of hard-surface modelling. Almost every panel, button, vent and handle on a game prop starts with them:

- **Extrude** grows the shape: pull a polygon out of a cube and you get a block sticking out, with four new [polygons](concepts.md#mesh-basics) as its walls.
- **Inset** adds detail without changing the outline: the polygon gets smaller where it is, flat on the surface, and a ring of new polygons fills the gap. It's how you draw a frame or a border onto a surface before pushing something in or out.

**Inset, then extrude** is the classic pair. Inset a cube's top a little, then extrude it inward: you have a recessed panel with a crisp rim.

![A cube's front polygon inset, then extruded outward. The Attributes panel shows the Last edit, with its Distance and Keep together switch, and the distance is written by the mouse.](../screenshots/2026-10-13-extrude.png)

## When to use it

- **Pull something out of a shape**, like a handle, a vent box or a pipe stub: extrude its polygons outward.
- **Push something in**, like a recess, a screen or a slot: extrude inward (drag the other way).
- **Make a frame or a lip** around a polygon before extruding it: inset first.
- **Grow a flat sheet at its edge**, like extending a plane: extrude its [border edges](concepts.md#mesh-basics) in Edges mode.

To move polygons without building new ones, use [Move](transforming.md) instead.

## How to use it

### Extrude polygons

1. Select a shape and press **3** for the [Polygons mode](modes.md).
2. [Select](selecting.md) the polygons to extrude.
3. Press **D** (or pick **Extrude** from the [tool menu](tool-menu.md) **E**, or the **Mesh** menu).
4. Drag left or right **anywhere in the viewport**. You don't have to aim at the polygons: right pushes them out, left pushes them in. A label by the mouse says how far.
5. Let go. The whole drag is one undo step.

The polygons stay selected, so you can drag again straight away to extrude another step. Each drag is its own extrude.

### Inset polygons

1. In the Polygons mode, select the polygons.
2. Press **I** (or pick **Inset** from the tool menu or the Mesh menu).
3. Drag right anywhere in the viewport to shrink them inward. Dragging back to the start leaves them as they were; it never grows them outward.

### Extrude border edges

1. Press **2** for the Edges mode and select edges on an **open border**: edges with a polygon on one side only, like the rim of a [plane](primitives.md).
2. Press **D** and drag right. A new strip of polygons grows outward from each edge, flat with the polygon it belongs to.
3. The selection moves to the new outer edges, so you can drag again to keep growing.

Edges with polygons on both sides have nothing to grow from and are left alone; the status bar doesn't count them.

### Change it afterwards

Right after an extrude or inset, the **Attributes panel** shows it at the top, under **Last edit**:

- **Distance**: type an exact value, drag the number, or use the arrows. The shape follows as you change it.
- **Keep together**: switch it to change how neighbouring polygons were treated (see Settings below).

This works until you make another edit on top. After that, the extrude is fixed in place, like any other edit.

### While you drag

- Hold **Shift** for ten times finer steps.
- Press **Esc** or **right-click** to cancel: everything goes back as it was.
- Let go where you started and nothing is recorded.
- A **click** without dragging still selects, so you can pick other polygons without leaving the tool. Press **Q** to go back to selecting with the box, lasso or brush.

## Settings

| Setting | What it does |
|---|---|
| **Keep together** (Attributes panel) | **On** (the standard): polygons that touch move as **one piece**, like a slab pulled out of a wall; the edges between them stay and no walls form between them. **Off**: **each polygon on its own**, so every one becomes a separate bump (extrude) or gets its own ring (inset). The switch shows at the top of the Attributes panel while Extrude or Inset is the tool in use, and on the last edit; changing it on the last edit also sets it for the next one. For edges, on means edges that meet at a point grow as one strip. |
| **Distance** (Attributes panel, Last edit) | How far the polygons were pushed or shrunk, in the object's own size. Negative extrudes push inward. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Extrude | `D` |
| Inset | `I` |
| Set the distance | Left drag anywhere in the viewport |
| Finer steps | Hold `Shift` while dragging |
| Cancel the drag | `Esc` or a right-click |
| Back to selecting | `Q` |

## Tips and common problems

- **Tip: inset, then extrude inward** makes a recessed panel. **Inset, then extrude outward** makes a raised plate with a border round it.
- **Tip:** to push several separate panels in at once, select them all with **Keep together** off; with it on, touching polygons go in as one big panel.
- **Tip:** the cube's hard edges carry over: an extruded block on a cube has crisp edges, and one on a sphere stays smooth. (Hard and soft edges get their own tools later in Phase 4.)
- **Problem:** pressing D or I does nothing but the status bar talks about modes. **Why:** Extrude works in the Polygons and Edges modes, Inset only in Polygons. **Fix:** press **3** (or **2** for edges) and try again.
- **Problem:** the drag does nothing. **Why:** nothing of the right kind is selected; the status bar says so. **Fix:** select polygons (or border edges) first.
- **Problem:** extruding edges does nothing. **Why:** the edges aren't on an open border; a cube has none. **Fix:** use a plane, or extrude polygons instead.
- **Problem:** an inset crossed over itself and looks folded. **Why:** it went further than the polygon is wide. **Fix:** undo, or lower the Distance in the Attributes panel.
- **Problem:** after changing the cube's segments, the extrude went somewhere odd or the Attributes panel says an edit no longer fits. **Why:** edits find their polygons again by where they were ([Attributes: Edits](attributes.md)); a big change can leave nothing at that spot. **Fix:** go back to the earlier settings, or undo.
- **Problem:** the extrude is too fast or too slow to control. **Why:** the drag follows the screen, so it is faster when zoomed out. **Fix:** zoom in, hold Shift, or type the Distance.

## Related

- [Selecting](selecting.md): picking the polygons and edges to work on.
- [Move, rotate and scale](transforming.md): moving extruded polygons further.
- [The tool menu](tool-menu.md): Extrude and Inset are in the top-right group.
- [Concepts](concepts.md): [polygons, borders and manifold shapes](concepts.md#mesh-basics), [extrude and inset](concepts.md#mesh-basics).
