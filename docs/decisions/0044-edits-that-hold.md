# 0044. Edits that hold: ids, switches, trusted ids, stretching rows

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0021](0021-object-stack-foundation.md), [0032](0032-edits-find-their-points-again.md), [0043](0043-attribute-layers-and-the-builder.md); from the [whole-app review](../reviews/2026-10-13-whole-app-review.md) (A2, A4) and the project lead's request of 2026-10-13

## Problem

- **Edits had no identity** (review A4). They were addressed by their place in the list, with no on/off or pin switches. The edit list planned for Phase 6 needs both.
- **Changing an earlier edit could lose later ones** (review A2). An edit finds its points again by position when the ids don't fit. But it couldn't tell "the points were renumbered" (more segments) from "the points were moved by an edit below" (a longer extrude). In the second case the right points still existed, yet the search looked in the old places and came back empty.
- **New rows bunched up** (the project lead's case). Lift a cube's top to three times its height, then raise its height segments. The new rows were spread over the cube's original height, leaving one tall stretched row above them.
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
3. **Moves keep their reach, and new rows stretch along.**
   - When the gizmo moves, turns or scales points of a primitive, the edit also keeps every edge from those points to their neighbours. For a neighbour that also moved it stores that point; for one that stayed, where it was.
   - When the shape's structure later changes, new points lying along those edges get part of the move: all of it next to a moved point, none next to one that stayed, in proportion in between (`Anchors::stretch`). Between two moved points, they get all of it.
   - Plain meshes never get renumbered, so their moves keep no reach.
4. **File format version 5.**
   - Each recorded edit is stored with its id and switches, beside version 4's operation stored as it was.
   - Each operation's anchors gain the structure fingerprint and the reach.
   - Plain meshes' attribute layers are stored beside the objects; hard edges stay with the mesh.
   - Everything is checked on opening: reach that names a point the edit doesn't have, numbers that aren't real, two edits with one id, layer values beyond the mesh's elements, and layers for objects that aren't there or have no mesh.
   - Edits in older files are numbered 1, 2, 3… in order, switched on and unpinned.

## Costs, measured

- **Reach:** about 20 bytes per edge in memory (less on disk, compressed). 160 bytes for a lifted cube top, a little more than the 50 estimated beforehand, because inner edges are kept too so new points between two moved points move fully.
- **Fingerprint:** 8 bytes per edit. Worked out once per set of polygons and kept with them, so it costs nothing while dragging.
- **Heavy-mesh check (3 million polygons):** every step still meets its target.
  - Dragging an extrude: 37 ms per frame.
  - Saving: 1.6 s.
  - Opening: 0.6 s.

## Consequences

- An edit now holds on to its points through changes below it that don't renumber them. That covers most changes to an earlier edit's settings once the edit list exists.
- What's still open from the review is stable element names (A3). When an earlier edit's change *does* renumber what comes after it (more bevel segments, say), later edits still fall back on finding by position. That comes before the Phase 6 edit list makes such changes common.
- A project saved in version 5 doesn't open in an app that only knows version 4.
