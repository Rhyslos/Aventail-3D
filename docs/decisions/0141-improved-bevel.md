# 0141. Improved bevel: rounded corners and stopping at neighbours

- **Status:** Accepted
- **Date:** 2026-10-24
- **Builds on:** [0040](0040-bevel.md) (bevel), [0043](0043-attribute-layers-and-the-builder.md) (the builder carries UVs and seams), [0048](0048-ctrl-wheel-changes-a-tools-count.md) (Ctrl + wheel for segments), [0085](0085-bevel-control.md) (even width, skew, stop at overlaps, weld), [0137](0137-biggest-first.md) (Phase 10's order)
- **Replaces part of:** [0085](0085-bevel-control.md) (Stop at overlaps narrowed the whole bevel; for new bevels it narrows each point only as much as it needs, and is called Stop at neighbours)

## Problem

Phase 10's third milestone: "rounded corners where bevels meet, and bevels that never overlap". The project lead was asleep when it was built, so the choices below are the recommended ones, to be checked.

What the bevel did before this decision:

- **Corners.** Where three or more bevelled edges meet at a point (a cube's corner), each strip ends with a row of points across it, and the hole left between the rows was closed with **one polygon** through all of them: a triangle with 1 segment, a 3 × segments-cornered polygon with more (9 corners for 3 segments). With several segments it was flat across a round bevel and bent (its corners don't lie in one plane), so it shaded badly, was cut into triangles any old way on export, and the strips' end rows leaned towards the corner point. Two edges turning a corner along a loop (a cube's top ring) already met in a clean mitre, the two strips sharing one row; one edge ending at a corner already let the polygon at its end take the strip's end.
- **Overlaps.** With Stop at overlaps on (0085), the bevel worked out, for every corner, how far it could slide before passing the far end of an edge (95% along a plain edge, halfway towards another bevelled point), and narrowed **the whole bevel** by the tightest one. One thin part made every other edge's bevel thin too. With it off, a wide bevel's corners pass each other and fold over.

## Options

For the corner:
1. **A patch of quads in the hole, cut into one region per corner of the hole, meeting in the middle (chosen).** It works for any number of edges: a grid of quads per region for a cube's 3-way corner, the same regions round a centre (a polar patch) for a pyramid's tip of 4, 5 or 16 edges. This is how Blender behaves (a grid for 3-way corners, a polar fan for more); none of its code was used.
2. A sphere fitted to the corner: exact on a cube, but nothing fits a pyramid's tip or a slanted corner.
3. One polygon as before, cut into triangles: still flat on a round bevel.

For the patch's shape, each region is a **Coons patch** spanned between its four borders: half of each side (the strips' end rows, exactly) and two *spokes* from the sides' middles to a centre. The spokes leave each side along its strip (so the surface carries on smoothly) and bend over to arrive level with the centre; the centre is placed as a quadratic triangle's middle (a third its corners, two thirds its sides' controls). For a flat profile all of it lies flat; for a hollow one it curves the other way.

For stopping at overlaps:
1. **Point by point (chosen, as the milestone asked):** each bevelled point is narrowed only as much as its own corners need. A strip between a tight point and a free one tapers from one width to the other. 0085 had turned this down because it leaves a bevel uneven; the trade is now worth it, because a whole bevel shrinking to a sliver for one thin part was the bigger surprise.
2. The whole bevel narrowed by the tightest point (0085's way), kept for bevels made before.

For bevels already in projects:
1. **They keep their old shape when opened (chosen).** Projects mustn't change shape on opening. A bevel saved before stays as it was until a person changes one of its settings under Last edit (or Ctrl + wheel changes its segments): then it is made the improved way, like a new one.
2. Improve every bevel on opening: rounder corners at once, but every older project's bevelled shapes, polygon counts and UVs would change without a word.

## Decision

1. **`BevelOptions::improved`** (new bevels: on; `BevelOptions::EARLIER` and every bevel read from a file before version 42: off). The groove (0086) keeps its bevel as it was.
2. **Rounded corners** (`mesh::bevel::corner`), only when improved, with 2 or more segments, and when every side of the hole is a strip's end row (`round_sides`: the hole's points and rows take turns, three rows or more, no point twice):
   - Each side row's inner points are reshaped to bend towards the bevelled edge beside them (the point on the edge's line level with the row's middle) instead of towards the corner point, so the row lies across its strip.
   - With an even number of segments 2m: one region of m × m quads per side, all meeting at a centre point. With an odd number 2m + 1: the regions, a strip of m quads between each two (from each side's middle segment) and a small polygon in the middle with a corner per side (a triangle on a cube). With 3 segments on a cube: 6 quads and a triangle per corner.
   - Every new point copies the bevelled point's values; each region's corners copy the corner of the polygon its hole corner came from, and each region is like that polygon (material, smoothing), so UVs come along as for the corner polygon before (decision 0043) and a corner touching a top of another material takes it there.
   - Hard edges follow 0040's rules: inside a rounded corner every edge is soft, as a rounded strip's inner rows are.
   - With 1 segment, or a hole with plain sides as well (two plain edges side by side round the point), the corner is one flat polygon as before. One segment's corner is exactly the old triangle.
   - New points and polygons are made in a fixed order (centre, spokes, each region), so the same bevel always gives the same ids.
3. **Stop at neighbours.** `plan` now finds how much narrower the bevel must be at each bevelled point (`fits`). Improved: each point is narrowed by its own amount. Not improved: the whole bevel by the smallest, exactly as before. Corners still meet halfway along an edge between two bevelled points and stop at 95% of a plain edge, so they never pass each other or the far end.
4. **The Attributes panel's switch is called Stop at neighbours** (it was Stop at overlaps; the field is still `clamp`), on as standard, with a tooltip saying only the points that need it are narrowed. Switching it off gives the old free bevel that can fold over.
5. **Old bevels improve when changed:** `EditSettings::improved` sets the flag; the Last edit rows use it when anything changes, and so does Ctrl + wheel on the last bevel. The next bevel's settings never inherit an old bevel's "not improved".
6. **File format 42.** Version 41's model with one more kind of recorded edit, `OperationV23::Improved` (stored as `OperationV23::Bevel` is), added at the end of version 23's list so every older version's files still read. A version of its own, because an app reading up to 41 would fail on the unknown edit; with 42 it says it needs updating. Files of version 41 and before never hold the new kind, and their bevels open not improved. `sample-v42.avtl` is kept, with round trip, older-file and random-body tests.

No new keys or mouse actions. Ctrl + wheel still changes a bevel's segments (0048).

**Faster along the way.** A bevelled seam's new seam was marked one edge at a time, each working out the whole shape's connections again; they are now marked together. Bevelling every edge of the edit-speed check's 32,000-polygon sphere went from about 4.1 s to 1.2 s (release build), improved corners included.

## Consequences

- A cube bevelled with segments now has smooth, ball-like corners made of quads, which shade and export cleanly; with 1 segment nothing changes.
- A bevel next to a thin part stays full width elsewhere; where it is narrowed, its strip tapers.
- Bevels in older projects look as they did until changed; a changed one may change shape at its corners and where it was narrowed.
- Projects with improved bevels need this version of the app or later.
- Corners where plain edges sit side by side among bevelled ones are still one polygon; Blender's terminal-edge and mixed patterns could come later.
- The profile's curve is still the app's quadratic (a parabola, not an exact circle), so a "round" corner is within a few percent of a ball, not exactly one.
