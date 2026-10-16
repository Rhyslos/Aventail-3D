# Symmetry

> Model one half: a live mirror shows the other half, copying every edit and every move, and the two join into one shape.

Most things are symmetrical: a crate, a gun, a car, a face. With **Symmetry** on, an object gets a **live mirror**: a copy of its shape on the other side of a mirror that stands in the middle of the world. You only ever work on your half. The mirrored half follows everything you do to it straight away, extrudes, bevels, cuts and moves alike, and where the halves meet at the mirror, they join into one surface.

The mirrored half is part of the object: it is drawn with it, counted in the Export window's numbers, and exported with it as one shape, ready for the game engine or Substance Painter.

![A cube moved to the right with Symmetry on: its mirrored twin stands on the left, the same distance from the faint red mirror in the middle of the world.](../screenshots/2026-10-16-live-mirror.png)

## When to use it

- **Building a symmetrical prop:** model the right half of a gun or a car, and see the whole thing as you go.
- **Placing pairs:** a pair of wheels, lamps or handles. Move one to the side, and its twin moves the other way, the same distance from the middle.
- **Keeping both halves matching:** the other side is always an exact mirror, so nothing drifts apart.

## How to use it

1. **Model one half.** Start from a half, or delete the half of your shape on the other side of the mirror (in the [Polygons mode](modes.md), select the polygons on that side and press **Delete**). A whole shape would get a whole second copy on top of it.
2. Select the object, and click the **Symmetry** button at the right end of the top palette (or **Tools → Symmetry**). It lights up, the mirrored half appears, and a **faint red sheet** shows where the mirror stands, with a clearer **red line** where it meets the ground.
3. Work on your half as usual: in any mode, with any tool. Each edit is made once, on your half; the other half follows.
4. Move the object (in the Model mode): its twin moves the opposite way, mirrored.

Click **Symmetry** again (with the object selected) to switch its mirror off: the mirrored half goes away, and your half is just as you left it. **Ctrl + Z** brings the mirror back.

Each object keeps its own mirror, saved with the project.

### Where the halves meet

Points of your half that sit **on the mirror** join their mirrored copies, so the two halves become one closed surface with no seam. Points within the **Weld distance** of the mirror (0.1 cm as standard) count as on it. So when you model a half, keep its middle points on the mirror: in the Front view, the red line shows exactly where it is.

A point you move away from the mirror leaves it again: the weld is live, never fixed into the shape.

### The Symmetry menu

![The Symmetry menu: On, Mirror across X, Y and Z, Move the mirror to X, Y and Z, and the Weld distance.](../screenshots/2026-10-16-symmetry-menu.png)

Click the small **arrow** just right of the Symmetry button for its settings. They change the selected objects' mirrors (one undo step while you work in the menu), and are kept for the next object you mirror.

- **On**: the same as the button, for the selected objects.
- **Mirror across X, Y, Z**: which of the **world's** axes to mirror across. X mirrors left and right (the standard), Y front and back, Z top and bottom. Pick two for four quarters, three for eight.
- **Move the mirror to X, Y, Z**: where the mirror stands in the world: the world centre (0) unless you move it. Moving it moves the twin.
- **Weld distance**: how close to the mirror points must be to join their copies. **0** keeps the halves apart.

The mirror stands along the world's axes, not the object's: turning the object turns its twin the mirrored way.

## Settings

| Setting | What it does |
|---|---|
| **Symmetry** (palette button, Tools menu, or On in its menu) | Gives the selected objects a live mirror, or takes it away. Each object keeps its own, saved with the project. |
| **Mirror across X / Y / Z** (Symmetry menu) | Which world axes to mirror across. X as standard. |
| **Move the mirror to X / Y / Z** (Symmetry menu) | Where the mirror stands. The world centre (0) as standard. |
| **Weld distance** (Symmetry menu) | How near the mirror points join their copies. 0.1 cm as standard, up to 10 cm; 0 keeps the halves apart. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Symmetry on or off | The Symmetry palette button, or Tools → Symmetry |
| Symmetry settings | The arrow beside the Symmetry button |

## Tips and common problems

- **Problem:** the shape looks doubled, two copies overlapping. **Why:** the whole shape was mirrored, not half of it. **Fix:** delete the half on the other side of the red sheet, in the Polygons mode.
- **Problem:** a thin crack runs down the middle. **Why:** the middle points are not quite on the mirror. **Fix:** move them onto it (snap to the grid, or type 0 for X in the Coordinates panel), or raise the Weld distance a little.
- **Problem:** clicking the mirrored half selects nothing. **Why:** it is a copy; only your half can be clicked and edited. **Fix:** click your half (in the Model mode, or the object's name in the Objects panel); edit the matching part of your half, and the copy follows.
- **Problem:** the twin is far away. **Why:** the mirror stands at the world centre and the object is far from it. **Fix:** move the object next to the red sheet, or move the mirror to it in the Symmetry menu.
- **Tip:** to keep a detail on one side only (a scratch, a logo), make it a separate object without a mirror.
- **Note:** projects made before the live mirror keep their mirrored edits, made on both sides, exactly as they were.

## Related

- [Points, Edges and Polygons modes](modes.md)
- [Move, rotate and scale](transforming.md)
- [Export and import](export-and-import.md): the mirrored half goes out as part of the object
- [Concepts: symmetry](concepts.md#objects-and-scenes)
