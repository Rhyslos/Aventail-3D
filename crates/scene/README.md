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
- **Stack** (`stack`, [decision 0021](../../docs/decisions/0021-object-stack-foundation.md)): an object's kind is its base (`ObjectKind::Primitive`, or `ObjectKind::Mesh` for a plain mesh) and `Object::stack` holds `Layer`s of `Operation`s (so far `MovePoints`). `Command::Record` adds an operation to the top layer (making an `Edits` layer first), as `InsertLayer`/`InsertOperation` edits; groups refuse (`SceneError::NoShape`). `Scene::evaluated` gives the final shape (`Evaluated`: mesh plus `StackWarning`s for operations whose points are gone), from a per-layer cache inside the scene that re-evaluates only from the first changed layer (or everything, when the base changed); `Scene::local_bounds` and the world bounds use it. `Command::MakeEditable` bakes shapes into mesh bases and empties their stacks; `Scene::can_make_editable` says whether it would change an object. Evaluation takes an `EvalContext`, empty until animation.
- **Rebuilding saved scenes**: `Scene::from_objects` (objects in tree order, checked: unique ids, parents first, names, groups without edits; `next_object_id` raised past every id), `Document::from_scene` (with a selection and a fresh history), `Stack::from_layers`, `ObjectId::from_raw`.
- **`Document`**: the scene, its history and the selection. `execute` carries out a command as small reversible edits and records them as one step (failures change nothing; no-ops record nothing); `undo`/`redo` replay the edits' opposites or the edits. `execute_merging` folds a change into the previous step when made with the same key (one drag of a value is one step; dragging back to the start leaves no step; a step undo returned to never merges). History is bounded by memory, not steps ([decision 0022](../../docs/decisions/0022-undo-memory-budget.md)): `memory` estimates each step's size (meshes in full), the history keeps a running total, and over the `HistoryLimit` (`Bytes`, standard 2 GB, or `Unlimited`) it forgets the oldest undo steps first, then far redo steps, always keeping the newest. `set_history_limit`, `history_memory`, `forgotten_steps`, `redo_steps`. Selection is set directly and is not an undo step yet. `revision` goes up with every change to the scene (not the selection), so the interface knows when to redraw.
- **Selected elements** ([decision 0029](../../docs/decisions/0029-selection.md)): `Document::elements(id)` gives an object's `ElementSelection`; `select_elements(changes, label, merge_key)` changes one or more objects' as one undo step (a merge key folds a brush stroke into one step; no step when nothing changes). Undo and redo restore them; a step holding only selection changes doesn't change `revision` (nothing to save). `element_revision` goes up with every change. Deleted objects lose theirs. Not saved in project files yet. **Selected objects** are undoable too: `set_selection`, `toggle_selected` and `clear_selection` record a step (labelled "Select Cube", "Select 3 objects", "Deselect all"), and `select_objects(ids, merge_key)` merges a brush stroke; `from_scene` sets its selection without one. **Transforms** ([decision 0030](../../docs/decisions/0030-transform-gizmos.md)): `Command::SetTransforms` (edit `SetTransform`, merged within a step; values that aren't numbers refused with `SceneError::NotANumber`); `Operation::TransformPoints` (points moved by a matrix in the object's own space); `Transform::from_matrix` (the reverse of `matrix`, nearest without skew); `Document::execute_labelled` (a command with the caller's undo label) and `Document::abandon(key)` (takes back and forgets the last step if made with that merge key: a gizmo drag replaces its previous frame, and cancels).

Tests: the undo budget (`history_tests`: sizes growing and moving between undo and redo, forgetting the oldest, keeping the newest however large, unlimited, lowering the limit, redo steps, merged drags not growing, and a random run checking the running total against a fresh measure); the stack (`stack_tests`: recording, undo and redo of edits, replay after a settings change, warnings for missing points, bounds following edits, the cache re-evaluating only what changed, switched-off layers, mesh bases, deleting and undoing, copies, determinism, and Make editable with groups, meshes, several objects and unknown ids); every command, its undo and redo (ids included), refused moves (into itself, unknown objects), names, visibility through parents, world bounds through parent transforms, and a fuzz test running random command sequences with seeds, checking the tree stays whole and that undoing everything and redoing it all lands exactly where it was, including recorded edits, Make editable and the evaluated shapes.
