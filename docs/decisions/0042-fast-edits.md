# 0042. Fast edits

- **Status:** Accepted
- **Date:** 2026-10-13
- **Follows:** [0021](0021-object-stack-foundation.md), [0032](0032-edits-find-their-points-again.md), [0040](0040-bevel.md); from the [whole-app review](../reviews/2026-10-13-whole-app-review.md) (P1, P3–P6)

## Problem

The whole-app review measured five ways editing slowed down as a model grew:

- **P1, the history.** Every edit lives in one *Edits* layer, and the shape was cached per layer. So every drag frame, and every change in the Last edit panel, replayed the object's whole history from its base shape. A 3,000-polygon prop with 200 edits took about 26 ms per frame; a 3-million-polygon shape with five extrudes took 3.5 s per frame.
- **P3, bevel.** Two loops compared every strip with every other strip and every cut with every other cut. Bevelling every edge of a 32,000-polygon sphere took 65 s, and every point 5 minutes.
- **P4, finding edits again** (decision 0032). Each anchored element searched the whole mesh, so 1,000 points on a 3-million-polygon shape took 19 s. On dense meshes, "the same place" was also judged against the whole shape's size, so one polygon could pick up its neighbours.
- **P5, undo memory.** A mesh's connections (its topology), often several times the mesh itself, weren't counted towards the undo budget.
- **P6, the end of a drag.** Hovering waited for a heavy shape's new picking index, and extruding edges worked out the whole shape's connections, on the interface thread, just as the drag ended.

## Decision

1. **Checkpoints inside a layer.** The cache keeps, for each layer, the shape after all its edits and the shape after all but the last. When the layer changes, evaluation starts from the longest run of edits that is still the same, at the nearest kept shape. So:
   - adding an edit applies only that edit;
   - changing the last edit (a drag, the Last edit panel) applies only that edit;
   - undoing the last edit applies nothing;
   - changing an earlier edit, or a base setting, still replays from there, as it must.

   The extra kept shape shares its unchanged parts with the others, and is kept only while the object's cache is.
2. **Bevel without quadratic loops.** Rows are found by their two ends in a lookup table, a point's cuts by a range of a sorted map, and points no polygon uses any more are removed in one pass (`Mesh::remove_unused_vertices`).
3. **Finding edits again through a grid.** Points are sorted into equal boxes first, so each anchored element looks only at the points near it. Edges and polygons also tighten "the same place" to a quarter of their own shortest side, so on a dense mesh an anchored polygon finds only itself (and the pieces it was split into), not its neighbours.
4. **Undo memory counts the topology:** as it is once worked out, and as an estimate before then, since nearly every mesh gets one.
5. **Nothing waits as a drag ends.** Hovering skips a heavy shape whose index is still being built (it highlights again a moment later); a click still waits, since it needs the answer. Extruding edges finds the new outer edges by looking back from the newest polygons, without working out the shape's connections.
6. **An edit-speed check** (`crates/scene/tests/edit_speed.rs`) joins the heavy-mesh check, so these numbers are measured again at the end of each phase.

## Results

Release build, same machine, before (main) and after:

| | Before | After |
|---|---:|---:|
| Drag after 200 edits, 3k-polygon prop (per frame) | 25.9 ms | 0.07 ms |
| Drag an extrude after 5 extrudes, 3M polygons (per frame) | 3,514 ms | 35 ms |
| Bevel every edge of a 32k-polygon sphere | 64.9 s | 1.9 s |
| Bevel every point of a 32k-polygon sphere | 298 s | 0.1 s |
| Find 1,000 edited points again, 3M polygons | 19.3 s | 0.15 s |

## Consequences

- Drags and Last edit changes cost about the same whatever the history holds. Changing an early edit still replays everything above it; a future Edit Layers panel can split long histories into layers to keep that quick too.
- The cache holds one more shape per layer. Its unchanged parts are shared, and it is not part of the undo budget (the cache can be dropped and rebuilt at any time).
- Undo now reports more memory than before, closer to the truth, so on huge shapes the oldest steps are forgotten sooner.
- On a dense mesh, an edit may now find fewer elements than before where it used to find too many. That is the intended fix, and the Attributes panel says when an edit is missing elements, as before.
