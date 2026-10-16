# 0104. UVs that survive modelling

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0095](0095-uvs-before-advanced-modelling.md) (the UV phase plan), [0043](0043-attribute-layers-and-the-builder.md) (the builder carries values), [0032](0032-edits-find-their-points-again.md) (edits find their elements again), [0099](0099-unwrap-relax-straighten.md) (unwrapping)

## Problem

The roadmap promises "change the model, keep the UVs": model and UV in any order, so a late bevel or extrude doesn't mean redoing the UVs. Two cases:

1. **The shape changes under recorded UV work** (a cube given more segments, an edit changed earlier in the list). UV edits are recorded like any edit and find their polygons again by their anchors, so they already replay on the new shape. Tests now prove it.
2. **An edit is made after the UV work.** The builder carries UVs over to what an edit makes, and some edits come out right that way (a loop cut blends them). Others don't. Measured on a cube:
   - an extrude's walls, an inset's ring, a filled hole and a bevelled corner start with **no room** on the texture (squashed flat);
   - a bevel **lost the seam** on the bevelled edge, merging two pieces;
   - a bevelled corner gave its neighbours two corners with the same UV.

## Options

- **When to fix:**
  - **automatically, as part of every edit that adds, removes or rewires polygons** (chosen). It is worked out from the edit's result each time the shape is evaluated, so it replays, stays undoable with the edit, and follows the edit's settings when they change in the edit list. No new operation or file format change.
  - a separate "fix UVs" command: one more thing to remember, and broken UVs in between.
  - a switch to turn it off: nothing is lost by mending what was squashed flat, and only polygons the edit changed are touched, so there's no switch.
- **Which polygons:** only those the edit made or changed (a polygon is changed when its corner list differs), and of those only the **broken** ones. A polygon is broken when it is:
  - squashed flat (no room);
  - mirrored against the polygons around it;
  - stretched 3 times more one way than the other;
  - 3 times bigger or smaller (in length) than the polygons around it.

  Polygons a user squashed on purpose elsewhere are never touched. Moving points never mends (the texture stretches as the shape does, as in every program); the Stretch view shows it and Relax or Unwrap fixes it.
- **How to fix:**
  - **solve the broken polygons around the kept ones** (chosen): least squares conformal maps with every kept corner held where it is. A cluster of broken polygons carries on from the kept polygons next to it, so hand placement, turns and scaling are kept exactly.
  - a cluster touching kept polygons of several pieces that meet only through it (a bevelled corner between three pieces; walls between a cap and what surrounds it) can't agree with all of them. It joins the one it shares the longest border with, and seams part it from the rest.
  - a cluster with nothing kept next to it (an extrude's ring of walls, cut off by seams the builder copies), or one that still lies badly after solving, is laid out as a piece of its own, as Unwrap does (strips straight, at the shape's usual density). It is then slid inside the texture square if it sticks out.
  - re-unwrapping whole pieces: simpler, but would undo hand work.
- **Seams:** wherever two changed, unbroken polygons now meet with different UVs, a seam is marked so pieces stay apart. A shape with no seam marks (imported) first gets them where its UVs split. Bevel moves a bevelled seam to one border of its strip. A bevelled corner's neighbours now blend their UVs along their edges.
- **Speed:** every step looks only near what changed (polygons around a cluster are sampled; whether kept polygons are joined is found by a search that stops as soon as it can tell). In a release build, one extrude on a 40,000-polygon plane mends in about 15 ms, and 4,000 separate extrudes in about 0.4 s. Past 20,000 broken polygons (`MEND_UP_TO`) nothing is mended; Unwrap fixes them.

## Decision

1. `uv::mend` (`mend(before, after)`): the steps above, with `uv::unwrap::conformal_pinned` (any wedges held) and `lay_out` split out of `unwrap`.
2. `scene::Operation::mends_uvs` lists the operations that mend: extrudes, inset, bevels, groove, loop cut, connect, bridge, weld, delete, dissolve, fill holes, slit and the mirror's weld. Not:
   - moving, sliding or placing points;
   - flips;
   - material, hardness, seam and UV edits.

   `Operation::apply` mends after those (the mirror's sides each mend as they are applied).
3. `mesh::bevel`: a bevelled seam moves to the strip's far border; a bevelled corner's neighbours blend their UVs along their edges.

## Consequences

- Extrude and bevel on unwrapped shapes give usable UVs straight away. New pieces may overlap others; Pack arranges them.
- Projects made on earlier test builds that recorded edits after UV work will show mended UVs where they had squashed ones.
- A new modelling operation must be added to `mends_uvs` (the match lists every operation, so the compiler asks).
