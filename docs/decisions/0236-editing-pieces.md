# 0236. Editing pieces: stitch, cut, align, orient, lattice

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 8. Pieces could be moved, turned, scaled and flipped by hand, and re-cut by marking seams and unwrapping, but not joined back along an edge, cut free in place, lined up, squared to an edge or bent.

## Decision

- **`Operation::SetUvs { corners }`**: polygon corners and their UVs, set as recorded (corners no longer in the shape are left out). Every tool here records its result as one `SetUvs` per object (with `SetSeams` where seams change), so undo, replay and the edit list work as for other edits. **File format 87** stores it (`v87::UvsEditV87`; a UV that isn't a number is refused). Milestones 10 (brushes) and 11 (copy UVs) can record with it too.
- **`uv::pieces`** works out the corners, never changing the mesh: `stitch` (for each edge cut in the texture, the smaller piece by polygon count, never a locked one, is moved by the similarity taking its side of the edge onto the other's; the corners at the edge's ends, and those sitting with them round each point, are made one; later edges between the same pieces are only joined), `cut` (the smaller part a cut parts off is nudged away from the cut's middle; slits move nothing), `align` (sides, middles, or spread evenly by middles, three or more pieces), `orient` (the smallest turn about the piece's box middle that makes an edge straight across or up), and the lattice (`lattice_grid`, `warp_point`: piecewise bilinear over an n × n grid, the nearest cell carrying on outside).
- **Controls** (chosen by the project lead): **C** stitches and **X** cuts, only while the mouse is over the UV Editor (taken by the workbench before the viewport's C for Bridge and X for Slit); both are in the E ring's **Stitch & cut** group, in the UV ring's last place. Align and spread, Orient by edge and Lattice warp are in UV → Pieces with no keys. The lattice is 3 × 3 handles drawn over the UV Editor; each drag is one undo step (re-recorded under one key as it moves); Esc or **Done warping** puts it away.
- In the Polygons and UV modes, Stitch uses every edge of the selected polygons and Cut the border of the selection.

## Consequences

- A stitch can leave the moved piece overlapping others; Pack fixes it.
- The cut's nudge is two texture pixels, enough to see and grab the part.
- The lattice's size is fixed at 3 × 3 for now; a 4 × 4 choice can be added to its menu entry later.
