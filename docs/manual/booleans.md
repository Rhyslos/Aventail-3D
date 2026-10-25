# Booleans: Union, Subtract and Intersect

> Combine shapes like solid blocks: add one to another, cut one out of another, or keep only where they overlap. The shapes that do the cutting stay shapes of their own, so you can move them and the cut follows.

A [boolean](concepts.md#mesh-basics) treats shapes as solid objects, like pieces of clay, and works out a new surface from how they overlap. It's the quickest way to make things that would take many careful edits by hand: a round hole through a panel, a notch in a beam, a slot for a screw, a window in a wall, two parts merged into one casting.

The shape you keep is cut by one or more **[cutters](concepts.md#mesh-basics)**: other objects in your scene. In this app the cut is **live**: the cutters don't disappear when you use them. They're drawn as see-through outlines, and when you move, turn, scale or edit one (change a cylinder's radius, extrude it, anything), the cut updates straight away. When you're happy, **Bake into shape** makes the cut a permanent part of the shape.

## When to use it

- **Subtract:** holes, notches, slots, grooves, bevels cut by a box at an angle, a hollow inside. "Take this shape away from that one."
- **Union:** stick parts together into one solid, with the surface where they meet joined up: a handle on a crate, a boss on a panel. (To keep parts as separate pieces that just touch, don't use a boolean: group them, or use [Join](split-and-join.md).)
- **Intersect:** keep only where the shapes overlap, like cutting a shape out of a block with a cookie cutter, or rounding off a box by intersecting it with a sphere.

For a single thin line on a surface, [Groove](groove.md) is lighter; for a new loop of edges, [Loop cut](loop-cut.md). Booleans make new edges wherever the shapes cross, so they suit hard-surface parts better than shapes you'll keep deforming by hand.

## How to use it

1. **Make the cutter.** Create the shape that will do the cutting (for a round hole, a cylinder from **Create** or **B**), and place it where it should cut, going through or into the shape you'll keep. For a hole right through, make the cutter a little longer than the part is thick, so its ends stick out on both sides.
2. **Select the shape to keep first.** Click it, in the viewport or the Objects panel.
3. **Ctrl + click the cutter** (or several cutters, one after another). The order matters: the shape you selected **first** is the one kept; the others cut it. It doesn't matter where they are in the Objects panel.
4. Choose **Union**, **Subtract** or **Intersect**, from any of these:
   - **Mesh → Booleans**,
   - the three buttons on the **modifiers palette** (the thin column of buttons on the right of the viewport, under Add edit layer),
   - **Space** ([command search](command-search.md)), then type *subtract* (or *union*, *intersect*) and press **Enter**.

The kept shape is cut at once, and the cutters turn into outlines. The kept shape is selected, and the [Attributes panel](attributes.md) shows its **Booleans** section.

If the entries are greyed out, hover over them: they say what's missing (for example, only one shape is selected, or the shape selected first is a group).

### Changing the cut later

- **Move, turn or scale a cutter**, in the viewport with the gizmo or by typing in [Coordinates](transforming.md): the cut follows as you drag. Moving the kept shape moves it against the cutters too.
- **Edit a cutter:** change its settings in the Attributes panel (a cylinder's sides, a cube's size), or model it in the Points, Edges or Polygons modes. The cut follows.
- **Change what it does:** in the kept shape's Attributes, under **Booleans**, pick Union, Subtract or Intersect from the list.
- **Take a cutter out:** click **Take out** beside its name; it becomes an ordinary shape again. **Remove** takes the whole boolean away.
- **Add more cutters:** select the kept shape, Ctrl + click the new cutters, and choose the same command again. They join its last boolean if it's the same kind; a different kind adds a new boolean after it. Booleans are applied in the order they're listed.
- **Hide a cutter** (the eye in the Objects panel) to get its outline out of the way: hidden cutters still cut.
- **Delete a cutter** and its cut is gone; **Ctrl + Z** brings both back.

### Selecting a cutter

Cutters are only outlines, so a click on the cut shape picks the kept shape, not the cutter inside it. To pick a cutter, click its outline where nothing else is in the way (where it sticks out), click its name in the **Objects panel**, or click its name in the kept shape's **Booleans** section. With a cutter selected, its Attributes say which shapes it cuts.

### Making it permanent

Select the kept shape and choose **Bake into shape** (**Ctrl + E**, or **Mesh → Shape and object**). The cut becomes part of the shape, as a plain mesh you can edit point by point, and the boolean is gone. The cutters are **hidden, not deleted** (shown, they'd sit on top of the result as solid shapes again): you'll find them in the Objects panel, to use again or delete. One **Ctrl + Z** undoes all of it. **Apply mirror** on a shape with [symmetry](symmetry.md) bakes the cut too, as it bakes what you see.

### What the result looks like

Booleans are worked out on triangles, then put back together into proper polygons:

- A polygon the cut doesn't touch stays as it was. One the cut crosses keeps its material, and its [UVs](concepts.md#uvs-and-texturing) are worked out at the new corners, so the texture doesn't jump. A quad with a corner cut off becomes one n-gon.
- A face with a hole through it can't be one polygon (polygons here can't have holes), so the part around the hole is split into a few flat, convex polygons.
- The cutter's faces (the walls of a hole, say) come from the cutter, with its UVs, and its material if the kept shape has a material of the same name (otherwise the kept shape's first material).
- Hard edges stay hard. The new edges along the cut are made [hard](hard-and-soft-edges.md) where the surfaces meet more sharply than the kept shape's smoothing angle (30° as standard), and they're [seams](seams.md), because the UVs of the two shapes part there.
- The kept shape's own edits, and the Points, Edges and Polygons modes, still work on its uncut shape: what you select and edit is the shape before the cut, and the cut is redone on top of every change. Bake into shape first to edit the cut surface itself.

### Saving and exporting

Booleans are saved with the project, live, and come back exactly as they were. [Export](export-and-import.md) sends out the cut shape and leaves the cutters out, so a game engine or Substance Painter gets just the finished part.

## Settings

| Setting | What it does |
|---|---|
| **Kind** (Attributes → Booleans) | **Union** adds the cutters to the shape; **Subtract** cuts them out of it; **Intersect** keeps only where they overlap. |
| **Cutters** (listed under each boolean) | The shapes doing it, in order. Click a name to select that cutter; **Take out** stops it cutting. |
| **Remove** | Takes the whole boolean away; its cutters become ordinary shapes. |
| **Smoothing** (Attributes → Basic) | The kept shape's smoothing angle also decides which new edges along the cut are hard. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Add a cutter to the selection | `Ctrl + click` |
| Find Union, Subtract or Intersect | `Space`, then type the name |
| Make the cut permanent (Bake into shape) | `Ctrl + E` |
| Undo a boolean, or anything done to it | `Ctrl + Z` |

Union, Subtract and Intersect have no keys of their own.

## Tips and common problems

- **Problem:** the shape stays whole, and Attributes says it's "shown uncut" because a shape "isn't a closed solid". **Why:** a boolean needs to know what's inside and outside each shape, so every shape has to be a [closed solid](concepts.md#mesh-basics): no holes in its surface, and no edge shared by more than two polygons. A plane, a cylinder without caps, or a shape with a deleted polygon is open. **Fix:** close it: [Fill hole](clean-up.md) (J) closes holes; for a plane, extrude it into a slab. Attributes names the shape that's open.
- **Problem:** "a cutter is squashed flat". **Why:** it's scaled to 0 along some direction. **Fix:** give it some size (the Coordinates band, or its Scale in Attributes).
- **Problem:** a thin sliver or flicker where the cutter's face lies exactly on the kept shape's face. **Why:** faces lying exactly on top of each other are a hard case for every boolean. The app handles them, but the result may have extra edges. **Fix:** let the cutter stick out a little past the surface (a millimetre is plenty).
- **Problem:** the cut on a very heavy shape appears a moment after the change. **Why:** cuts of 5 000 polygons or more (the shapes together) are worked out on another thread so the window keeps working; the last cut is shown meanwhile. **Fix:** nothing to fix; for quicker cuts, keep cutters simple (a 16-sided cylinder is plenty for most holes).
- **Problem:** Union, Subtract and Intersect are greyed out with "the shape to keep already cuts one of the others". **Why:** that would be a loop: each shape waiting for the other to be worked out first. **Fix:** take the boolean away from one of them first.
- **Problem:** the wrong shape was kept. **Why:** the shape selected **first** is kept. **Fix:** Ctrl + Z, then select the shape to keep on its own (a plain click), and Ctrl + click the cutters.
- **Tip:** a cutter can cut several shapes at once: the same cylinder can drill through two plates. And a cutter can itself be cut by others.
- **Tip:** a swept or lathed [curve](curves.md) is a closed solid when its caps are on (or its curve is closed), so it can cut or be cut: a ring swept along a circle cuts a groove round a cylinder. A curve that is only a line has no inside, so it can't.

## Related

- [Concepts: booleans, cutters and closed solids](concepts.md#mesh-basics)
- [The Attributes panel](attributes.md) and [the Objects panel](objects.md)
- [Bake into shape](objects.md#bake-into-shape)
- [Hard and soft edges, and smoothing](hard-and-soft-edges.md)
- [Clean-up: Fill hole](clean-up.md)
- [Export and import](export-and-import.md)
- Why it works this way: [decision 0135](../decisions/0135-live-booleans.md)
