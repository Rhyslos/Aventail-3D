# 0032. Edits find their points again when a shape's settings change

- **Status:** Accepted
- **Date:** 2026-10-11
- **Follows:** [0021](0021-object-stack-foundation.md), [0030](0030-transform-gizmos.md)

## Problem

Phase 3's last milestone: edits recorded on top of the base, so base settings stay editable. The stack ([0021](0021-object-stack-foundation.md)) and the move, rotate and scale tools ([0030](0030-transform-gizmos.md)) already record edits on top of the base. One thing was still wrong.

Recorded edits name points by ID, and a primitive numbers its points in the order it builds them. Give a cube more segments after pulling its top polygon up, and the IDs now name *other* points: the edit lifts a strip of points somewhere else, and nothing warns about it (0021 lists this as a known compromise). The roadmap's check for this milestone is exactly that case: change a cube's segments after moving points, and the result should still make sense.

## Options

1. **Number points so they survive changes.** Each primitive would give a point an ID derived from where it sits on the shape (side, row, column). This only works for primitives, needs every primitive rewritten, and doesn't help when the new segments create points that didn't exist before (the middle of a split polygon).
2. **Find points by position.** Each edit also records where its elements were (relative to the shape's box) when it was made. If the IDs no longer sit there, the edit finds the elements again by position.
3. **Ask the user to redo the edit**, or block changing settings once there are edits. Simple, but takes away the point of keeping the base editable.

## Decision

Option 2. A new `mesh::Anchors` is stored beside the point IDs in `MovePoints` and `TransformPoints`:
- the box around the shape when the edit was made;
- each named point and where it was;
- the elements selected when the edit was made (points, edges or polygons), by position.

When the edit is applied:
1. **The IDs still fit** (every point is where the anchors say, measured relative to the shape's box, so a resized cube still fits): the IDs are used as they are. This is the usual case, and stays exact and fast.
2. **They don't fit:** the edit finds its elements by position, in the new shape:
   - **points:** the point at each recorded place;
   - **edges:** every edge lying along a recorded edge, so an edge cut in three by more segments moves as a whole;
   - **polygons:** every polygon lying inside a recorded polygon, so a polygon cut into four moves as a whole.
3. Anything not found is counted as missing, as before.

The Attributes panel's **Edits** section says when edits found their points again (a quiet note, hover for why) and, as before, warns in amber when some elements weren't found at all. Places are compared within 0.1% of the shape's box diagonal. A flat shape (a plane has no height) keeps distances along its flat side instead of dividing by zero.

Edits without anchors (from version 1 files, or recorded by code that has no selection) keep working by ID, as before.

The anchors are saved with the edit in project format version 2 ([0031](0031-project-format-version-2.md)). Version 2 hasn't been released yet (it arrived in the unmerged PR #22), so it is changed rather than adding version 3. Every saved number is checked on reading, as everything else is.

## Consequences

- The roadmap case works: pull up a cube's top polygon, change its segments, and the whole top stays lifted.
- Fewer segments still lose points that have no place left; that case still warns, and going back brings them back.
- Finding by position costs a scan of the shape per edit, only when the IDs don't fit. Fine for game assets; a spatial index can come later.
- An edit made on points that sit exactly where other points will appear (a point moved onto another) may pick the nearest one. This is rare and visible in the Attributes note.
- Later operations (extrude, bevel, cut in Phase 4) should record anchors the same way, so they survive settings changes too.
