# 0029. Selection: controls, where element selections live, and selecting through

- **Status:** Accepted
- **Date:** 2026-10-11
- **Builds on:** [0027](0027-component-modes.md)

## Problem

Phase 3's selection milestone needs these:
- clicking, a box, a lasso and a painting brush;
- loops, rings, grow and shrink;
- select all and invert;
- undo of selection changes.

It also needs clicking shapes in the viewport, which the project lead expected and which didn't exist yet. Four questions had to be answered:
- which controls to use;
- where the selected points, edges and polygons are kept;
- whether selecting them is an undo step;
- what a box does with the far side of a shape.

## Options and decision

### Controls (chosen by the project lead, [0028](0028-ask-about-controls.md))

| Action | Control |
|---|---|
| Select | Click (replaces what's selected); click on nothing unselects |
| Add or remove one | `Ctrl` + click |
| Box, lasso, brush | Drag replaces, `Shift` + drag adds, `Ctrl` + drag removes |
| Switch Box → Lasso → Brush | `Q` (Halberd's select key, pressed again) |
| Loop, ring | Double-click, `Shift` + double-click (`Ctrl` adds them) |
| Grow, shrink | `Ctrl` + wheel up or down; with the brush, `Ctrl` + wheel sizes the brush |
| Select all, invert | `Ctrl + A`, `Ctrl + I` |
| Unselect parts | `Esc` in the Points, Edges and Polygons modes (again in Model unselects shapes) |

`Ctrl` + wheel no longer zooms the view.

### Select through, or visible only

1. **Visible only.** Many programs do this by default. The project lead dislikes it, because a box over a model misses its back.
2. **Through as standard, with a switch.** The box, lasso and brush catch the far side too; a switch on the left palette (and in the Select menu and Settings) limits them to what can be seen.

Decided: option 2.
- Clicks, double-clicks and the hover highlight always take what is visible, since the mouse points at one thing.
- The switch is remembered between sessions with the other view options.

### What counts as inside

- **Points:** inside the region.
- **Edges:** both ends inside a box or lasso, or passing through the brush.
- **Polygons:** the middle inside. The brush also catches the polygons under its middle, so painting over a large polygon selects it.
- **Shapes, in the Model mode:**
  - any point inside;
  - any edge crossing the region;
  - or the surface under the region's middle.

### Where element selections live

1. **On the mesh**, as a flag per element. Mixes what the shape is with what is being worked on, and every selection change would copy meshes for undo.
2. **In the document**, per object: `ElementSelection` (points, edges and polygons kept apart, by their stable IDs), in `Document`.

Decided: option 2.
- Each mode keeps its own selection, and switching modes doesn't lose it.
- Elements whose IDs no longer exist (after changing a primitive's segments, say) are ignored, not errors.
- The type and the selection commands (all, invert, grow, shrink, loop, ring) live in the `mesh` crate (`mesh::selection`), with their tests.

### Undo

1. **Not an undo step**, like object selection in the Object Manager.
2. **An undo step for elements**, as DESIGN.md §4.3 and [0022](0022-undo-memory-budget.md) planned for this milestone.

Decided: option 2.
- A brush stroke is one step. Its dabs merge with a stroke key, as Attribute drags do.
- Undoing a selection step doesn't count as an unsaved change.
- Selecting whole objects stays outside undo for now, as in the Object Manager.

### Polygon loops and rings

- A double-click in the Polygons mode uses the polygon's edge nearest the mouse.
- The *loop* is the band of polygons running across that edge.
- The *ring* is the band at right angles to it.
- In the Points mode, a double-click uses the edge passing nearest the mouse.

## Consequences

- **Selections aren't saved in project files yet.** That comes with a later file format version.
- **Hidden selected elements are hard to see.** Points and edges selected on the far side are hidden by the shape, like the rest of the overlay; the status bar counts them. Showing them faintly needs a pipeline without the depth test, if wanted.
- **Region selection tests every element against every triangle when "visible only" is on.** That is fine for game assets. A spatial index (BVH) can be added inside `tools::PickScene` without changing its users.
- **Moving the selection** (the gizmos) is the next milestone. It will record `Operation`s that use these selections.
