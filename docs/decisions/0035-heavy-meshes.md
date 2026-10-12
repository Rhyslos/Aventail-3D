# 0035. Heavy meshes: shared parts, flat lists, kept indexes, background work

- **Status:** Accepted
- **Date:** 2026-10-12
- **Follows:** [0033](0033-heavy-mesh-safeguard.md)

## Problem

Decision 0033 set the safeguard: one imported object of 2–3 million polygons must stay usable, with targets in DESIGN.md §4.8. This milestone, the first of Phase 4, had to meet them before the modelling tools build on the same foundations.

The first run of the heavy-mesh check, at only 300 000 polygons, found:
- clicking an object: 0.25 s;
- hovering over polygons: 0.12 s per frame;
- dragging a polygon: 2.7 s per frame;
- box-selecting points: 2.5 s.

At 3 million polygons the check ran out of memory. Profiling showed where the time went:

| Problem | What was slow |
|---|---|
| Copying | Every edit made a full copy of the mesh, one list per face. |
| Connections | The connections were kept in tree maps, about 3 s to build, and built again on every change. |
| Picking | Picking tested every triangle, and rebuilt its world-space copy whenever the selection changed. |
| Drawing | The renderer rebuilt one world-space shape of every object whenever anything changed, the selection included. |
| Overlay | The edit-mode overlay redrew every edge of the edited object each time the mouse moved over a new element. |
| Mesh check | The Attributes panel's check ran on the main thread after every change. |
| Selection | Box selection removed duplicates by searching a list (quadratic). |

## Options

1. **Tune each slow spot** where it is, keeping the structures.
2. **Change the foundations** so the cost of an action follows what it changes, not the size of the mesh. The interfaces the rest of the app uses mostly stay the same.

## Decision

Option 2. What changed, by layer:

**Mesh (`mesh`)**

- **Shared parts.** A mesh's parts are shared between copies until one copy changes them:
  - point positions, in chunks of 4096;
  - faces, as one flat list of corners with where each face starts;
  - hard edges.

  An edit that moves a few points copies only their chunks. `Mesh::same_structure` and `Mesh::moved_since` tell caches what moved.
- **Connections.** They are kept in flat lists (`Topology`), built in one pass with a counting sort. They're worked out once per set of faces and kept with them (`Mesh::topology`), so moving points never rebuilds them.
- **Box.** Each chunk keeps the box around its points; a mesh's box is merged from those.
- **Shading.** Shading works in passes over the corners (`smooth_groups`, `corner_shading`), sharing the work between cores.
- **Updates.** `group_normals_at` and `shading_reach` update shading around moved points only.

**Picking (`tools`)**

- **Per-mesh index.** Each mesh gets its own index in its own space (`MeshPick`): a bounding volume hierarchy, boxes inside boxes, over its triangles, points, edges and polygon middles. The point, edge and polygon indexes are built only when first needed.
- **Placements.** A `PickScene` only places these indexes in the world, so selecting or moving objects never rebuilds them.
- **After a move.** Only the boxes around the moved points are worked out again.
- **Visibility.** Hovering checks visibility for the nearest candidates first, so a dense mesh with thousands of points under the mouse costs one or two ray casts.

**Interface (`ui`)**

- **Background indexing.** New shapes of 50 000 polygons or more are indexed on another thread as soon as they appear (`ComponentTracker::prepare`), with their connections.
- **During a drag.** The picking scene stays as it was when the drag began, since what moves is never a snap target. Nothing is looked up under the mouse while dragging.
- **The overlay** holds only the selected elements and the one under the mouse. The edited object's own edges and points are drawn with its mesh.
- **The mesh check** runs on another thread for heavy shapes. The Attributes panel says "Checking the mesh…" meanwhile.
- **Help → Add a heavy test shape** adds a 3-million-polygon bumpy ball, so anyone can try how their computer copes.

**Drawing (`render`, `app`)**

- **Per-object meshes.** Each object's mesh stays on the GPU in its own space (`MeshGeometry`, `ViewportRenderer::set_mesh`), indexed: one vertex per smooth group at each point. It is placed in the world when drawn (`MeshDraw`, a uniform per object).
- **Updates.** Selecting, moving objects or switching modes never re-uploads a mesh. Moving points uploads only the vertices around them.
- **Background geometry.** A new heavy shape's geometry is worked out on another thread. The view says "Preparing a heavy shape for drawing…" meanwhile.
- **Dense wireframes** (edges only a few pixels apart, as on a heavy shape seen from afar) fade out in the shader. A selected one keeps a faint tint. Points and edges too dense to see aren't drawn at all.
- **Overlay sprites** are one record each, made into two triangles on the GPU: a sixth of the memory.

**The heavy-mesh check** (`tools/heavy/heavy-check.sh`) times the interface, the drawing geometry and project files against the targets, and prints a table for the pull request.

## Consequences

Results on a 2-core test machine, 3 million polygons:

| Step | Before | After |
|---|---|---|
| Selecting | 0.6 s | 2 ms |
| Hovering | 0.13–0.33 s | under 7 ms |
| Dragging a polygon | 2.1 s a frame | under 1 ms a frame |
| Box selection | 7.6 s | 60 ms |

- **Indexing a new shape** takes 1.3 s and **its drawing geometry** 1.8 s, both in the background.
- **Project files:** saving takes 0.9 s and opening 0.6 s, for a 52 MB file.
- **Not measured here:** drawing speed on screen depends on the graphics card, and this machine has none. The project lead checks it with the heavy test shape.
- **Memory:** about 0.8–1 GB for the interface side, plus about 190 MB for the GPU's copy. The connections are the largest part. Storing edge numbers instead of edge names would roughly halve them, a later step if needed.
- **Undo budget:** steps that hold a whole mesh (creating a mesh object, Make editable) share it with the scene rather than copying it, but the budget counts it in full. A heavy shape uses up the budget faster than it should; a fairer count can come later.
- **Big primitives:** changing a primitive's settings still rebuilds it on the main thread. Primitives are capped (a cube at most about 400 000 polygons), which is fast enough. Import (Phase 5) will need its own background loading.
- **For later work:** modelling tools that change faces (extrude, cut) make a new structure, so the picking index and drawing geometry are rebuilt for that object, in the background when heavy. Tools should keep faces shared where they can.
