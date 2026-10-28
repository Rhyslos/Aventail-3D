# 0188. Speed with big textures

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's tenth milestone ([0177](0177-texture-painter-plan.md)): a 4K texture set (4096 × 4096 pixels) of 20 layers must stay smooth to paint, undo must stay within its memory budget ([0022](0022-undo-memory-budget.md)), and the heavy-mesh check ([0033](0033-heavy-mesh-safeguard.md)) must cover painting.

A new check measured it first. On the test machine (two cores) the first frame took 4.5 s, a frame where nothing changed 0.4 s, each frame of a stroke 1.6 s, and undo 1.7 s.

## What was slow, and what was done

- **Counting undo memory compared every tile with every other** (tens of thousands of tiles, so billions of comparisons a frame while painting). Now two sets laid out alike are compared layer by layer and tile by tile, and anything else is counted by address in one pass. Tiles in groups and masks, and the stroke records kept with layers, now count too.
- **The Paint layers panel tried Merge down every frame** to decide whether its button could be pressed, which mixes two whole layers, and copied the whole set for every button. Merge down now has a quick check (`TextureSet::can_merge_down`); the other buttons' states are kept while the layers are laid out the same; the panel no longer copies the set.
- **Mixing the layers** (the picture the views show):
  - Only tiles whose layers changed are looked at, compared with the layers directly instead of making a copy of each tile's inputs.
  - Each tile keeps the mix of the layers under the one that last changed there, so painting on one layer mixes only from it up. When the layers from there up were empty, the tile shown already is that mix.
  - Many tiles are mixed on all the processor's cores at once (`std::thread::scope`; no new dependency).
  - Laying a layer over nothing, or an opaque normal layer over anything, takes a short cut.
- **Painting dabs:**
  - All the dabs of a frame are painted together, shared out over the cores by rows of tiles. Each core paints every dab on its own rows, in order, so the result is the same as one by one; threads are started once a frame. Smudge still paints dab by dab, as each picks up what the last laid.
  - Along each row, only the texels inside the triangle are visited. Their weights, place and place on screen come from sums made once per triangle.
  - Triangles facing away are skipped whole, and a texel already covered as much as the dab could cover it is skipped first.
  - How much each texel is covered is kept in a list by tile instead of a hash map, and the depth map's neighbourhood is worked out once a stroke.

## Decision

- **The heavy painting check** (`workbench::heavy_paint_check`, run by `tools/heavy/heavy-check.sh` beside the heavy-mesh check, nightly and at phase ends) paints a 4K set of 20 layers on the cube in the real interface. Its targets allow for a two-core machine:

  | Step | Target |
  |---|---|
  | First frame (mixing every layer) | 3 s |
  | A frame where nothing changes | 16 ms |
  | Start, each frame and end of a stroke with a 20-point brush | 100, 50 and 50 ms |
  | The same with a 200-point brush, as big as the cube | 500, 150 and 50 ms |
  | Undo, redo | 150 ms each |

  It also checks that 22 strokes add less than two layers' worth of memory to the history.
- `HEAVY_TEXTURE` and `HEAVY_LAYERS` change the set's size; `HEAVY_REPEAT=radius:count` repeats strokes for profiling.
- **Undo** keeps only the tiles a stroke changed, before and after, inside the existing budget (2 GB as standard, Edit → Undo memory). No separate budget for painting.

## Consequences

- On the test machine, a frame of a 20-point stroke now averages about 25 ms, undo about 70 ms, and a frame where nothing changes about 2 ms. Computers with more cores do better, as mixing and painting are shared out.
- Starting a stroke with a brush as big as the whole object on a 4K set still takes about a quarter of a second on two cores: it reaches millions of texels. Painting on the graphics card would be the next step if that needs to be quicker; it isn't planned yet.
- The first time a set is shown, every layer is mixed: about two seconds for 4K × 20 layers on two cores.
