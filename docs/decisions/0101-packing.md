# 0101. Packing: shapes into the square, with real-pixel padding

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0095](0095-uvs-before-advanced-modelling.md) (the UV phase plan), [0100](0100-texture-size-and-texel-density.md) (texture size, so padding is in pixels), [0099](0099-unwrap-relax-straighten.md) (pieces at one texel density)

## Problem

After unwrapping, pieces sit where they were and can overlap. Packing fits every piece into the texture square as large as it can go, at one shared scale (so texel densities stay matched), with a gap in real texture pixels so colours don't bleed when the texture is shrunk (mipmaps). The roadmap asks for nesting (small pieces in the gaps and holes of big ones), groups and texture sets, and stacking identical parts.

## Options

- **How pieces are placed:**
  - **by their shapes, on a grid of cells** (chosen): each piece is drawn into cells of a few texture pixels (half the padding, from 128 to 512 cells across), grown by half the padding, and placed at the lowest, then leftmost, spot where it touches nothing. Shapes, not boxes, so a small piece can sit in the dent or hole of a big one (**nesting**). The largest scale that fits is found by halving the gap between one that fits and one that doesn't. About 0.1 s for 20 pieces, 0.3 s for 100 and 1 s for 300 in a release build;
  - rectangle packing of the pieces' boxes: quicker, but wastes the space around round and L-shaped pieces and can't nest;
  - xatlas's packer (C++): a large dependency.
- **Turning:** each piece is first turned so its box is smallest, then tried at a quarter turn as well (**Turn pieces to fit**, on as standard).
- **Padding:** in **texture pixels** (8 as standard, up to 256), between pieces, and half of it to the square's edges. Remembered with the other UV settings (it's a choice made when packing; the packed result is saved in the edit).
- **Texture sets and groups:**
  - **A texture per material** (on as standard): each material's pieces fill a square of their own, as Substance Painter's texture sets expect;
  - **A texture per object** (off): each object's pieces fill their own square.
  - Otherwise every selected object's pieces share one square.
- **Stacking** (**Stack identical pieces**, off as standard): pieces of exactly the same shape (allowing a half turn) are packed once, and the copies laid exactly on top, sharing pixels. Off as standard because unique pixels bake cleaner ambient occlusion.
- **What is recorded:** one edit per object, `PackUvs`, listing each island's polygons and its move (turn, scale and place): it means the same when replayed, whatever else was packed with it. It has no anchors: polygons are named by their ids, which edits keep stable.
- **Where:** **Pack** in the UV ring (E over the UV Editor, and in the UV mode) and the UV menu; **UV → Packing** holds the padding and switches. No keys.

## Decision

1. `uv::pack`: `Piece`, `PackOptions` (padding in texture widths, rotate, stack, cells), `pack` (the moves, or `None` with nothing to pack), `islands_of`, `piece_of`, `place` (moves exactly the islands' corners). `uv::unwrap::smallest_box_turn` is shared.
2. `scene::Operation::PackUvs { islands }`; not mirrored.
3. **File format version 32** (`project/v32.rs`): `OperationV32::PackUvs` (each island's polygon ids and six real numbers). `ProjectV31` is now generic over its edits. Sample in `tests/data/sample-v32.avtl`.
4. UI: `UvSettings` gains `padding`, `rotate`, `stack`, `per_material`, `per_object` (remembered; older settings get the standards); `Workbench::run_pack`, `packing_menu`; Pack built in the command list (every UV tool in the ring is now built).

## Consequences

- A prop can be unwrapped and packed in the app, ready for baking.
- Big texel-density differences between objects packed together stay as they are: Set or Match texel density first.
- Packing runs on the interface thread; a very large set of pieces makes the window wait for a second or two.
