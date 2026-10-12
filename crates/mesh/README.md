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

- **`Mesh`** (`polygon`): points and n-gon faces by id (`VertexId`, `FaceId`; ids count up and are never reused), and the set of hard edges (`EdgeKey`). `add_face` refuses faces with fewer than three corners, unknown points or repeated points (`MeshError`, in plain words). `edges`, `bounds` and `face_normal` (Newell's method, so it is sensible for n-gons; straight up for a face with no area). Adjacency lives in `Topology`.
  - **Built for heavy meshes** ([decision 0035](../../docs/decisions/0035-heavy-meshes.md)): its parts are shared between copies until one changes them: positions in chunks of 4096 (each keeping the box around its points), faces as one flat list of corners with where each face starts (`face_layout`), hard edges. Copying is cheap, and a move copies only the chunks it touches. `same_structure` (the very same faces and hard edges) and `moved_since` (which points moved, comparing only chunks that aren't shared) let caches update only what changed. `topology` is worked out once per set of faces and kept with them; `edges` comes from it. `point_slots` and `face_slots` iterate the slots, `point_slot_count` counts them.
- **Samples** (`samples`): `heavy_ball(polygons, radius)`, a closed, bumpy ball of about that many polygons standing on the ground, for the heavy-mesh check and Help → Add a heavy test shape.
- **Parallel work** (`parallel`): `in_parallel(count, work)` splits work over the computer's cores when there is enough of it (used for shading and picking indexes).
- **Shading** (`shading`): each face corner's normal is the angle-weighted average of the faces joined to it across soft edges at its point. `smooth_groups` gives each corner its smooth group (split by hard edges; unchanged while points move), `corner_shading` the groups and every corner's normal in a few passes, `corner_normal_list`/`corner_normals` the normals alone. For updates after a move, `shading_reach` gives the points whose shading changes and `group_normals_at` works out one point's groups again. `shaded_triangles` cuts faces into fans of triangles with those normals (right for convex faces; faces with dents need a proper cut, Phase 4).
- **Editing** (`edit`): modelling operations as pure functions on a mesh, working on the element ids that still exist and reporting how many were missing (`Outcome`) instead of failing. So far `move_points`, the first operation the scene's stack records. `Mesh::set_position`, `point_slots`/`face_slots`/`hard_edges` and `Mesh::from_slots` (rebuilds a saved mesh with its ids, checking every value), `Mesh::memory_size` (an estimate, for the undo budget), and `VertexId::from_raw`/`FaceId::from_raw` for ids recorded earlier. `transform_points` moves points by an affine matrix (turns and scaling), each once, missing ones counted, a matrix that isn't numbers moving nothing.
- **Anchors** (`anchors`, [decision 0032](../../docs/decisions/0032-edits-find-their-points-again.md)): `Anchors::record(mesh, kind, elements)` keeps where an edit's points, edges or polygons were (`AnchorShape`), relative to the mesh's box. `still_fit` says whether the IDs still sit there (a resized shape fits); `find` finds them again by position when they don't: the nearest point, every edge along an anchored edge, every polygon inside an anchored polygon, and how many weren't found. Flat boxes (a plane) keep distances on their flat side.
- **Topology** (`topology`, [decision 0025](../../docs/decisions/0025-mesh-topology.md)): `Topology::new(&mesh)` builds an adjacency index from the face list:
- **Selection** (`selection`, [decision 0029](../../docs/decisions/0029-selection.md)): `Element` (point, edge or polygon) and `ElementKind`; `ElementSelection` keeps the selected points, edges and polygons apart, by ID, with `combine` (`Combine::Replace`, `Add`, `Remove`, `Toggle`) and `valid_in` (drops IDs the mesh no longer has). Commands worked out from the topology: `all`, `inverted`, `grown` and `shrunk` (one step through shared points), `loop_through` and `ring_through` (an edge's loop or ring as points, edges, or the band of polygons across or along it), and `face_edges`.
  - `edge_uses` (faces and the direction each runs), `edge_faces`, `vertex_edges`, `vertex_faces`, `face_edges` (corner order), `face_neighbours`, `vertex_neighbours`;
  - `is_border`, `border_edges`, `is_non_manifold`, `non_manifold_edges`, `flipped_edges`, `pieces`;
  - `edge_loop` (straight on through points where four quads meet, or along a border; stops at poles, triangles, n-gons) and `edge_ring` (across quads to the opposite edge).

  Edges are named by their points (`EdgeKey`). Everything is kept in flat lists (each element's neighbours one after another, with where each starts), built in one pass with a counting sort: a 3-million-polygon mesh in about 0.6 s. `edge_list` gives the edges as a slice. The mesh keeps its topology with its faces (`Mesh::topology`); `Topology::new` builds a fresh one.
- **Check** (`check`): `MeshReport` counts points, edges, faces, triangles (fan count) and pieces, open borders, and problems (non-manifold, flipped, zero-length edges, faces with no area, loose points); `is_closed`, `has_problems`.
- **Primitives** (`primitives`, [decision 0020](../../docs/decisions/0020-primitives.md)): `Primitive` (cube, plane, cylinder, sphere, disc, tube, cone) with per-kind settings and standard values, `PrimitiveKind` (the kind without settings, with name and description), `clamped` (sizes 1 mm to 10 km, segment and side limits, holes smaller than their shape), `mesh` and analytic `bounds`. Shapes are 1 m across as standard and stand on the ground with the pivot at the base centre; round ends are single n-gons as standard, or cut into rings (`cap_segments`, `ring_segments`) and/or met at a centre point (`cap_centre`, `centre_point`), built by `round_end`; round sides are soft and rims hard; the cube's twelve edges are hard.

Tests:
- topology on every primitive (edge counts, Euler numbers including the tube's hole, consistent winding, one piece) and on awkward meshes (three faces on one edge, flipped neighbours, loose points, zero-length edges, separate and touching pieces, n-gons);
- loops (across a grid, round borders, round a cut cube, stopping at poles, sphere equators and pole-to-pole, cylinder sides) and rings (grid, cut cube, cylinder, stopping at triangles);
- the mesh check on every primitive and on broken meshes;
- faces and ids, normals across hard and soft edges, triangulation, and for every primitive: closed surfaces facing outward (or flat ones facing up), volumes close to the true shape, bounds, point and face counts, hard edges, smooth and crisp shading, silly settings (NaN, zero, huge, holes bigger than the shape), the 1 m standard sizes, end rings and centre points, determinism, and the largest allowed mesh.
