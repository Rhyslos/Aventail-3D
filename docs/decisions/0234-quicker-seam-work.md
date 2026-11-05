# 0234. Quicker seam work: a slider, a brush, loops, rings and mirroring

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 6. Seams could only be marked on a selection, suggested all at once, or taken from hard edges. Finding the right number of seams meant changing two settings (corner angle, stretch budget) and running Suggest seams again each time; painting seams meant selecting with the brush and then marking; a cylinder's length or a strip's cuts were edge-by-edge work; and a symmetrical shape made without live symmetry had to be seamed twice.

## Decision

Everything is in **UV → Seams**, with no keys of its own (the project lead's choice, 2026-11-05):

- **Fewer seams ↔ less stretch slider** (`uv::seam_tools::balanced`): one number from 0 to 1. Below the middle the corner angle rises from the standard 66° to 180° and there is no budget; above it the corner angle falls to 30° and the stretch budget tightens from 4× to 1.15× (evenly on a log scale). The middle gives the standard settings. Dragging sets those two settings and re-runs Suggest seams on the selected objects; each new place takes back the last one (`Document::abandon`) and records again under the same undo key, so the drag is one undo step and the seams are seen live. Where the slider was left is remembered with the UV settings (`seam_balance`).
- **Seam brush**: picked from the menu, it uses the selection brush's circle and dabs. While it is in use the workbench takes the viewport's brush gestures and clicks before selecting does, finds the visible edges under them, and records one `SetSeams` per object for the stroke so far, replacing the previous dab's (one undo step per stroke). The controls are the ones chosen for UV brushes: Ctrl + drag clears, `[` and `]` size it (as Ctrl + wheel does), Esc puts it down. Ctrl is read as the stroke starts. With Unwrap as seams change on, the pieces around the stroke's edges are unwrapped once, when the stroke ends, in the same step. Leaving the Edges mode, or choosing another select tool or a gizmo, puts it down.
- **Mark seam loop / Mark seam ring**: the selected edges widened with `Topology::edge_loop` / `edge_ring` (the walkers a double-click selects with), marked as seams.
- **Mirror seams**: each seam's mirror across the object's own X axis through its origin, found by position (`mesh::symmetry::Partners`), is marked; seams without a match, and edges marked "never" (decision 0111), are skipped. Shapes with Symmetry on mirror their seams already (`SetSeams` is mirrored, decision 0059).

No new operations and no file format change: everything records `SetSeams` (and Unwrap).

## Consequences

- The slider, like Suggest seams, replaces the selected objects' seams; the manual says to prefer or lock what should stay.
- Mirror seams only works across X for now; other axes can be added to the menu if wanted.
- `Document::last_merge_key` is public, so a tool can add to its own step only while it is still the last one.
