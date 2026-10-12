# 0038. Extrude and inset

- **Status:** Accepted
- **Date:** 2026-10-13
- **Follows:** [0006](0006-base-plus-operation-stack.md), [0028](0028-ask-about-controls.md), [0032](0032-edits-find-their-points-again.md), [0037](0037-tool-menu-and-command-list.md)

## Problem

Extrude and inset are the first modelling tools that add points and polygons rather than only moving them. They need:

- **controls** for setting the distance;
- a **rule for neighbouring polygons**;
- **ids** that keep selections and later edits working;
- a way to change the distance afterwards;
- a place in project files.

## Controls (asked first, per 0028)

The project lead chose:

| Question | Choice | Alternatives offered |
|---|---|---|
| Setting the amount after `D` or `I` | **Drag anywhere** in the viewport, left or right, as in Cinema 4D. Let go to finish; change it afterwards in the Attributes panel | The new part follows the mouse at once and a click finishes (Blender); an arrow handle on the selection |
| Typing an exact value | **Attributes panel only**, as with the gizmos. No new keys | Digits typed during the drag |
| Neighbouring polygons | **A switch for the artist**: together or each on its own | Always together; always separate |

Following the gizmos (0030):
- `Shift` makes the drag ten times finer.
- `Esc` or a right click cancels.
- `Q` goes back to selecting.

A plain click still selects while the tool is in use, so polygons can be picked without leaving it. `D` and `I` were chosen in 0037.

## Decision

**Mesh operations** (`mesh::extrude`), pure functions like every edit:
- `extrude_polygons`, `inset_polygons` and `extrude_edges`, each with a `Grouping`:
  - `Together`: polygons sharing an edge, or edges sharing a point, are one piece.
  - `Separate`: each on its own.
- **Ids stay put.** An extruded or inset polygon keeps its id and gets new corners (`Mesh::replace_corners`). The selection still holds afterwards, and pressing on carries on from there.
  - New points and polygons take the next ids, in a fixed order, so replaying an edit gives the same ids.
  - Inside a piece kept together, points that touch nothing else simply move instead of being copied.
- **Even thickness.** A point moves along the average facing of the polygons around it, lengthened where they meet at an angle (at most four times). Each polygon ends up the full distance away, so two sides of a cube's corner each move exactly the distance.
- **Hard edges carry over.**
  - The new border copies the old border's hard edges.
  - A wall's upright edge is hard where a hard border edge meets it.
  - The result: an extruded cube stays crisp and an extruded sphere stays smooth until the hard-edge tools arrive.
- **Border edges.** Only edges with a polygon on one side can be extruded. They grow flat with their polygon. Others are skipped and counted (`Outcome::skipped`).
- Inset never goes outward from the tool (it would overlap the neighbours). Folding past the middle is allowed by the operation, as in other programs; the Attributes panel can take it back.

**Scene:**
- `Operation::ExtrudePolygons`, `InsetPolygons` and `ExtrudeEdges` keep their elements, distance, grouping and anchors.
  - Anchors are recorded as for moves (0032). When a settings change renumbers the shape, the edit finds its polygons or edges again by position, so a cube given more segments still has its whole top extruded as one block.
- `Command::ReplaceOperation` puts a changed edit in its place, as two small reversible edits.

**Interface:**
- `Tool::Extrude` and `Tool::Inset`. The viewport sends `ToolDragEvent`s and writes the distance by the mouse.
- A drag is one undo step: each frame takes back the last (`Document::abandon`) and records again. Letting go at no distance records nothing.
- After extruding edges, the selection moves to the new outer edges.
- The **Attributes panel** shows the top edit, when it is an extrude or inset, as **Last edit** (Distance and Keep together), changed through `ReplaceOperation` merged per gesture.
  - With no such edit, the tool's own Keep together switch shows while the tool is in use.
  - Changing the switch on the last edit also sets it for the next drag, so only one switch is ever shown.
- The **Mesh menu** entries for built tools now work.

**Project files:** format **version 3** (`project/v3.rs`), following 0031's plan.
- `OperationV3` stores version 2's operations as they were (`V2(...)`), then the three new ones. Every distance is checked, and an edge must join two different points.
- Versions 1 and 2 are still read; `sample-v3.avtl` is committed with a test.

## Consequences

- Saved projects need this version of the app or newer. Older builds refuse them clearly ("made by a newer version").
- **Heavy meshes:** each frame of a drag copies the face list and works the topology out again. That is quick for game assets but will be slow on the 3-million-polygon test shape (not measured yet). The heavy-mesh check at the phase gate will show whether partial updates are needed.
- The drag follows the screen, so it is quicker when zoomed out. Shift and the Attributes panel give precision.
- **Bevel** (`V`) can reuse `Grouping`, the Last edit section and the drag.
- Points mode has no extrude yet. Extruding inner edges (with polygons on both sides) does nothing; a later tool may make "extrude edges as a flap" if wanted.

## Also decided

At the project lead's request, a **splash screen while the app starts** is added to Phase 5 of the plan (the project lead chose Phase 5 from: Phase 5, Phase 6, the ideas backlog, or right after this milestone).
