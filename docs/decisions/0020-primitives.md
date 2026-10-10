# 0020. Primitives: how they are built, placed and drawn

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

Primitives (cube, plane, cylinder, sphere, disc, tube, cone) are the first real geometry, and the first use of the mesh data structure. Several choices are hard to change once models exist: where a new shape's pivot sits, how its ends are built, which edges are hard, and how the mesh is stored.

## Options and decisions

**Where shapes sit.** Programs differ: some centre new shapes on the origin (half the cube below the ground), others stand them on it. For game assets, the pivot at the bottom centre is what engines want (an asset dropped on the floor sits on it), so **every primitive stands on the ground with its pivot at the middle of its base**; the plane and disc lie flat on it. New shapes appear at the origin until the Move tool exists (Phase 3).

**Flat ends.** A cylinder's ends can be a fan of triangles, rings of quads, or one face with many corners. **One n-gon per end**: the cleanest for hard-surface modelling (insets and extrusions start from one face), and it stays flat so it shades correctly. The disc is one n-gon too (a ring of quads when it has a hole). *Added 2026-10-11:* each end can also be cut into rings (cap segments; ring segments on a disc), and its middle can be triangles meeting at a centre point instead of the n-gon, for people and engines that prefer that. The n-gon stays the standard.

**Hard and soft edges.** Shading comes from hard edges, never stored normals (DESIGN.md §4.1). The cube's twelve edges are hard however many segments it has; round sides are soft; the rims where a round side meets a flat end are hard; the sphere is all soft. Corner normals average the faces joined across soft edges, weighted by the angle each face fills at the corner (area weighting tilted the normals near a sphere's poles).

**Settings.** Each kind has its own settings with standard values. *Changed 2026-10-11:* every shape is **1 m across** as standard (a 1 m cube and plane, a cylinder and cone 1 m across and 1 m tall, a 1 m sphere with 24 × 12 pieces, a 1 m tube with a 70 cm hole); the first sizes (a 2 m cube, a 4 m plane) felt large, and 1 m is the easiest size to scale from. Sizes are stored in centimetres; the interface shows metres or centimetres as the user chooses. Every setting is clamped: sizes from 1 mm to 10 km, up to 256 segments and 512 sides, holes smaller than their shape, so no input can make an invisible or impossibly large mesh.

**The mesh.** `mesh::Mesh` stores points and n-gon faces by id (ids count up and are never reused) and the set of hard edges. Full adjacency (walking edges, loops and rings) is left for mesh editing in Phase 3. Faces are cut into triangles as fans from their first corner, which is right for the convex faces primitives make.

**Drawing.** The interface rebuilds a list of shown objects (mesh, world placement, selected) only when the scene or selection changes, reusing a primitive's mesh until its settings change; the GPU copy is replaced only when that list changes. Selected objects' edges are drawn in the accent colour, as are their names in the Object Manager (at first orange; changed at the project lead's request so selection follows the theme). Faces are now drawn from both sides (a plane must show from below; the back side is a little darker), and the grid sits just behind shapes lying on the ground.

**A new scene is empty.** The test cube is gone; the starting scene has no objects, as a new document should, and the Object Manager says how to add some.

## Consequences

- Primitive settings are ready to edit in the Attribute Manager (next milestone) and to be the base of an object's operation stack ([0006](0006-base-plus-operation-stack.md)).
- Meshes from primitives are closed (every edge shared by two faces) where the shape is solid; tests check this, the volume, the bounds and the shading for each kind.
- Fan triangulation must be replaced (ear clipping or similar) before tools can make faces with dents (Phase 4).
