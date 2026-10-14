# 0047. Loop cut: hover, click or drag; Cuts in the Attributes panel; new keys; file format version 6

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0028](0028-ask-about-controls.md) (ask about controls), [0037](0037-tool-menu-and-command-list.md) (the command list), [0043](0043-attribute-layers-and-the-builder.md) (the builder), [0044](0044-edits-that-hold.md) (file format version 5)

## Problem

Loop cut is the first Phase 4 tool that works on the shape under the mouse rather than on a selection. Several things needed deciding:

- **The controls**: how the cut is placed, and how the number of cuts is set.
- **The keys**: `F` for Loop cut and `X` for Slit had been agreed, but `F` framed the selection and `X`, `Y` and `Z` locked axes.
- **What happens where the ring of polygons ends**, at a triangle, a polygon with five or more sides, or an open border.
- **How the cut is stored**, so it can be changed afterwards and follow later changes to the shape.

## Options

### The number of cuts

1. **The mouse wheel while pointing at an edge.** Quick, but the wheel zooms the view. The project lead asked to use it only if Cinema 4D or 3ds Max do. Neither does:
   - Cinema 4D's loop cut changes the count with the *middle* mouse button, a heads-up display and its attribute panel.
   - 3ds Max's SwiftLoop has no wheel control.

   The middle button is our pan.
2. **A number in the Attributes panel** (chosen): set before cutting, and changeable afterwards as the Last edit.

### Placing the cut

1. **Click only.**
2. **Hover, click, drag** (chosen by the project lead): the loop shows while pointing; a click cuts there; pressing and dragging along the edge slides it before letting go.

## Decision

1. **Controls.**
   - In the Points, Edges or Polygons mode, press `F` (or pick Loop cut from the tool menu `E` or the Mesh menu).
   - Point at an edge of a selected object: the new loop shows across every polygon it would cut.
   - **Click** to cut. **Press and drag** along the edge to slide the cut, then let go. A cut, with its sliding, is one undo step.
   - With one cut, it sits where the mouse is along the edge. Several cuts are spread evenly, and dragging slides them together.
   - **Cuts** (1 to 64) is set in the Attributes panel before cutting. Afterwards **Cuts** and **Position** show as the Last edit, and changing them reshapes the cut just made.
   - Esc or a right click cancels a drag. While Loop cut is in use, a left click never selects. In the Model mode clicks select objects as usual.
   - Shift makes the slide ten times finer, as with every drag.
2. **Keys** (settled 2026-10-13, switched now in one go):
   - `F` is Loop cut;
   - `Z` frames the selection (`H` still frames everything);
   - the axis locks move to `Shift + X / Y / Z`;
   - plain `X` stays free for Slit.
3. **The cut** (`mesh::loopcut`).
   - **The ring.** It runs through four-sided polygons. From the picked edge it goes into each polygon and out through the edge opposite, both ways, until it comes back to the start (a closed ring) or reaches:
     - a polygon that isn't four-sided;
     - an open border;
     - an edge shared by three or more polygons.
   - **Where each edge is cut.** Every edge of the ring is cut at the same share of its length, measured from the same side. So the loop runs straight across polygons of different sizes.
   - **Where the ring ends.** The polygons there (triangles, many-sided polygons, the polygons beyond an edge shared by three) take the new points into their corners. No gap opens, and nothing is joined that wasn't.
   - **Ids.** Cut polygons keep their id for their first piece. New points and polygons take the next ids in a fixed order.
   - **Values.** Everything is carried through the builder ([0043](0043-attribute-layers-and-the-builder.md)):
     - new points and corners blend their two ends;
     - new polygons copy the one they were cut from;
     - a cut edge's values (hard, seam) go to each piece.
4. **The edit.** `Operation::LoopCut` holds:
   - the edge;
   - the number of cuts;
   - the position (0 to 1 from the edge's lower point id);
   - anchors.

   When the points are renumbered (a shape given more segments), the edge is found again by its anchors, and the position is turned round if needed, so the cut stays on the same side. If the edge was split, the cut follows one of its pieces. The size guard counts the polygons of the ring times the cuts.
5. **File format version 6.**
   - Version 5's file, whose operations add `LoopCut` beside version 4's (stored as they were).
   - The edge's two points must differ, and the position must be a real number.
   - The cuts are brought into 1 to 64 and the position into 0 to 1.
   - Version 5's structures became generic over the operation, so later versions add only their new kinds.

## Consequences

- Loop cut works on any shape made of four-sided polygons: boxes, cylinders' sides, spheres' bands, grids. Rings stop cleanly at caps and poles.
- Muscle memory changes:
  - `F` no longer frames; `Z` does.
  - `X`, `Y` and `Z` alone no longer lock axes; Shift does.

  The manual, the tool menu and the Tools menu all show the new keys.
- Projects saved now need this version of the app or newer. Older projects still open.
- Not yet: snapping the slide to the middle or to steps, and an "even" mode that keeps the cut a fixed distance from one side. These can be added later as Attributes panel settings.
