# 0240. Packing control: groups, locked space, regions, grain, scale, UDIM

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 12. Packing fitted every selected piece into the whole square at one texel density, turned freely, around nothing: locked pieces were skipped but could be packed over, parts that belong together drifted apart, wood grain got turned, important parts couldn't get more texture, and very big models couldn't spread over several textures.

## Decision

- **`uv::pack::pack_with(pieces, asks, filled, options)`** (`pack` is it with nothing extra): `PieceAsks { grain, scale }` per piece (grain: no free or quarter turns; scale: the piece is sized by it before the common scale, so it gets that much more texture), `filled` pieces (locked ones) drawn onto the grid before anything is placed, `PackOptions::region` (packing space mapped into the region, kept square) and `PackOptions::tiles` (UDIM: the grid is that many squares wide, with the sides between tiles blocked so no piece straddles one).
- **Packing groups**: the workbench merges pieces of one group (within one texture) into one piece for packing, so they keep their places to each other, and gives them the same move.
- **Per piece, saved with the shape**: polygon layers `uv_pack_group`, `uv_grain` (0 the app's, 1 kept, 2 free) and `uv_pack_scale` (`mesh::uvs::PackValues`, `pack_values`, `set_pack_values` with `PackParts` for which to set), recorded by `Operation::SetPackValues`; **file format 88** stores it (`v88::PackEditV88`). "Per-object density" is the texture scale given to a whole object's pieces: one setting for pieces and objects alike.
- **In the app's settings** (UV → Settings → Packing, remembered): Keep the grain (the standard for pieces without their own), Pack into a region (numbers for its corners; drawing it in the UV Editor can follow), UDIM tiles across (1 to 10).
- **UV → Pieces → Packing for the selection** sets a group, a grain and a texture scale on the whole pieces holding the selection, one undo step each. No new keys.

## Consequences

- UVs past 1 (UDIM) are exported as they are; the manual says Godot and Substance Painter read them by tile.
- A group's pieces are packed as they lie, so lay them out together first.
- **Regions, after the UV test pass (2026-11-06):** a region is no longer "kept square". The packing grid is laid over it in texture units, with cells of one size, so the padding is the same texture pixels as in the whole square and a wide or tall region is filled along its whole length. Its ends may be given either way round (`pack::region_of` puts them in order); a region with no width or height packs into the whole square, and the app's message says so. With UDIM tiles, the same region of each tile is used.
