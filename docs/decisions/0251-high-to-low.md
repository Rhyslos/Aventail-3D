# 0251. High to low: pairing by name, the cage on the model, cage painting

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 26. A kit has many low-detail pieces, each with its high-detail model; choosing each pair by hand in the Bake window is slow, neighbouring pieces spoil each other's bakes when baked together, small floating details (bolts, panels resting on the surface) need baking onto the piece beneath, and the cage (0250) was a number with nothing to see or adjust where it fell short.

## Decision

- **Matched by name (_low and _high)** in the Bake window: each selected object whose name ends in `_low` (any case) is baked from every object whose name starts with the same and goes on with `_high`: `crate_low` from `crate_high`, and from `crate_high_bolts` as a floating detail (`baker::name_pairs`). Several high objects are baked as one. One set of files per low object.
- **Exploded bakes without moving anything:** each pair is baked on its own, its rays meeting only its own high objects, which is what moving the pairs apart (exploding) is for. Nothing in the scene moves.
- **Floating details** are high objects that lie above the low surface without touching it: the rays from the cage find them first, so their detail is projected onto the surface beneath.
- **The cage on the model:** while the Bake window is open, **Show the cage** (on as standard) draws the selected objects' cage as see-through cyan lines (`OverlayStyle::Cage`): each edge between its points pushed out along their averaged facing by the cage push and their painted push.
- **Cage painting** (**Paint the cage** in the window): the controls already chosen for the seam and UV brushes, as the recommended choice since the project lead had approved them for brushes (2026-11-05): drag over points to push the cage out a step (half the cage's push) there, **Ctrl + drag** pulls it in, **[** and **]** change the size, **Esc** puts the brush down; the Points mode and the selection brush's circle. Each stroke is one undo step. **Clear** clears the painted push.
- **Saved** as a point layer (`mesh::cage`, `bake_cage`, the extra push in the shape's units) set by `Operation::SetCagePush` (file format 93). The bake reads it per texel, blended over each triangle's corners (`bake::maps::cage_pushes`, `bake_map_caged`), never less than a twentieth of the cage's push.

## Consequences

- Names decide the pairs; renaming an object changes what it bakes with.
- The painted push follows the points, so it survives the low model's later edits that keep its points.

## Amended 2026-11-06, the cage on a scaled object

The painted push is kept in the object's own units, and the cage is drawn in them and then placed in the world, so on a scaled object it is drawn scaled. The bake works in the world, so it now scales each point's push the same way (by how the object's matrix stretches that point's facing) before baking; before, a 4× object's bake reached a quarter as far as the cage drawn.
