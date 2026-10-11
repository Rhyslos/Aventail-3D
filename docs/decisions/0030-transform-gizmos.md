# 0030. Move, rotate and scale: gizmos, drags as one step, snapping and the Coordinates panel

- **Status:** Accepted
- **Date:** 2026-10-11

## Problem

Phase 3's fourth milestone: move, rotate and scale the selection, with axis constraints, snapping and typed values. This needs answers to five questions:
- what the controls are;
- what moves in each mode;
- how a drag becomes undo steps without piling up;
- how snapping works;
- how values are typed.

## Controls (chosen by the project lead, [0028](0028-ask-about-controls.md))

| Action | Control |
|---|---|
| Move, rotate, scale, all three | `W`, `R`, `S`, `T` (Halberd's keys, [0019](0019-only-the-tool-keys-come-from-halberd.md)) |
| Back to selecting | `Q` (pressed again: the next select tool) |
| Handles | Move: an arrow per axis, a square per plane, the centre (free, facing the viewer). Rotate: a ring per axis and an outer ring around the line of sight. Scale: a box per axis, a square per plane (added after the first test: the mouse's direction sets how much each of the two axes scales), and the centre for all axes. |
| A left drag off the gizmo | Selects, as with the Select tool |
| Snapping | The palette button; holding Ctrl while dragging flips it for that drag. A setting makes it button only. |
| Snap steps | Moves: the grid lines shown (1 m, 50 cm or 10 cm); turns: 15°; scaling: 10% |
| Fine | Hold Shift: ten times slower |
| Cancel | Esc or a right click while dragging |
| Locks, axes | `X`, `Y`, `Z` lock an axis; `L` switches world and object axes |
| Typed values | The Coordinates panel |

## Decisions

### What moves

- **Model mode:** the selected objects. An object inside another selected one moves with it, not twice.
  - The gizmo sits at the object's axis; for several objects, at the middle of their axes.
  - A change is a world matrix around that point. Each object's new transform is worked out in its parent's space with `Transform::from_matrix`.
  - A move keeps rotation and scale exactly.
  - Scaling a turned object along the world's axes can't be shown without skew. The nearest unskewed transform is used, and the manual suggests object axes (`L`).
- **Points, Edges, Polygons modes:** the points of the selected elements of the selected objects. The gizmo sits at the middle of their box.
  - Moves are recorded as `Operation::MovePoints`.
  - Turns and scaling are recorded as a new `Operation::TransformPoints` (an affine matrix in the object's own space).
  - Both are recorded on top of the base ([0021](0021-object-stack-foundation.md)), so primitives keep their settings.
- A new command, `Command::SetTransforms`, sets object transforms (edit `SetTransform`; merges in a step like `SetKind`).

### One drag, one undo step: replace, don't pile up

1. **Merge every frame's command into one step.** For objects, transforms merge cleanly. For points, each frame would add another recorded operation, so the stack would fill with hundreds of tiny moves.
2. **Preview outside the document, commit on release.** The renderer, the overlay and the Coordinates panel would all need to know about the preview.
3. **Take back the previous frame and do the whole change again.** `Document::abandon(key)` undoes the last step only if it was made with the drag's merge key, and forgets it (it can't be redone). Each frame then executes the full change from the start of the drag with the same key.

Decided: option 3.
- The scene always shows the real result, with no preview state.
- A drag leaves exactly one step and one recorded operation per object.
- Cancelling is one more `abandon`.
- A frame where the mouse hasn't moved does nothing, so a still mouse doesn't redraw for ever.

### The gizmo is maths in `tools`, drawing in `ui`

- `tools::gizmo` holds the handles' screen shapes, finding the handle under the mouse, and turning mouse movement into a `Change` (translate, rotate around an axis, scale per axis):
  - it applies locks;
  - it accumulates each frame's step, so Shift slows only the steps taken while it is held.
  - Rings measure the turn around the gizmo's centre on screen. This works even when a ring is seen edge-on.
- The viewport draws the shapes with egui on top of the image. The gizmo is never hidden by the model: it's a handle, not part of the scene. It stays `GIZMO_SIZE` points long at any distance.

### Snapping

`Target::snapped`:
- **Moves** first look for a point near the mouse, then a spot on an edge, then the surface under the mouse (`PickScene::snap_target`). What is being moved is excluded: the moving objects, or the moving points and the edges and polygons they belong to.
  - The found spot is projected onto the dragged arrow's line or square's plane, so an arrow keeps its direction.
  - Without a target, moves land on the grid lines shown (world axes) or move in grid steps (object axes).
- **Turns** round to the angle step; **scaling** rounds to the scale step.
- The settings (`SnapSettings`: on, Ctrl flips, targets, steps) are saved with the view options and shown in Settings → Viewport → Snapping.

### The Coordinates panel

- It shows world positions:
  - **One object:** its axis, its own size (local bounds × scale) and heading, pitch and bank.
  - **Several objects, or elements:** the middle and size of their box; no rotation.
- The "Object (Rel)" switch and Apply button from the mockup are gone. Values apply on Enter or while dragging the number, and positions are always world positions.
- A field's drag or typed value is one undo step, through the same replace-the-last-frame mechanism.

## Consequences

- `TransformPoints` needs a new project file format version ([0031](0031-project-format-version-2.md)).
- The next milestone, edits recorded on top of the base, builds on these operations: their points are found again when the base's settings change ([0032](0032-edits-find-their-points-again.md)).
- During a drag the pick scene is rebuilt each frame, for snapping, because the drawing changes. That is fine for game assets; a spatial index can come later.
- Objects can't be scaled to nothing (the matrix would be flat); points can, which is how a selection is flattened.
