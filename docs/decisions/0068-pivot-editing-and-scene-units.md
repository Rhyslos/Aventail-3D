# 0068. Pivot editing in the Object mode, and units saved with the project; file format version 16

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** [0028](0028-ask-about-controls.md) (ask about controls), [0030](0030-transform-gizmos.md) (the gizmos), [0006](0006-base-plus-operation-stack.md) (base plus operations)

## Problem

Phase 5 is the export pipeline, and export needs two things first:

1. **Pivots that can be placed.** A game engine places, turns and snaps an asset by its pivot (its origin). Every shape here starts with its pivot at the middle of its base, and until now the pivot could only move with the shape. A door needs its pivot on the hinge, a wall piece at a bottom corner.
2. **Units that belong to the project.** Godot and Unity count 1 unit as a metre; Unreal as a centimetre. Metres or centimetres was a per-user display setting, so a project didn't know what it was measured in.

The Object mode's button has been on the palette since Phase 1, described as "including moving their axis", without doing anything different from the Model mode.

## Options

The project lead was asked (decision 0028) and chose:

- **How to move a pivot:** **the Object mode moves the pivot** (chosen): W and R move and turn the pivot only, and the shape stays. Alternatives were a key held while dragging (as in Blender's "affect only origins"), or a separate pivot gizmo.
- **A key for the Object mode:** **none for now** (chosen: "I'll think on it"); the palette and the Modes menu.
- **Ready-made places:** **Centre of the shape, World origin, Object to world origin, and Pivot to object** (chosen), the last described as "place your pivot point to a target, and that can be another model for example".
- **Units:** **a project setting, plus an export scale later** (chosen), rather than only an export option or a fixed metre scene.

How a moved pivot is stored:

1. **Move the shape's points** the opposite way (Blender's "origin to geometry" changes the mesh). Simple, but a primitive would stop being a primitive, and every recorded edit's anchors would need shifting.
2. **An offset between the pivot and the shape** (chosen, as 3ds Max's "affect pivot only"): each object keeps where its pivot is (its transform) and, separately, where its shape sits relative to the pivot. Primitives keep their settings, recorded edits stay in the shape's own space, and nothing about the mesh changes.

## Decision

1. **Every object has an offset** (`Object::offset`, a transform, nothing as standard): where its shape sits relative to its pivot. The shape is drawn, picked, edited and joined at `Scene::shape_matrix` = the pivot's place in the world × the offset. Objects inside it (children) stay relative to its pivot.
2. **`Command::MovePivots`** gives objects new transforms and changes their offsets, and their children's transforms, the opposite way, so nothing visible moves. One undo step; a drag merges into one step like any move. A pivot flattened to nothing (scale 0) or not a number is refused. Copies and split-off pieces keep the offset; instances have their own.
3. **The Object mode**:
   - Move and Rotate (and the Coordinates panel's Position and Rotation) move and turn the pivots: undo steps read "Move Cube's pivot".
   - Scaling does nothing, because a pivot has no size; the Coordinates panel shows a dash for Size.
   - While a pivot moves, no shape moves, so it can snap onto its own shape's points, edges and polygons (a corner, say).
4. **Four pivot commands**, in the Object mode's tool menu (E) and the Tools menu:
   - **Pivot to centre:** the middle of the shape's box; for a group, of everything in it.
   - **Pivot to world origin.**
   - **Pivot to object…:** then click another object, and the selected objects' pivots move to its pivot; Esc or Tab cancels, and the pointer is a cross while it waits.
   - **Object to world origin:** the object moves, shape and pivot together, so its pivot sits at the origin. The project lead's working name was "Object to pivot"; this name says where it goes.

   The ready-made places keep the pivot's turn. The Object mode's ring keeps every place it shares with the Model mode, leaves Scale's place empty, and puts the pivot commands where Duplicate, Make editable, Duplicate as instance and Join are in the Model mode.
5. **Units belong to the project**: `scene::Units` (metres, standard, or centimetres) is part of the scene, saved with it, and changed with `Command::SetUnits` (an undo step) from the Attributes panel's Units menu or Settings → Interface → Units. Settings also keeps **the units for new projects**, a preference remembered between sessions (the old per-user setting becomes this). The scene still stores centimetres; units only change how lengths read. Scaling exports for an engine comes with export.
6. **File format 16**: version 15's file as it was, plus the offsets of objects whose shape doesn't sit on its pivot, and the project's units. A stored offset must be real numbers and not flattened, name an object that is there, and appear once. Older files open with every shape on its pivot, in metres.

## Consequences

- Pivots can be placed anywhere without touching the shape, its settings or its edits, and undo puts them back exactly.
- Everything that works with a shape's place in the world must use `shape_matrix`, not `world_matrix` (which is now the pivot's place): drawing (and so picking and element editing), bounds, joining, and Symmetry's world centre do. New code should do the same.
- Turning the pivot of an object stretched unevenly (scaled more along one axis than another) can't be described exactly as an offset without skew; the nearest one is used, so the shape may change slightly. Rare in practice; the manual says to turn the pivot before stretching.
- The Object mode's gizmo looks like the Model mode's; only the status bar and the undo step's name say it moves the pivot. A different look for the pivot gizmo can come later if it confuses.
- Export will read `shape_matrix` and the project's units for its scale.
