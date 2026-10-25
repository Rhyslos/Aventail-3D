# 0160. Modular kit helpers: a kit grid, kit-sized shapes and pivot presets

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0068](0068-pivot-editing-and-scene-units.md) (pivots placed without moving the shape, the ready-made pivot places, units saved with the project), [0078](0078-kit-export.md) (kit export: each piece its own file, its pivot at the origin), [0030](0030-transform-gizmos.md) (snapping), [0126](0126-rework-and-first-use.md) (Edit pivot as a switch, grouped menus, one development file format), [0109](0109-pins-and-unwrap-methods.md) (a group of commands in one place of the tool ring), [0020](0020-primitives.md) and [0129](0129-more-shapes.md) (the primitives and their settings), [0028](0028-ask-about-controls.md) (ask about controls), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0159](0159-struts.md) (the development file format moves to version 61)

## Problem

Phase 10's twenty-second milestone is **modular kit helpers** (DESIGN.md wish **W16**: "pieces on a grid with sensible pivots"). A modular kit is a set of pieces (walls, floors, corners, doorways, stairs) that a game places side by side on a grid, the way a level designer builds a dungeon from tiles. Two things make or break a kit:

1. **Sizes.** A wall 3.9 m long leaves a 10 cm gap on a 4 m grid; one 4.1 m long overlaps. Each piece's outside sizes should be whole steps of the kit's grid (1 m, 2 m or 4 m are common), while its thickness can be anything.
2. **Pivots.** The engine places a piece by its pivot. A floor tile or a wall snaps cleanly with its pivot at a bottom corner; a prop wants it at the middle of its base; a lamp hanging from the ceiling at the middle of its top. The app can already move a pivot without moving the shape (Edit pivot, decision 0068), and has Pivot to centre, but getting it exactly onto a corner meant snapping by hand.

The milestone notes ask for a **Kit grid** project setting (1 m recommended, 2 m and 4 m and any other), a **Snap size to kit grid** command for the selected shapes, a Create menu option **Kit-sized shapes**, and **pivot presets** (bottom centre, a bottom corner, centre, top centre) as a command with a list, all fitted into what exists rather than beside it. The Create menu (18 shapes, 5 curves, Group) and the modifiers palette (which had to start scrolling in 0159) are crowded, so new entries should go into a short list of their own or Attributes rows.

## Options

Where the kit grid lives:
1. **A project setting, saved with the project, changed as an undo step (chosen),** as the project's units are (0068): a kit is a project, and two projects can have different grids.
2. A preference of the person, as the viewport grid's spacing is: one grid for every project, lost when the file is sent to someone else.

Where the kit's settings are set:
1. **A Modular kit list at the end of the Create menu (chosen):** the grid (1 m, 2 m, 4 m, and a field for any other), Kit-sized shapes, Size to kit grid and the snapping switch, in one place, under one new entry. The list stays open while its settings change, so a step can be typed.
2. The Attributes panel's Units menu: next to the units, but Size to kit grid and Kit-sized shapes aren't about units.
3. Settings: for preferences of the person, not settings of the project.
4. A modifiers palette button: the palette is for modifiers, and full.

Snapping to the kit grid:
1. **A Snap setting, *Snap moves to the kit grid* (chosen):** when on, a move that doesn't find a point, edge or polygon lands on whole steps of the kit grid instead of the grid lines shown. It joins the existing Snap settings (Settings → Viewport → Snapping) and is also in the Modular kit list. A preference of the person, as the other Snap settings are.
2. A new grid drawn in the viewport at the kit's step: more to draw, and a third grid next to the 1 m / 50 cm / 10 cm levels. Left for later if wanted.

What Size to kit grid rounds:
1. **Each outside size to the nearest whole number of steps (at least one), leaving any size shorter than half a step as it is (chosen):** a 3.9 m × 20 cm × 3.1 m wall becomes 4 m × 20 cm × 3 m on a 1 m grid: its thickness isn't a size it lines up along. Round shapes are rounded by how far across they are (their diameter); parts inside the outline (a tube's hole, a gear's teeth and axle hole, a torus's tube) keep their proportion to it; an arch's outside width and its height to the top are rounded, its legs taking up the difference. Counts (segments, sides, steps) and angles don't change.
2. Every size, thin ones included: a 20 cm wall would become 1 m thick.
3. Round up only: a 4.1 m wall would become 5 m.

