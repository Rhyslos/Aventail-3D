# 0043. Attribute layers and the mesh builder

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0006](0006-base-plus-operation-stack.md), [0040](0040-bevel.md); from the [whole-app review](../reviews/2026-10-13-whole-app-review.md) (A1, A5)

## Problem

The mesh only knew where its points were, its polygons, and which edges were hard. Everything still to come needs more:
- material slots on polygons (Phase 5);
- UVs on polygon corners and seams on edges (Phase 8);
- vertex colours, creases, and later skin weights.

Each tool also carried hard edges over by hand (bevel had about 15 lines of it). Every new kind of value would have meant rewriting every tool.

There was also no way to delete a polygon. Delete, Dissolve, Weld, Bridge and splitting into an object all need it.

## Decision

1. **Attribute layers** (`mesh::attributes`). A layer is a name, a domain and a default value.
   - The domain says what the layer holds a value for: each point, edge, polygon or polygon corner.
   - The default also gives the kind of value: on/off, a whole number, or 1–4 numbers.
   - Point, polygon and corner values are kept in typed lists by id, only as far as a value was set. Anything beyond is the default, so new points and polygons cost nothing until a value is set on them.
   - Edges have no ids, so edge values are kept by the edge's two points, and only where they differ from the default.
   - Layers are shared between copies of a mesh until one copy changes them, so a recorded move still copies only positions.
2. **Hard edges become the built-in edge layer `hard`.** `set_hard`, `is_hard` and `hard_edges` keep working as before. A mesh whose last hard edge is made soft equals one that never had any.
3. **The mesh builder** (`mesh::builder::Builder`). Tools now say where each new element comes from:
   - a new point copies another or lies between two;
   - a new polygon is *like* an original polygon;
   - each new corner copies or blends original corners;
   - edges are copied, moved or cleared with all their values.

   The builder fills in every layer the mesh has, including ones the tool doesn't know about. Extrude, inset, extrude edges and bevel (edges and points) use it:
   - walls and strips copy the polygon they grew from;
   - a bevel strip's corners blend the corners of the two polygons it runs between, so UVs will stretch across it instead of being lost;
   - edges that slide along take their seams with them as well as their hardness.
4. **Deleting polygons** (`Mesh::delete_faces`). Ids are never reused. Points stay; `remove_unused_vertices` deletes the leftovers. Corner values of the remaining polygons are kept. Edge values go only from edges no polygon uses any more.

Also fixed while measuring: bevel still checked every chosen edge for every point it touched. Bevelling every edge of a 32,000-polygon sphere went from 2 s to 0.34 s (the builder itself adds about a tenth).

## Consequences

- Apart from faster big bevels, nothing changes for users yet. Hard edges behave exactly as before, and every existing test passes unchanged.
- New tools (Loop cut, Slit, Bridge, Weld and the rest of Phase 4) are written against the builder from the start, so UVs and materials won't mean rewriting them.
- Layers other than `hard` aren't saved in project files yet, and nothing in the app creates them yet. Saving them comes with the next file format version, in the second part of Foundations 3.
- Corner values are blended in a straight line. Bevel strips with many rows get evenly spread UVs, not ones following the curve. Good enough for now; the UV phase can refine it.
