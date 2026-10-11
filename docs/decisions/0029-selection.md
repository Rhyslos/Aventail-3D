# 0029. Selection: controls, where element selections live, and selecting through

- **Status:** Accepted (controls, undo and double-clicks revised after the project lead's first test, the same day)
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

The first version gave Shift and Ctrl different jobs for clicks and for drags. After testing, the project lead asked for one pattern everywhere. **Plain replaces, Ctrl adds, Shift removes** applies to clicks, the box, the lasso, the brush and double-clicks alike. A click with the "add" key on something already selected unselects it.

Other patterns can be chosen in Settings (`SelectKeys`), as the project lead asked:

| Pattern | Adds | Removes |
|---|---|---|
| Ctrl adds, Shift removes (standard) | Ctrl | Shift |
| Shift adds, Ctrl removes | Shift | Ctrl |
| Ctrl flips | Ctrl flips each thing (a brush stroke flips each thing once) | — |
| Ctrl adds, Alt removes | Ctrl | Alt |

In the last pattern, Alt + drag still orbits while "Alt + mouse works in every scheme" is on (navigation comes first, [0015](0015-navigation-schemes.md)). There, Alt removes with clicks, and with drags only when that setting is off. Settings explains this.

| Action | Control |
|---|---|
| Select | Click or drag replaces; click on nothing unselects |
| Add, remove | The pattern's keys, as above |
| Switch Box → Lasso → Brush | `Q` (Halberd's select key, pressed again) |
| Loop, ring | Double-click, `Alt` + double-click (in every pattern; the add and remove keys add or take away the loop or ring) |
| Grow, shrink | `Ctrl` + wheel up or down; with the brush, `Ctrl` + wheel sizes the brush |
| Select all, invert | `Ctrl + A`, `Ctrl + I` |
| Unselect parts | `Esc` in the Points, Edges and Polygons modes (again in Model unselects shapes) |

`Ctrl` + wheel no longer zooms the view.

**Double-clicks:**
- Two clicks count as one double-click only within 0.5 s, Windows' standard (egui's own is 0.3 s, which felt unreliable).
- They must also land within 6 screen points of each other. egui only checks the time, so two quick clicks on different shapes counted as a double-click and the second was lost.
- Double-clicks reach edges twice as far from the mouse as hovering does.

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

1. **Not an undo step**, like object selection in the Object Manager before.
2. **An undo step for elements only**, as DESIGN.md §4.3 and [0022](0022-undo-memory-budget.md) planned for this milestone.
3. **An undo step for every selection**: elements, and whole objects too, in the viewport and the Object Manager.

Decided: option 3, at the project lead's request after testing option 2.
- `Document::set_selection`, `toggle_selected` and `clear_selection` record a step ("Select Cube", "Select 3 objects", "Deselect all"); selecting what is already selected records nothing.
- A brush stroke is one step. Its dabs merge with a stroke key, as Attribute drags do.
- Undoing a selection step doesn't count as an unsaved change.
- As in most editors, a new selection after undoing forgets what could be redone.

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