What it works on:
1. **Shapes with settings (primitives), as a change of their settings (chosen):** the shape stays a shape whose sizes can still be typed, edits recorded on top replay, and undo puts the old sizes back. Baked meshes and groups are refused with a reason (they have no sizes to round; scaling them would stretch bevels and holes).
2. Scaling the object: works on anything, but leaves the object scaled (decision 0089's Apply transforms exists for that) and stretches details.

Kit-sized shapes:
1. **A switch saved with the project (chosen), stored with the kit grid:** with it on, a new shape (B list, Alt + B list, Create menu, palette buttons, command search) is made at the kit grid's size instead of 1 m (every length times step ÷ 1 m) and then fitted, so on a 2 m grid a new cube is 2 m and a gear 2 m across (and still a thin slab). With a 1 m grid nearly nothing changes, since the shapes are already 1 m.
2. A preference of the person: would make shapes kit-sized in projects that aren't kits.

The pivot presets:
1. **Four commands on the shape's box in its own axes (chosen): Pivot to bottom centre, Pivot to bottom corner, Pivot to centre (the existing one), Pivot to top centre.** The box is the shape's own (as Pivot to centre's), so on a turned object the corner is the shape's corner, not the world's; for a group, the box round everything in it. Like the other ready-made places, they move the pivot and keep its turn; the shape doesn't move; one undo step each.
2. More places (each of the eight corners, each side's middle): more to choose from than a kit needs; a corner and the bottom centre cover floors, walls and props, and the pivot can still be snapped anywhere by hand.

Which bottom corner:
1. **The one lowest along the shape's X and Y (chosen): the front left corner, seen in the Front view.** A floor tile or wall piece with its pivot there grows to +X and +Y, the way grid coordinates count, so its position on the grid is the tile's number times the step.
2. A choice of corner: four more commands, or a setting, for little gain; Edit pivot with snapping reaches any other corner.

Where the presets go:
1. **A *Pivot presets* list in the Tools menu (chosen),** in place of Pivot to centre (now one of the four), with Pivot to world origin, Pivot to object… and Object to world origin as before; and **a *Pivot presets* group in the Edit pivot ring (E)**, in its empty last place, opening the four round it as the UV ring's turns do (0109). Command search finds each.
2. Four new entries straight in the Tools menu: a longer menu.

## Decision

### The work (`mesh::primitives::kit`, layer 0)

- `Primitive::scaled(factor)`: every length times `factor` (counts and angles kept), within limits; a factor that isn't a positive number changes nothing.
- `Primitive::fitted_to_grid(step)`: the outside sizes as above, within limits; a step outside **1 cm to 100 m** (`MIN_GRID_STEP`, `MAX_GRID_STEP`, `is_grid_step`) or not a number changes nothing. Fitting twice changes nothing more. Never past the largest size (10 km).

### In the scene

- `scene::KitGrid { step, sized_shapes }` (standard **1 m**, Kit-sized shapes **off**), part of the scene; `KIT_GRID_CHOICES` (1 m, 2 m, 4 m).
- `Command::SetKitGrid(KitGrid)`: an undo step ("Kit grid 2 m", "Kit-sized shapes on"); a step out of limits is refused; the same grid again is no step. No shape changes.

### In the interface

- **Create → Modular kit** (`kit::kit_entries`, `workbench::kit`): **Kit grid** (1 m, 2 m, 4 m, **Other:** a field in the project's units, a drag one undo step), **Kit-sized shapes**, **Size to kit grid** (greyed out with the reason when it can't be used), and **Snap moves to the kit grid**. The list stays open while it's used.
- `CommandId::SizeToKitGrid` ("**Size to kit grid**"): every selected primitive's settings fitted, as one undo step (`Command::SetPrimitive` per shape under one merge key). Refused, with the reason, with nothing selected, no shapes with settings, or everything already fitting.
- **Kit-sized shapes**: `Workbench::new_primitive` makes every new primitive (Create menu, B and Alt + B lists, palette shape buttons, command search).
- `SnapSettings::kit_grid` (standard off; **Settings → Viewport → Snapping** and the Modular kit list): moves snap to `move_step`, the kit grid's step instead of the grid lines shown; also when drawing curves.
- `CommandId::PivotPresets` (a group) and `PivotToBottomCentre`, `PivotToBottomCorner`, `PivotToTopCentre` (with `PivotToCentre`): `pivots::Placement::{BottomCentre, BottomCorner, TopCentre}` place the pivot at a point of the shape's box (`Workbench::box_point`) with `Command::MovePivots`. In the Edit pivot ring's place 14 and **Tools → Pivot presets**.
- Command search finds Size to kit grid (under Create), each preset (under Tools; the other pivot commands now say Tools too, not Mesh), and Kit-sized shapes and Snap moves to the kit grid.
- Icons drawn in code (`icons/kit.rs`): a box seen from the front with its pivot where each preset puts it, and a box filling two squares of a grid.
- **No new keys or mouse actions.**

### Saving

Development file format **version 61**: version 60's model with the project's kit grid (`ProjectV43::kit_grid`, `v61::KitGridV61 { step, sized_shapes }`), stored only when it isn't the standard, checked on reading (a step out of limits makes the file refused as damaged, as any other bad value). Older files open with 1 m and Kit-sized shapes off. New sample `sample-v61.avtl` (version 60's sample with a 2.5 m grid and Kit-sized shapes on). An older app would drop the kit grid without a word, so it says it needs updating instead.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** Create → Modular kit, Tools → Pivot presets, the Edit pivot ring (E) and command search.
- The kit grid is **1 m as standard**, with **1 m, 2 m and 4 m** in the menu and any other step from 1 cm to 100 m typed; **saved with the project**.
- **Size to kit grid** rounds each outside size **to the nearest** whole step (at least one), and **leaves sizes under half a step alone** (thickness). Round shapes by their diameter; holes and teeth keep their proportion. **Shapes with settings only**: baked meshes and groups are refused with a reason.
- **Kit-sized shapes** is **off as standard** and **saved with the project**; on, new shapes are made at the grid's size, then fitted.
- **Snap moves to the kit grid** is a **Snap setting** of the person (off as standard), in Settings and the Modular kit list; **no kit grid is drawn** in the viewport.
- The bottom corner is the one **lowest along X and Y** (front left in the Front view), in the **shape's own axes**.
- The presets are **Tools → Pivot presets** (Pivot to centre moves into it) and a **group in the Edit pivot ring's empty place**.

## Consequences

- A kit's pieces can be sized to the grid in one step, made at its size from the start, placed by snapping to it, and given the pivots a game engine wants, then exported each as its own file with File → Export kit… (0078).
- Projects with a kit grid other than the standard, or Kit-sized shapes on, need this version of the app or later.
- Baked meshes can't be sized to the grid; size them before baking, or scale them by hand.
- Not yet: a kit grid drawn in the viewport, a check that pieces line up (DESIGN.md W16's "checking that pieces line up"), a choice of corner, pivot presets for points, edges and polygons.
