# 0238. UV brushes: grab, relax and pinch

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 10. Small UV fixes (a bulge, a crumpled patch, a detail that needs more texture) meant selecting points and moving them one by one.

## Decision

- **`uv::brush`**: `Spots::of(mesh)` groups each point's corners by where they sit in the texture (one spot inside a piece, one per side on a seam), with neighbours along polygon sides; pinned corners and locked pieces are fixed. `Spots::dab(kind, centre, radius, pull, strength, out)` moves spots within the radius by a smooth falloff (`falloff`: (1 − (d/r)²)²): **Grab** by the mouse's move, **Relax** towards the mean of its neighbours, **Pinch** towards the middle (or out). `moved_corners` gives every moved spot's corners for `Operation::SetUvs`.
- **Controls** (the project lead's choice for UV brushes, phase 8 plan): picked from the menus (UV → Pieces → Brush, also through the UV Editor's UV menu), then dragged over the UV Editor; **Ctrl + drag** does the opposite (Pinch pushes out; Grab and Relax relax gently); **[** and **]** change the size (6 to 300 screen points, 40 standard); **Esc** goes back to selecting.
- **Recording**: a stroke keeps each shown object's spots from its start, moves them dab by dab, and records one `SetUvs` per object under the stroke's key, taking back the last dab's record each time: one undo step per stroke, holding the final UVs. No file format change (format 87 stores `SetUvs`).
- The brush and the lattice (decision 0236) put each other down; both stop in Paint mode.

## Consequences

- Strength is fixed (0.3 a dab for Relax and Pinch); a setting can follow if wanted.
- A stroke works on every selected object's UVs under the brush, wherever the selection is.
- Keys that change the document (Delete, Ctrl + Z, Tab) wait while a stroke is under way, as they wait for a gizmo drag, and **Esc mid-stroke takes the stroke back** (the brush stays up; Esc between strokes still puts it down), as Esc cancels a gizmo drag (decision 0126). Only the left button brushes: the view's navigation works under the brush. A stroke touches only the pieces the editor shows (Isolate hides the rest), and picking a tool (W, R, S, T, Q) puts the brush down, as it puts a modelling tool away. (Fixed after the UV test pass; no new keys.)
