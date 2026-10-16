# 0105. Bake readiness for UVs

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0077](0077-bake-check.md) (the bake check, model part), [0101](0101-packing.md) (packing), [0100](0100-texture-size-and-texel-density.md) (texture size and density), [0102](0102-checker-and-stretch.md) (stretch)

## Problem

The UV phase promises: "if the UV editor is happy, the bake is clean" (DESIGN.md §7, wish W13). The bake check (0077) lists what in a model would spoil a bake, but not yet its UVs. Problems that spoil a bake through the UVs:

- hard edges where the UVs don't split (a line along the edge in the normal map);
- polygons folded over or squashed flat;
- pieces outside the texture square;
- pieces lying over each other;
- pieces too close for the padding (they bleed in the bake's padding and in mipmaps);
- uneven texel density.

## Options

- **Where:** **in the same Check for baking window** (chosen), also opened from **UV → Check for baking…**, so one list covers the whole bake. Live markers in the UV Editor were set aside for now: checking overlaps draws every piece of a texture into a grid (tens of milliseconds), too much to redo on every frame of a drag.
- **Which pieces are checked together:** **those that share a texture, grouped exactly as Pack groups them** (chosen): a texture per material (on as standard) and per object (off as standard). Two props with the same material share one texture set in Substance Painter, so overlaps between them matter.
- **How overlaps and gaps are found:** **draw the pieces of a texture into a grid** of up to 1,024 cells across (a cell is a texture pixel or more).
  - A cell whose middle lies in two pieces is an overlap.
  - Gaps are measured by spreading each piece's cells outwards; where two spreads meet, the gap is the sum of how far each spread.
  - One cell of slack, so a freshly packed layout is never flagged.
  - Pieces exactly on an identical one (same corners, area and box) are **stacked**, told apart from overlaps.
  - Exact polygon intersection tests were set aside: slower, and a sub-pixel overlap doesn't matter to a bake.
- **The rules:**
  - hard edges as the export shades them (by angle while Automatic hard edges is on);
  - **folded:** no room, or mirrored against most of its piece (a whole mirrored piece is fine);
  - **outside:** any corner past the square;
  - **uneven:** a piece twice or half the shape's usual density, the area-weighted middle of its pieces'.
- **Fixes**, each one undo step:
  - **no UVs:** suggested seams, unwrap, then pack with whatever shares the texture;
  - **hard edges:** mark them as seams and unwrap the pieces around them;
  - **folded:** unwrap their pieces;
  - **outside, overlapping, too close:** pack everything sharing the texture;
  - **stacked:** pack too, unless **Stack identical pieces** is on (then it's on purpose: listed, explained, no fix);
  - **uneven:** set those pieces to the shape's usual density.
- **A fix that makes something else worse** (scaling pieces can make them overlap) says so in the status bar, as the model fixes do.

## Decision

1. `uv::check`:
   - `UvReport::new(mesh, hard)`: `hard_joined`, `folded`, `outside`, `uneven`, `usual_density`;
   - `check_sheet(pieces, size, padding)` → `SheetReport`: `overlapping`, `stacked`, `too_close`, by piece;
   - `RESOLUTION` (1,024) and `UNEVEN_DENSITY` (2).
2. `ui::workbench::baking`:
   - new `Kind`s: `NoUvs`, `HardJoined`, `Folded`, `Outside`, `Overlapping`, `Stacked(faces, on_purpose)`, `TooClose(faces, padding)`, `Uneven(faces, density)`;
   - fixes are a `Fix` (`Record`, `FirstUvs`, `Seams`, `Unwrap`, `Pack`, `Density`), turned into edits on the current shape when clicked;
   - `sheet_problems` groups pieces with `Workbench::texture_group` (shared with Pack);
   - the list is found again when the scene, the UV settings or the hard-edge angle change.
3. Pack is split into `run_pack` and `pack_shapes(shapes, whole, key)`. `record_each_in` lets a fix and a pack share one undo step.
4. UV → Check for baking… opens the window.

## Consequences

- No new keys, no file format change.
- Two objects made from the same primitive with the same material are listed as stacked until packed: true to what Substance Painter would bake.
- Live UV Editor markers can come later, from the same `uv::check`, if they can be made cheap enough.
