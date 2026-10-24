# Curves: draw a line, then sweep or lathe it into a solid

> Draw a smooth or sharp line through the scene, or start from a ready-made line, circle, arc or spiral, and turn it into a pipe, a cable, a rail or a vase. The line stays editable: move its points and the solid follows.

A **[curve](concepts.md#curves)** is a line through space, defined by a few points. Between two points it bends the way the points' **[Bézier handles](concepts.md#curves)** pull it: a point with no handles is a sharp corner, a point with handles is a smooth bend. On its own a curve is just a line (it guides, or it's a cross-section for another curve). Give it a **[sweep](concepts.md#curves)** and a shape (its **[profile](concepts.md#curves)**: a circle, a square, a rectangle, or another curve) is carried all along it, like squeezing toothpaste along a path: pipes, cables, hoses, railings, trims and mouldings. Give it a **[lathe](concepts.md#curves)** instead and it's spun round an axis, like clay on a potter's wheel: vases, bottles, columns, bowls, wheels.

The sweep or lathe is **live**: it's a setting of the curve, worked out again whenever the curve or a setting changes. Drag a point and the pipe bends with it. When you're done, **Bake into shape** turns it into an ordinary mesh you can model point by point.

## When to use it

- **Sweep:** anything long with the same cross-section all the way: pipes and cables (a circle), bars and beams (a square or rectangle), rails, skirting, picture frames, gutters and decorative trims (another curve as the cross-section). A closed curve makes a ring (a tyre, a hoop, a gasket).
- **Lathe:** anything round, the same all the way round an axis: vases, bottles, cups, lamp shades, table legs, columns, knobs, wheels and rims. Draw half the outline, from the axis outward, and spin it.
- **Line only:** a guide, or the cross-section for another curve's sweep.

For a short pipe with one bend, a [Pipe elbow](primitives.md) is quicker, and an [extrude with a bend](extrude-and-inset.md) bends a pipe out of an existing shape. For one ring of a fixed shape, a [Torus](primitives.md) or [Tube](primitives.md) may be enough.

## How to use it

### Drawing a curve

1. Pick **Draw curve**: from **Create**, from the **Alt + B** list (at the end, with the other curves), or press **Space** and type *draw curve*. The app switches to the Model mode if you were editing points, edges or polygons: curves are objects.
2. **Click** in the viewport to place a point: a sharp corner.
3. **Press and drag** to place a point with handles: the drag pulls the handles out, for a smooth bend. The further you drag, the wider the bend; the direction you drag is the way the curve leaves the point.
4. Keep placing points. While you draw, the curve so far, its points and handles, and the stretch the next click would add are drawn over the view.
5. Finish it:
   - **Enter**, or **right-click** (a click, not a drag), keeps it as an open curve;
   - **clicking the first point** again (once there are three points) closes it into a loop and finishes it;
   - picking another tool or mode keeps what you've drawn too.

   **Esc** cancels, and **Backspace** takes back the last point you placed.

The finished curve is one new object called *Curve*, selected, with its pivot in the middle of its points. One **Ctrl + Z** takes the whole curve back. A curve needs at least two points.

**Where the points go.** In the perspective view, points go on the **ground** (the grid). In a flat view (top, front or right, **F2**–**F4**), they go on the plane facing you through the middle of the world, where that view's grid is: draw a profile standing up in the front view, or a path on the ground in the top view. All of a curve's points go on the plane its first point was placed on. With **[snapping](transforming.md#snapping)** on (the Snap button; hold Ctrl to flip it), a point snaps to a point, edge or polygon under the mouse, or else to the grid lines; the handles you drag out follow the mouse freely.

### Ready-made curves

**Create → Line, Circle, Arc** or **Spiral** (also in the Alt + B list and command search) makes a curve from settings, lying on the ground round its pivot, like a [primitive](primitives.md). Change its size in the Attributes panel. As soon as you edit its points (press 1 and move one), it becomes an ordinary curve of points and its settings are gone; Ctrl + Z brings them back.

### Making it a solid

Select the curve. In the [Attributes panel](attributes.md), under **Curve**, set **Makes**:

- **Line only:** the standard, just the line.
- **Sweep:** a cross-section carried along the curve. Its settings appear under **Sweep**.
- **Lathe:** the curve spun round an axis. Its settings appear under **Lathe**.

**A sweep's cross-section** is a **Circle** (pipes: set Width, how wide across, and Sides), a **Square** or a **Rectangle** (Width across the curve and Height up from it), or **another curve** from the list (each curve object in the scene is there by name). For a curve as the cross-section:

1. Draw the cross-section in a **flat view**: the front view (F4) is the usual choice. Draw it round the spot where the path should run through it, and close it for a solid trim (or leave it open for a thin strip, such as a gutter).
2. Draw or make the path.
3. Select the path, set Makes to Sweep and pick the cross-section curve in **Cross-section**.

The cross-section is laid flat on the plane it was drawn in and placed round its pivot (the middle of its points): what you saw in the front view, right is across the path and up is up. Move the cross-section's points and the sweep follows. Delete the cross-section curve and the sweep goes back to a circle (Ctrl + Z undoes both).

**A lathe** spins the curve round one of its own axes, through its pivot: **Z** (up) as standard. Draw half the outline in the front view, starting on the axis (the green line through the middle) and going out and up; put the curve's pivot on the axis (Edit pivot, or draw its first and last points on the axis). Points on the axis close the surface there, like the bottom of a bottle.

### Editing a curve's points

1. Select the curve and press **1** (the Points mode). Its points show as dots, and so do the ends of its handles, joined to their points by thin lines.
2. **Select** points and handle ends as you would a mesh's: click, drag a box or lasso, **Ctrl** adds, **Shift** removes, Ctrl + A selects all ([Selecting](selecting.md)).
3. **Move, rotate or scale** them with the gizmo (W, R, S) or the Coordinates band ([Move, rotate and scale](transforming.md)). Moving a point brings its handles along. Moving one handle of a smooth point turns the other one too, so the bend stays smooth.
4. **Double-click a point** to switch it between **sharp** (a corner, no handles) and **smooth** (handles in line, pointing from one neighbour towards the other).
5. **Delete** (or Backspace) deletes the selected points. Deleting a selected handle end takes that handle back to nothing, making that side of the point sharp. A curve keeps at least two points.
6. **Closed** in the Attributes panel closes an open curve into a loop or opens a closed one (with three points or more).

The Edges and Polygons modes have nothing to select on a curve: its solid is made from the curve. To model the solid itself, **Bake into shape** (Ctrl + E) first.

### Making it permanent

**Bake into shape** (**Ctrl + E**, or Mesh → Shape and object) turns a swept or lathed curve into a plain mesh, ready for the Points, Edges and Polygons modes; the curve is gone (Ctrl + Z brings it back). A curve that is only a line has nothing to bake.

### What the solid looks like

- **Quads**, in rows along the curve and round the cross-section (triangles where a lathe closes at its axis), facing outward. With caps, the ends are flat polygons, so the solid is a [closed solid](concepts.md#mesh-basics): ready to export, bake, or use in a [boolean](booleans.md) (a swept ring makes a good cutter for a groove round a cylinder).
- **The cross-section turns with the curve without flipping or spinning**, even round a spiral: it starts upright (its up as close to the world's up as the curve's first direction allows) and turns only as much as the curve makes it.
- **At a sharp corner** the cross-section is stretched across the corner (a mitre), so the pipe stays as wide on both sides of it, like a picture frame's corner.
- **[Hard edges](hard-and-soft-edges.md)** along a square's or rectangle's corners (and a cross-section curve's sharp points), round the caps, and round the curve's corners sharper than 30°.
- **[UVs](concepts.md#uvs-and-texturing)** laid out at their real sizes: the length of the solid as one piece (round the cross-section one way, along the curve the other), each cap a piece of its own, packed into the texture square like every primitive's, with [seams](seams.md) where they part. The whole solid uses the object's first [material](materials.md).
- **Saved and exported:** curves are saved with the project, live. [Export](export-and-import.md) sends out the solid; a curve that is only a line goes out as nothing.

## Settings

| Setting | What it does |
|---|---|
| **Length** (Line), **Radius** (Circle, Arc, Spiral), **Angle** (Arc), **Climb** and **Turns** (Spiral) | A ready-made curve's size and shape. |
| **Points** | How many points a drawn curve has. |
| **Closed** | Whether the last point joins the first, making a loop. |
| **Makes** | Line only, Sweep or Lathe. |
| **Cross-section** (Sweep) | Circle, Square, Rectangle, or another curve by name. |
| **Width**, **Height**, **Sides** (Sweep) | The built-in cross-section's size (Width is a circle's diameter) and how many sides make a circle. 12 sides is plenty for most pipes; 6 to 8 for thin cables seen from afar. |
| **Segments** | Pieces along the curve. With **Adaptive** on (the standard) it's pieces for each quarter turn the curve bends, and a straight stretch is one piece: detail only where it shows. With Adaptive off, it's pieces for every stretch between two points, evenly. 4 to 8 is usual for game assets. |
| **Twist** (Sweep) | How far the cross-section turns round the curve from start to end: a twisted rope, a drill bit. On a closed curve it's rounded to whole turns, so the ends meet. |
| **Caps** | Close an open curve's ends (a sweep), or the open ends and the sides of a part turn (a lathe), with flat faces. |
| **Axis** (Lathe) | Which of the curve's own axes it spins round, through its pivot. |
| **Spin angle** (Lathe) | How far round: 360° all the way; less for a slice. |
| **Segments round** (Lathe) | Pieces round the axis: 16 to 32 for most round things. |

## Shortcuts

| Action | Shortcut |
|---|---|
| A sharp point / a smooth bend (Draw curve) | Click / press and drag |
| Finish the curve | `Enter`, or right-click |
| Close the curve | Click its first point |
| Cancel the curve | `Esc` |
| Take back the last point | `Backspace` |
| Edit a curve's points | Select it, `1` |
| Switch a point between sharp and smooth | Double-click it (Points mode) |
| Delete selected points | `Delete` (or `Backspace`) |
| Bake the solid into a mesh | `Ctrl + E` |

Draw curve has no key of its own: it's in **Create**, the **Alt + B** list and **Space** (command search). The sweep's and lathe's counts are ordinary fields in the Attributes panel (Ctrl + wheel is for the count of a [tool in use](bevel.md), and these belong to the object).

## Tips and common problems

- **Problem:** nothing happens when I click to place a point. **Why:** the view looks straight along the curve's plane (for example, you started the curve in the top view and are clicking in the front view, edge-on to the ground). **Fix:** place the points in a view that faces the plane, or start a new curve in this view.
- **Problem:** the pipe looks flat, or is a strip with no inside. **Why:** a curve as the cross-section that isn't closed makes a strip. **Fix:** close the cross-section curve (Attributes → Closed, or click its first point when drawing).
- **Problem:** "Nothing to show: the whole curve lies on the axis it spins round". **Why:** a lathe spins the curve round its own axis, through its pivot; a curve drawn straight along that axis has nothing to spin. **Fix:** draw the outline beside the axis (half of the shape's silhouette), or pick another Axis.
- **Problem:** the lathe comes out the wrong size or off to one side. **Why:** it spins round the curve's pivot, which is in the middle of its points. **Fix:** move the pivot onto the axis you want with **Edit pivot**, or draw the outline from the axis.
- **Problem:** the sweep goes the wrong way round, or the trim is upside down. **Why:** the cross-section is placed as you see it in the view you drew it in. **Fix:** flip or turn the cross-section curve's points (in the Points mode, select all and scale by −1 or rotate), or use **Twist** 180°.
- **Problem:** a sharp corner looks pinched. **Why:** a corner turning almost straight back can only be stretched so far (four times). **Fix:** make the point smooth (double-click it) or add a point to round the corner.
- **Tip:** use the same cross-section curve for several paths (all the skirting in a room): change it once and they all follow.
- **Tip:** a closed curve swept with a circle is a quick ring; give it a rectangle cross-section and Twist 360° for a twisted band.

## Related

- [Concepts: curves, Bézier handles, sweep, lathe, profile](concepts.md#curves)
- [Primitives](primitives.md) and [the shape lists](primitives.md#how-to-use-it)
- [Selecting](selecting.md) and [Move, rotate and scale](transforming.md)
- [The Attributes panel](attributes.md)
- [Bake into shape](objects.md#bake-into-shape)
- [Booleans](booleans.md): a swept or lathed curve can cut or be cut
- [Export and import](export-and-import.md)
- Why it works this way: [decision 0138](../decisions/0138-curves-sweep-lathe.md)
