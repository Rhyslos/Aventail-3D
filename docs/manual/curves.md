# Curves: draw a line, then sweep or lathe it into a solid

> Draw a smooth or sharp line through the scene, or start from a ready-made line, circle, arc or spiral, and turn it into a pipe, a cable, a rail or a vase. The line stays editable: move, add, join, split and round its points and the solid follows.

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

### Carrying a curve on from an end

To make an open curve longer, select it, pick **Draw curve**, and **click one of its ends first**: the ends of the selected curves show as dots before you start, and the ring under the mouse snaps onto an end when you're over it. Then place points as usual. They join that curve: after its last point, or, if you started from its first point, in front of it. **Clicking the curve's other end** closes it into a loop. Enter or a right click finishes, Esc cancels (the curve stays as it was), Backspace takes back the last new point. One **Ctrl + Z** takes the whole carrying on back.

Why it's handy: you can draw a path in pieces, look at it from another view, then carry on, without joining curves afterwards. A curve stays selected after you draw it, so starting a *new* curve exactly on its end carries it on instead; click on empty space to deselect it first if you want a separate curve there.

**Where the points go.** In the perspective view, points go on the **ground** (the grid). In a flat view (top, front or right, **F2**–**F4**), they go on the plane facing you through the middle of the world, where that view's grid is: draw a profile standing up in the front view, or a path on the ground in the top view. All of a curve's points go on the plane its first point was placed on. With **[snapping](transforming.md#snapping)** on (the Snap button; hold Ctrl to flip it), a point snaps to a point, edge or polygon under the mouse, or else to the grid lines; the handles you drag out follow the mouse freely.

### Ready-made curves

**Create → Line, Circle, Arc** or **Spiral** (also in the Alt + B list and command search) makes a curve from settings, lying on the ground round its pivot, like a [primitive](primitives.md). Change its size in the Attributes panel. As soon as you edit its points (press 1 and move one), it becomes an ordinary curve of points and its settings are gone; Ctrl + Z brings them back.

### Making it a solid

Select the curve. In the [Attributes panel](attributes.md), under **Curve**, set **Makes**:

- **Line only:** the standard, just the line.
- **Sweep:** a cross-section carried along the curve. Its settings appear under **Sweep**.
- **Lathe:** the curve spun round an axis. Its settings appear under **Lathe**.

[Command search](command-search.md) (**Space**) does the same: type *sweep* or *lathe* and pick **Sweep the curve** or **Lathe the curve** to make every selected curve a sweep or a lathe (a curve already making one keeps its settings). They're greyed out until a curve is selected.

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
4. **Double-click a point** (or one of its handle ends) to switch it between **sharp** (a corner, no handles) and **smooth** (handles in line, pointing from one neighbour towards the other). A broken or auto point becomes sharp.
5. **Double-click the curve's line**, between two points, to **add a point** there. The curve keeps exactly the same shape: the stretch is split in two, and the handles either side are shortened to match. On a straight stretch the new point is a sharp corner on the line; on a bend it's a smooth point. Drag it to reshape the curve.
6. **Delete** (or Backspace) deletes the selected points. The points either side keep their handles pointing the same way, stretched across the gap, so the curve keeps its shape as near as it can (deleting a point you've just added gives back the curve you had). Deleting a selected handle end takes that handle back to nothing. A curve keeps at least two points, and a closed curve at least three.
7. **Closed** in the Attributes panel closes an open curve into a loop or opens a closed one (with three points or more).

With a curve selected in the Points mode, the Attributes panel shows a **Curve points** section (how many points are selected, their handle type, Round corners and Even spacing), and the **E** menu shows the curve tools: Handle type, Round corners, Even spacing, Reverse curve, Open or close curve, Delete, Weld, Connect and Split curve. The same tools are in **Mesh → Curve** and command search (**Space**); each is greyed out, with the reason on hover, when it can't be used.

### Handle types

Each point has a **[handle type](concepts.md#curves)**, which decides how its two handles behave when you move them:

| Type | Handles | Use it for |
|---|---|---|
| **Sharp** | None | Corners: the curve turns sharply there. |
| **Smooth** | Always in one line: drag one and the other swings round to match, keeping its own length | Smooth bends you shape by hand. |
| **Broken** | Each moves on its own | A kink with curved sides: the top of a heart, a leaf's tip, a scroll. |
| **Auto** | Worked out by the app: along the line between the neighbouring points, a third of the way to each | Smooth curves that stay smooth while you move the points: you never touch the handles. Pulling an auto handle by hand makes the point smooth, so your pull stays. |

To set it, select points (press 1, then click or box them) and click **Sharp**, **Smooth**, **Broken** or **Auto** under **Handle type** in the Attributes panel (the type all the selected points share is lit), or open the E menu's **Handle type**, or use **Mesh → Curve** or command search. A sharp point made smooth or broken gets handles to pull, along its neighbours.

### Joining curves and closing them

- **Two ends of one curve:** select both end points (press 1, Ctrl + click the second) and press **M** (Weld) to melt them into one point at their middle, or **A** (Connect) to join them with a straight stretch. Either way the curve becomes a loop. Welding needs four points or more, so that three are left.
- **Ends of two curves:** select both curves, press 1, and select one end point of each. **M** melts them into one point at their middle; **A** joins them with a straight stretch. The curve you selected first stays, with its sweep or lathe settings; the other becomes part of it and is gone as an object.
- **Whole curves:** select two or more open curves in the Model mode and choose **Join objects** (Mesh → Shape and object, the E menu or command search). They join into the one you selected first, each at its end nearest the curve so far. Ends on the same spot melt into one point; ends apart are joined by a straight stretch. If the joined curve's two ends then meet, it closes. A closed curve has no ends, so it can't be joined: open it or split it first.

M and A with anything else selected on a curve (a middle point, three points) say in the status bar what they need. Every join is one Ctrl + Z.

### Splitting, reversing, opening and closing

- **Split curve** (Points mode): select one or more points inside the curve and choose it. The curve is cut at each, into separate curve objects with the same sweep or lathe settings, and all the pieces are selected. A closed curve cut at one point opens there. An open curve's end points cut nothing.
- **Reverse curve** turns the selected curves round, so they run from their last point to their first. It matters for a sweep's twist and the direction a profile faces, and for carrying a curve on. Reversing twice gives the same curve.
- **Open or close curve** closes an open curve with a straight stretch from its last point to its first, or opens a closed one.

Reverse, Open or close and Even spacing work on whole curves, in the Model mode as well as the Points mode.

### Rounding corners

**Round corners** turns sharp corners into smooth arcs, a **[fillet](concepts.md#curves)**: select sharp points (in the Points mode), set the **radius** under **Round corners** in the Attributes panel, and click **Round** (or choose Round corners from the E menu, Mesh → Curve or command search). Each corner becomes two smooth points joined by an arc of that radius, touching the straight legs either side, so a swept pipe bends round instead of mitring.

The radius is **cut back to what fits**: an arc can take all of a leg, or half of it where the corner at its other end is rounded too, so neighbouring arcs never overlap. Only sharp points inside the curve with a real corner are rounded: ends of an open curve, points with handles (they're round already) and points in a straight line are left alone. Ctrl + Z takes it back; change the radius and round again.

### Spacing points evenly

**Even spacing** puts the curve's points at equal distances along it (**[resample](concepts.md#curves)**), keeping its shape as near as it can. Set how many under **Even spacing** in the Attributes panel (**Points along**; until you change it, as many as the curve has now), then click **Space evenly**, or choose Even spacing from the E menu, Mesh → Curve or command search. Each new point gets handles along the curve, so bends stay smooth; on a straight stretch the points are sharp, so straight lines stay straight. Corners are rounded off slightly unless a point lands on them, so space a curve with sharp corners before rounding its corners.

Why: points bunched in one place and sparse in another make a sweep's detail uneven, and a lathe's outline lumpy. Even spacing also turns a few points into many (for finer control) or many into few.

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
| **Handle type** (Curve points) | Sharp, Smooth, Broken or Auto, for the selected points. |
| **Round corners** (Curve points) | The radius of the arcs Round corners makes (10 cm to start with), cut back to what fits. |
| **Even spacing** (Curve points) | How many points Space evenly puts along the curve: as many as it has, until you change it. |
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
| Add a point, keeping the shape | Double-click the curve's line (Points mode) |
| Delete selected points | `Delete` (or `Backspace`) |
| Weld two end points at their middle | Select them, `M` (Points mode) |
| Join two end points with a straight stretch | Select them, `A` (Points mode) |
| Carry a curve on from an end | Select it, Draw curve, click the end first |
| The curve tools | `E` (Points mode, curves selected) |
| Bake the solid into a mesh | `Ctrl + E` |

Draw curve has no key of its own: it's in **Create**, the **Alt + B** list and **Space** (command search). Split curve, Reverse curve, Open or close curve, the handle types, Round corners and Even spacing have no keys either: they're in the E menu, **Mesh → Curve** and command search. The sweep's and lathe's counts, Round corners' radius and Even spacing's count are ordinary fields in the Attributes panel (Ctrl + wheel is for the count of a [tool in use](bevel.md); these are settings of the object or of a command run once).

## Tips and common problems

- **Problem:** nothing happens when I click to place a point. **Why:** the view looks straight along the curve's plane (for example, you started the curve in the top view and are clicking in the front view, edge-on to the ground). **Fix:** place the points in a view that faces the plane, or start a new curve in this view.
- **Problem:** the pipe looks flat, or is a strip with no inside. **Why:** a curve as the cross-section that isn't closed makes a strip. **Fix:** close the cross-section curve (Attributes → Closed, or click its first point when drawing).
- **Problem:** "Nothing to show: the whole curve lies on the axis it spins round". **Why:** a lathe spins the curve round its own axis, through its pivot; a curve drawn straight along that axis has nothing to spin. **Fix:** draw the outline beside the axis (half of the shape's silhouette), or pick another Axis.
- **Problem:** the lathe comes out the wrong size or off to one side. **Why:** it spins round the curve's pivot, which is in the middle of its points. **Fix:** move the pivot onto the axis you want with **Edit pivot**, or draw the outline from the axis.
- **Problem:** the sweep goes the wrong way round, or the trim is upside down. **Why:** the cross-section is placed as you see it in the view you drew it in. **Fix:** flip or turn the cross-section curve's points (in the Points mode, select all and scale by −1 or rotate), or use **Twist** 180°.
- **Problem:** a sharp corner looks pinched. **Why:** a corner turning almost straight back can only be stretched so far (four times). **Fix:** make the point smooth (double-click it), or round it (Round corners).
- **Problem:** Round corners is greyed out. **Why:** none of the selected points is a sharp corner inside the curve. **Fix:** select corners that have no handles (Handle type → Sharp first), not the ends of an open curve.
- **Problem:** M or A says it joins end points. **Why:** on a curve, Weld and Connect join two ends of open curves, nothing else. **Fix:** select exactly the two end points (on one curve to close it, or one on each of two curves).
- **Problem:** clicking with Draw curve carried an existing curve on instead of starting a new one. **Why:** you clicked an end of the selected curve. **Fix:** Esc, deselect the curve (click empty space), and draw again.
- **Problem:** a curve can't be joined ("a closed curve has no ends to join"). **Fix:** open it (Open or close curve) or split it at a point first.
- **Tip:** for a smooth path that follows your points without fiddling with handles, select all its points and set Handle type → Auto.
- **Tip:** use the same cross-section curve for several paths (all the skirting in a room): change it once and they all follow.
- **Tip:** a closed curve swept with a circle is a quick ring; give it a rectangle cross-section and Twist 360° for a twisted band.

## Related

- [Concepts: curves, Bézier handles, sweep, lathe, profile, handle types, fillet, resample](concepts.md#curves)
- [Connect, Bridge and Weld](connect-bridge-weld.md) on meshes, and [Join objects](objects.md)
- [Primitives](primitives.md) and [the shape lists](primitives.md#how-to-use-it)
- [Selecting](selecting.md) and [Move, rotate and scale](transforming.md)
- [The Attributes panel](attributes.md)
- [Bake into shape](objects.md#bake-into-shape)
- [Booleans](booleans.md): a swept or lathed curve can cut or be cut
- [Export and import](export-and-import.md)
- Why it works this way: [decision 0138](../decisions/0138-curves-sweep-lathe.md) and [decision 0139](../decisions/0139-spline-editing.md)
