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

- **`Mesh`** (`polygon`): points and n-gon faces by id (`VertexId`, `FaceId`; ids count up and are never reused), and the set of hard edges (`EdgeKey`). `add_face` refuses faces with fewer than three corners, unknown points or repeated points (`MeshError`, in plain words). `edges`, `bounds` and `face_normal` (Newell's method, so it is sensible for n-gons; straight up for a face with no area). Full adjacency (loops, rings, walking from edge to face) arrives with mesh editing in Phase 3.
- **Shading** (`shading`): `corner_normals` gives each face corner the angle-weighted average of the faces joined to it across soft edges; `shaded_triangles` cuts faces into fans of triangles with those normals (right for convex faces; faces with dents need a proper cut, Phase 4).
- **Primitives** (`primitives`, [decision 0020](../../docs/decisions/0020-primitives.md)): `Primitive` (cube, plane, cylinder, sphere, disc, tube, cone) with per-kind settings and standard values, `PrimitiveKind` (the kind without settings, with name and description), `clamped` (sizes 1 mm to 10 km, segment and side limits, holes smaller than their shape), `mesh` and analytic `bounds`. Shapes are 1 m across as standard and stand on the ground with the pivot at the base centre; round ends are single n-gons as standard, or cut into rings (`cap_segments`, `ring_segments`) and/or met at a centre point (`cap_centre`, `centre_point`), built by `round_end`; round sides are soft and rims hard; the cube's twelve edges are hard.

Tests: faces and ids, normals across hard and soft edges, triangulation, and for every primitive: closed surfaces facing outward (or flat ones facing up), volumes close to the true shape, bounds, point and face counts, hard edges, smooth and crisp shading, silly settings (NaN, zero, huge, holes bigger than the shape), the 1 m standard sizes, end rings and centre points, determinism, and the largest allowed mesh.
