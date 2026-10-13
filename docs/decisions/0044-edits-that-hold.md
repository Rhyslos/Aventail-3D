# 0044. Edits that hold: ids, switches and trusted ids

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0021](0021-object-stack-foundation.md), [0032](0032-edits-find-their-points-again.md), [0043](0043-attribute-layers-and-the-builder.md); from the [whole-app review](../reviews/2026-10-13-whole-app-review.md) (A2, A4)

## Problem

- **Edits had no identity** (review A4). They were addressed by their place in the list, with no on/off or pin switches. The edit list planned for Phase 6 needs both.
- **Changing an earlier edit could lose later ones** (review A2). An edit finds its points again by position when the ids don't fit. But it couldn't tell "the points were renumbered" (more segments) from "the points were moved by an edit below" (a longer extrude). In the second case the right points still existed, yet the search looked in the old places and came back empty.
- **Attribute layers weren't saved** ([0043](0043-attribute-layers-and-the-builder.md)).

## Decision

1. **Edits become entries** (`scene::Entry`): an `EditId`, an on/off switch, a pinned switch, and the operation.
   - Ids are unique among an object's edits: the next one is one more than the highest.
   - `Command::SetEditEnabled` and `SetEditPinned` change the switches, each one undo step.
   - Changing an edit's settings (the Last edit panel) keeps its id and switches.
   - A switched-off edit is skipped when the shape is worked out, and Make editable leaves it out, as the shape shows.
   - There is no interface for the switches yet; the edit list brings it.
2. **Ids are trusted when the structure is the same.**
   - Anchors keep a fingerprint of the structure of the shape the edit was made on: its polygons' corners by point id, and how many point slots it has, but not where points are (`Mesh::structure_fingerprint`).
   - When the shape below the edit has the same structure, the ids name the same elements wherever the points went, so they are used as they are.
   - Searching by position is only for shapes whose structure changed.
   - The fingerprint is a fixed 64-bit FNV-1a hash written out in the code, the same on every computer and in every version, because it is saved.
3. **File format version 5.**
   - Each recorded edit is stored with its id and switches, beside version 4's operation stored as it was.
   - Each operation's anchors gain the structure fingerprint.
   - Plain meshes' attribute layers are stored beside the objects; hard edges stay with the mesh.
   - Everything is checked on opening: two edits with one id, layer values that aren't real numbers or lie beyond the mesh's elements, and layers for objects that aren't there or have no mesh.
   - Edits in older files are numbered 1, 2, 3… in order, switched on and unpinned.

## Tried and dropped: stretching new rows along an edit

The project lead asked for new rows to stretch with an edit. The case: lift a cube's top to three times its height, then add height segments. The new rows were spread over the cube's original height, below the lift. Two attempts were built and tested:

1. **Along edges.** A move kept its *reach*: every edge from a moved point to a neighbour. New points along those edges got part of the move: all of it next to a moved point, none next to one that stayed, in proportion in between.
   - This worked when segments were added on one axis.
   - It broke as soon as segments were added on a second axis. New points in the middle of a side lie on none of the old edges, so they didn't move.
2. **Across polygons too.** The move also kept the polygons it touched. A point inside one got a share blended from its corners (mean value coordinates, which reproduce straight-line changes exactly).
   - This made boxes correct for every combination of segments.
   - Round shapes given more sides still broke: on a cylinder going from 16 to 32 sides, 64 of 160 points didn't move. The new points lie on the curve, on none of the old flat polygons.
   - Telling "a point on the round surface" from "a point on the far side of a thin plate" is a deep problem.

The project lead judged it too little reward for this stage, so it was removed before merging. New rows follow the shape's settings only, as in most modelling programs, and Loop cut (Phase 4) adds rows where they're needed. The attempt is in the history of pull request #37 if it's ever wanted again.

## Costs, measured

- **Fingerprint:** 8 bytes per edit. Worked out once per set of polygons and kept with them, so it costs nothing while dragging.
- **Heavy-mesh check (3 million polygons):** every step still meets its target.
  - Dragging an extrude: 37 ms per frame.
  - Saving: 1.6 s.
  - Opening: 0.6 s.

## Consequences

- An edit now holds on to its points through changes below it that don't renumber them. That covers most changes to an earlier edit's settings once the edit list exists.
- What's still open from the review is stable element names (A3). When an earlier edit's change *does* renumber what comes after it (more bevel segments, say), later edits still fall back on finding by position. That comes before the Phase 6 edit list makes such changes common.
- A project saved in version 5 doesn't open in an app that only knows version 4.
