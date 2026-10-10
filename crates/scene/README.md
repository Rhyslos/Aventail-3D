# scene

**Layer 1 · Core**

The scene document: objects, their operation stacks, commands and undo history.

## Purpose

The single source of truth for an open project. Holds the object hierarchy (transforms, names, visibility), each object's base (a parametric primitive or a mesh) and its stack of recorded operations, evaluated and cached layer by layer. Every change is a command that can undo itself; history is bounded by a memory budget, not a step count. See DESIGN.md §4.2–4.3 and [decision 0006](../../docs/decisions/0006-base-plus-operation-stack.md).

## Must never

- Know about files, the GPU or UI
- Be changed except through commands
- Store world-space transforms: only local ones, with world transforms derived
- Assume the scene is frozen in time: evaluation takes a context, ready for animation later

## Depends on

`mesh` (bounding boxes), `uv` (planned). Outside library: `glam` (maths).

## Status

The object tree, commands and undo ([decision 0018](../../docs/decisions/0018-scene-document-and-undo.md)):

- **`Object`**: a name, a kind (`ObjectKind::Group`, or `ObjectKind::Primitive` with its settings, whose `mesh` and `local_bounds` come from the `mesh` crate), a visibility switch and a local `Transform` (position, heading/pitch/bank, scale). `ObjectId`s count up and are never reused.
- **`Scene`**: the tree, with order among brothers and sisters. Reads: `children`, `parent`, `place`, `walk` (top to bottom with depth), `subtree`, `is_inside`, `is_shown` (it and all its parents shown), `world_matrix`, `bounds`/`bounds_of` (world boxes of what is shown), `unused_name`.
- **`Command`**: Create, Insert, Rename, SetVisible, Move (reparent or reorder, before the child at an index), Delete (with everything inside), Group, Ungroup. `describe` gives the Edit menu's wording.
- **`Command::SetPrimitive`**: new settings for primitives of the same kind (clamped; other kinds refused with `SceneError::WrongKind`), as a reversible `SetKind` edit.
- **`Document`**: the scene, its history and the selection. `execute` carries out a command as small reversible edits and records them as one step (failures change nothing; no-ops record nothing); `undo`/`redo` replay the edits' opposites or the edits. `execute_merging` folds a change into the previous step when made with the same key (one drag of a value is one step; dragging back to the start leaves no step; a step undo returned to never merges). History keeps every step for now. Selection is set directly and is not an undo step yet. `revision` goes up with every change to the scene (not the selection), so the interface knows when to redraw.

Tests: every command, its undo and redo (ids included), refused moves (into itself, unknown objects), names, visibility through parents, world bounds through parent transforms, and a fuzz test running random command sequences with seeds, checking the tree stays whole and that undoing everything and redoing it all lands exactly where it was.
