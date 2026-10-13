# 0040. Bevel

- **Status:** Accepted
- **Date:** 2026-10-13
- **Follows:** [0038](0038-extrude-and-inset.md), [0039](0039-drags-follow-the-pull.md)

## Problem

Bevel is the third tool that adds geometry, and the hardest. Where bevelled edges end, the polygons around them have to be reshaped. Where several bevelled edges meet, a hole opens that has to be closed. It needed:
- controls;
- a geometric recipe that stays sound on cubes, cylinders, grids, n-gons and awkward input;
- a place in the edit stack, the Attributes panel and project files.

## Controls (asked first, per 0028)

Agreed with the project lead before building:

| Question | Choice |
|---|---|
| Setting the width | `V`, then **drag away from the selected edges** (or points) to widen and back to narrow, following "drags follow the pull" (0039). The drag is measured as how much further the mouse is from the nearest selected edge on screen than where it started |
| Segments (roundness) | **In the Attributes panel only**: set before the drag, changeable after. The mouse wheel during the drag was offered |
| Profile | A slider in the Attributes panel, round (100%) to flat (0%) |
| Last edit | Width, Segments and Profile stay changeable until the next edit, like Extrude |

How Cinema 4D, Blender and 3ds Max offer both a bevel *tool* and a bevel *modifier* was discussed. This milestone is the tool, recorded in the edit stack. A non-destructive bevel modifier by angle comes later, with the modifiers palette and the game bevel (Phase 7).

## Decision

**Edges** (`mesh::bevel::bevel_edges`, with a `BevelShape`: width, segments, profile):
- The polygons around each end of a bevelled edge are walked in order (a *fan*). Edges need a polygon on each side, and ends a single fan; others are skipped and counted.
- **Each polygon corner at a bevelled point moves:**
  - *between two bevelled edges*, to where both offsets meet, `width` from each (in the polygon's plane);
  - *between a bevelled and a plain edge*, it slides along the plain edge until it is `width` from the bevelled one (at most 95% of the edge);
  - *between two plain edges*, it takes the slides on either side, and keeps the point itself unless both sides slid.
- **A strip** of `segments` rows replaces each bevelled edge. The rows follow a quadratic curve from one side to the other, pulled towards the original corner by `profile`. 0 gives a straight line, a chamfer.
- **Where strips end:**
  - strips ending between the same two points (round a cube's top corner) share their end rows;
  - when only one polygon borders the hole (a single bevelled edge reaching a cube's corner), that polygon takes the strip's end, as in other programs;
  - otherwise one corner polygon closes the hole (three edges at a cube's corner). With several segments this is one n-gon; a rounded corner patch comes with the improved bevel (Phase 7).
- **Ids:** polygons keep their ids and get new corners (`Mesh::rewrite_faces`). New points and polygons take the next ids in a fixed order. Points no polygon uses any more are deleted (`Mesh::remove_unused_vertex`, which also drops their hard edges).
- **Hard edges:** a hard bevelled edge's strip keeps hard borders. With one segment, its ends and corner polygons are hard too, so a chamfered cube stays crisp. Rows inside a rounded strip are soft. Plain edges that slid keep their hardness.
- The work is done on a copy and swapped in at the end, so a failure leaves the mesh as it was.

**Points** (`bevel_points`): a point is put `width` along each of the point's edges (at most 95%), polygons take the two cuts in place of the point, and one polygon closes the corner.

**Scene:**
- `Operation::BevelEdges` and `BevelPoints`, with anchors (0032).
- `EditSettings` with `Operation::settings` and `Operation::with`: the changeable settings of an extrude, inset or bevel (distance or width, grouping, segments, profile). These generalise 0038's `distance`/`grouping`/`with_settings`, which remain as shorthands.

**Interface:**
- `Tool::Bevel` on `V` (Edges and Points modes).
- The drag is measured from the selected edges on screen (points count as edges of no length).
- The Attributes panel shows Segments and Profile as the tool's settings, and Width, Segments and Profile on the Last edit. Changes there also become the next drag's settings.
- The tool's own section hides only while the last edit is the same kind of tool's.

**Project files:** format **version 4** (`project/v4.rs`), following 0031's plan.
- `OperationV4` wraps version 3's operations (`V3(...)`) and adds the two bevels.
- Widths and profiles must be numbers. Rows are brought within 1–64 and profiles within 0–1.
- Versions 1–3 still read; `sample-v4.avtl` is committed with a test.

**Fixed along the way:** a mesh worked out its topology once and kept it with its faces. But adding a polygon to a mesh whose face list wasn't shared kept the old topology. Adding or changing faces now always clears it, with a regression test.

## Consequences

- Wide bevels can cross over neighbouring polygons. Clamping, mitre patterns and rounded corner patches belong to the improved bevel in Phase 7.
- Edges on an open border can't be bevelled yet.
- After an edge bevel the selected edges no longer exist, so the selection empties; the Last edit is how the bevel is adjusted.
