# mesh

**Layer 0 · Geometry**

Polygon meshes: the data structure, topology queries and modelling operations.

## Purpose

The foundation everything else builds on: an n-gon mesh with full adjacency (vertices, edges, faces, face corners), stable element IDs, typed attribute layers (hard edges, seams, UVs, later skin weights), and every topology operation as a pure function (extrude, inset, bevel, loop cut, bridge, weld and so on). See DESIGN.md §4.1.

## Must never

- Know about objects, scenes, undo, files, the GPU or UI
- Change a mesh in place behind the caller's back: operations take a mesh and return a result
- Reuse a deleted element's ID
- Crash or allocate without bound on degenerate input (zero-length edges, n-gons, open boundaries, non-manifold geometry)

## Depends on

No other app crates. Outside libraries: `glam` (maths).

## Status

- **`Bounds`**: an upright bounding box (`new`, `around` points, `union`, `centre`, `size`, `radius`, `contains`), used for framing the view. Skips points that aren't real numbers.

The mesh data structure arrives in Phase 3.
