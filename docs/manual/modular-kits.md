# Modular kit helpers

> Size the pieces of a modular kit to whole steps of a grid, make new shapes at that size, snap them onto it, and put their pivots where a game engine wants them, with one click each.

## When to use it

A [modular kit](concepts.md#the-game-asset-pipeline) is a set of pieces (walls, floors, corners, doorways, stairs) that a game places side by side, again and again, to build a level: like building a dungeon from tiles. The pieces only fit if two things are right:

- **Their sizes.** On a 4 m grid, a wall 3.9 m long leaves a 10 cm gap next to its neighbour, and one 4.1 m long pokes into it. Every piece's outside sizes (its length, width and height) should be whole steps of the **[kit grid](concepts.md#the-game-asset-pipeline)**: 1 m, 2 m and 4 m are the common choices. A wall's *thickness* can be anything: pieces don't line up along it.
- **Their [pivots](concepts.md#objects-and-scenes).** The engine places each piece by its pivot. A floor tile or a wall piece snaps cleanly with its pivot at a **bottom corner**; a crate or a pillar wants it at the **middle of its bottom**; a lamp hanging from a ceiling at the **middle of its top**.

These helpers do both:

- **Kit grid**: the step your kit is built on, saved with the project.
- **Size to kit grid**: rounds the selected shapes' sizes to whole steps of it.
- **Kit-sized shapes**: new shapes are made at the kit grid's size.
- **Snap moves to the kit grid**: moving snaps onto it, so pieces sit edge to edge.
- **Pivot presets**: the pivot jumps to the bottom centre, a bottom corner, the centre or the top centre of the shape, which stays where it is.

When the kit is ready, **File → Export kit…** writes each piece as its own file, with its pivot at the file's origin and Godot collision if you want it: see [Kit export and collision](kit-export.md).

For a pivot anywhere else (a door's hinge, say), move it by hand with **Edit pivot**: see [Pivots and Edit pivot](pivots.md).

## How to use it

### Choose the kit grid

1. Open **Create → Modular kit**.
2. Under **Kit grid**, pick **1 m**, **2 m** or **4 m**, or type any other step in the **Other** field (in the project's [units](concepts.md#objects-and-scenes)). The list stays open while you change it.

The kit grid is saved with the project; each change is an undo step (*Kit grid 2 m*). It's 1 m until you change it.

### Size shapes to the grid

1. In the Model mode, **select the shapes** to size (a cube, a cylinder, stairs and so on: shapes that still have their settings).
2. Choose **Create → Modular kit → Size to kit grid** (or press **Space** and type *kit*).

Each shape's outside sizes are rounded to the **nearest whole number of steps**, at least one:

- A 3.9 m × 20 cm × 3.1 m wall becomes 4 m × 20 cm × 3 m on a 1 m grid. The 20 cm thickness stays: **a size shorter than half a step is left alone**, so walls and slabs stay thin.
- **Round shapes** (cylinders, spheres, discs, cones…) are rounded by how far **across** they are: a cylinder 1.6 m across becomes 2 m across.
- Parts **inside** the outline keep their proportion: a tube's hole, a gear's teeth and axle hole.
- An **arch** gets its outside width and its height to the top rounded; its legs take up the difference.
- Counts (segments, sides, steps) don't change, and edits recorded on the shape replay on the new size.

All the selected shapes change in **one undo step**: **Ctrl + Z** puts their old sizes back. The status bar says how many were sized. Shapes already the right size are left alone.

### Make new shapes at the kit's size

Tick **Create → Modular kit → Kit-sized shapes**. From then on, every new shape (from the Create menu, the **B** and **Alt + B** lists, the palette's shape buttons or command search) is made at the kit grid's size instead of 1 m, and fitted to it. On a 2 m grid a new cube is 2 m, a cylinder 2 m across and 2 m tall, and a gear 2 m across (and still a thin slab).

The switch is saved with the project, so it's on for a kit and off for everything else. Untick it to go back to 1 m shapes.

### Snap pieces onto the grid

Tick **Create → Modular kit → Snap moves to the kit grid** (also in **Edit → Settings… → Viewport → Snapping**). With [snapping](transforming.md#snapping) on (the magnet button, or **Ctrl** held while dragging), a move that doesn't find a point, edge or polygon under the mouse lands on **whole steps of the kit grid** instead of the grid lines shown. So a 4 m wall with its pivot at its bottom corner, dragged with snapping, lands exactly where the next 4 m tile starts.

The viewport still draws its own grid (1 m, 50 cm or 10 cm, as the grid button is set); with a 1 m grid shown and a 2 m kit grid, pieces snap to every other line.

### Put the pivot where the engine wants it

1. **Select the objects** (any mode).
2. Choose **Tools → Pivot presets** and one of:

| Preset | Where the pivot goes | Good for |
|---|---|---|
| **Pivot to bottom centre** | The middle of the shape's bottom (where every new shape has it) | Props that stand on the floor: crates, pillars, barrels |
| **Pivot to bottom corner** | The shape's bottom front left corner: lowest along its own X and Y (front left in the Front view, F4) | Floor tiles and wall pieces that snap on a grid |
| **Pivot to centre** | The middle of the shape's box | Things that turn round their middle: wheels, crystals |
| **Pivot to top centre** | The middle of the shape's top | Things that hang: lamps, chains, ceiling pieces |

With **Edit pivot** on, they're also in the **E** [tool menu](tool-menu.md): the **Pivot presets** place (the last one, at the top left) opens the four round it.

The shape doesn't move, and neither do objects inside it: only the pivot does. The corners and middles are those of the shape's own box, along its own axes, so on a turned piece the corner turns with it. For a group, it's the box round everything in it. Each is one undo step.

## Settings

| Setting | What it does |
|---|---|
| **Kit grid** | The step the kit is built on: 1 m (standard), 2 m, 4 m or any other from 1 cm to 100 m. Saved with the project. **Create → Modular kit**. |
| **Kit-sized shapes** | Off (standard): new shapes are 1 m. On: new shapes are made at the kit grid's size, their outside sizes whole steps of it. Saved with the project. |
| **Snap moves to the kit grid** | Off (standard): grid snapping uses the grid lines shown. On: it uses the kit grid. Remembered between sessions, like the other snapping settings. **Create → Modular kit**, or **Settings → Viewport → Snapping**. |

## Shortcuts

None of these have a key of their own.

| Action | Shortcut |
|---|---|
| Any of them, by name | `Space`, then type (*kit*, *bottom corner*…) |
| Pivot presets, with Edit pivot on | `E`, then the **Pivot presets** place |
| Snap for this drag (or not, if snapping is on) | Hold `Ctrl` while dragging |

## Tips and common problems

- **Problem:** Size to kit grid is greyed out. **Why:** nothing is selected, the selection has no shapes with settings, or they already fit the grid. Rest the mouse on it to see which. **Fix:** select the shapes; for a baked mesh, see the next problem.
- **Problem:** a baked mesh (after **Bake into shape**) won't size to the grid. **Why:** it has no sizes to round, only points; scaling it would stretch its bevels and holes too. **Fix:** size shapes to the grid before baking them, or undo the bake; or scale the mesh by hand with snapping.
- **Problem:** my wall stayed 20 cm thick. **Why:** sizes shorter than half a step are thicknesses, not sizes the pieces line up along, so they're left alone. **Fix:** nothing to fix; type a thickness in the Attributes panel if you want another.
- **Problem:** a 1.5 m piece became 2 m, not 1 m. **Why:** half a step rounds up. **Fix:** type the size you want in the Attributes panel.
- **Problem:** pieces don't snap to the kit grid. **Why:** snapping is off, *Snap moves to the kit grid* is off, or the mouse was over a point, edge or polygon, which snapping prefers. **Fix:** turn snapping on (or hold Ctrl), tick the setting, and drag away from other shapes; or switch off Points, Edges and Polygons under **Settings → Viewport → Snapping → Moves snap to**.
- **Problem:** the bottom corner isn't the corner I wanted. **Why:** it's always the one lowest along the shape's own X and Y. **Fix:** switch on **Edit pivot**, turn snapping on, and drag the gizmo's middle onto the corner you want.
- **Tip:** for a whole kit, set the grid first, tick **Kit-sized shapes**, and build every piece from shapes made at that size; size older pieces with **Size to kit grid**; give each a pivot preset; then **File → Export kit…**.
- **Tip:** a project opened from before this version has a 1 m kit grid and Kit-sized shapes off.

## Related

- [Kit export and collision](kit-export.md): each piece as its own file, with Godot collision
- [Pivots and Edit pivot](pivots.md): moving a pivot by hand, and the other ready-made places
- [Move, rotate and scale: snapping](transforming.md#snapping)
- [Primitives](primitives.md): the shapes and their settings
- [Concepts: kit grid, modular kit, pivot](concepts.md#the-game-asset-pipeline)
