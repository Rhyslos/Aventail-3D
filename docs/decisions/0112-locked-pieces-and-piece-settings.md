# 0112. Locked pieces and piece settings

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0108](0108-uv-powerhouse-and-baking.md) (the Phase 8 plan), [0109](0109-pins-and-unwrap-methods.md) (unwrap methods), [0110](0110-controllable-automation.md) (controllable automation)

## Problem

Phase 8's fourth milestone:

- **Lock a piece:** nothing automatic changes it.
- **Per-piece settings** (method, straightening, density), kept for every re-unwrap.

## Options

- **Controls:** menu commands only (no keys or mouse actions; the E ring is full). **Lock pieces** and **Unlock pieces** go in the UV menu after Pin and Unpin, with a **Piece settings** submenu.
- **What a lock is:** a polygon layer, `uv_lock`.
  - Lock and settings go on whole pieces (the pieces holding the selection), so a piece is never half locked.
  - Polygons made from locked ones by modelling are locked too: the builder copies polygon values.
- **What leaves a locked piece alone:**
  - **every tool that works on many pieces at once** (chosen): Unwrap and live unwrapping, Relax, Straighten, Set and Match texel density and Put on trim row (all through `uv::unwrap::chosen_pieces` and `uv::density`), Pack (skips locked islands), Suggest seams (keeps the seams on edges touching locked polygons) and mending (skips locked polygons).
  - Hand edits (the gizmo, turn and flip, pins) still work. Locking protects against the automatic tools, not against you.
- **Where piece settings are kept:** three polygon layers.
  - `uv_method`: 0 for the app's, then 1 to 4.
  - `uv_strips`: 0 for the app's, 1 to straighten, 2 not to.
  - `uv_density`: texture widths per scene unit, 0 for the app's.
  - A piece takes each from its first polygon (by id) that has one.
  - The density is stored in the texture's units, as `SetTexelDensity` records it, so the piece keeps its size on the texture when the texture size changes.
- **How settings apply:** `uv::PieceSettings::over` puts the piece's method and straightening over the unwrap's options for that piece. Its density replaces the one the unwrap would match.
  - They live on the shape, not in the unwrap edit, so an unwrap replays the same: decision 0110 holds.
- **Drawing:** locked polygons get faint grey stripes in the UV Editor. Nothing changes in the viewport.

## Decision

1. `mesh`:
   - `attributes::UV_LOCK`, `UV_METHOD`, `UV_STRIPS` and `UV_DENSITY`;
   - `uvs::locked`, `any_locked` and `set_locked`;
   - `uvs::PieceValues`, `piece_values` and `set_piece_values`.
2. `uv`:
   - `PieceSettings { method, strips, density }` (`of`, `from_values`, `values`, `over`, `is_empty`, `is_real`);
   - `chosen_pieces` and `density::set_density` skip locked pieces, and `mend_with` skips locked polygons;
   - `lay_out` uses each piece's settings.
3. `scene`:
   - `Operation::SetUvLock { faces, lock, anchors }` and `SetPieceSettings { faces, settings, anchors }` (neither is mirrored);
   - `numbers_are_real` checks the density.
4. File format 37: `OperationV37`.
5. `ui`:
   - `workbench::piece_settings`: `run_lock`, `run_piece_settings`, `piece_settings_menu` and `chosen_pieces`;
   - `UvSettings::piece_method`, `piece_strips` and `piece_density` (px/m);
   - Pack skips locked islands and says so; Suggest seams keeps locked pieces' seams;
   - the UV Editor's stripes (`theme::UV_LOCKED`).

## Consequences

- No keys.
- Packing *around* locked pieces (leaving their space free) comes with milestone 12. For now, Pack leaves them where they are and may lay other pieces over them.
