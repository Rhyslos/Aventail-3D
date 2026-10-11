# 0025. Mesh topology: a derived index over a face list

- **Status:** Accepted
- **Date:** 2026-10-11

## Problem

Phase 3's selection and editing tools need to know how a mesh connects:
- which faces use an edge, and which edges meet at a point;
- how edge loops and rings run;
- where the open borders are;
- what is broken.

DESIGN.md §4.1 asks for "full adjacency (vertices, edges, faces, corners), similar to Blender's BMesh or a half-edge structure", with stable ids. The storage chosen now is hard to change once tools are written against it.

## Options

1. **Half-edge structure.** Each edge is stored as two linked halves; walking around a face or a point is fast. It can't represent an edge shared by three faces, or a point where two surfaces touch, and imported or damaged meshes have both. Every operation must also keep the links correct, which is where half-edge code usually breaks.
2. **A BMesh-like structure** with radial cycles. It handles everything, but it's a large amount of linked bookkeeping kept correct by every operation.
3. **Keep the face list as the source of truth, and derive an adjacency index from it.** The mesh stays what it is today: points and faces as lists of corners, which can describe any mesh, including the awkward ones. `Topology::new(mesh)` builds the index in one pass. Operations only ever produce a new face list, so there are no links to get wrong.

## Decision

Option 3.
- **`mesh::Topology`** answers:
  - the faces using an edge, and which way each runs along it;
  - the edges and faces at a point;
  - a face's edges in corner order, and its neighbours;
  - open-border, non-manifold and flipped edges;
  - separate pieces.
- **Edge loops** run straight on through points where exactly four quads meet, or along an open border, and stop at poles, triangles, n-gons and borders.
- **Edge rings** cross quads to the opposite edge, and stop at anything else.
- **Edges are named by their two points** (`EdgeKey`), so an edge's name is as stable as its points' ids. Corners are named by face and position.
- **`mesh::MeshReport`** is a health check built on the topology:
  - counts: points, edges, polygons, triangles, pieces;
  - open borders, which are information rather than a fault;
  - problems: edges shared by three or more faces, flipped neighbours, edges with no length, faces with no area, loose points.

  The Attributes panel shows it for every shape.
- **Generic attribute layers are left for the phases that need them:** corner UVs (Phase 6), seams (Phase 6), material slots (Phase 5) and, much later, skin weights. Each will be designed with its first real use, as named, typed layers (DESIGN.md §4.5). Hard edges stay a set of edge keys.

## Consequences

- Any mesh can be stored and inspected, however broken, and tools refuse to do what they can't do safely rather than crash.
- The index is rebuilt after each change. That costs a pass over the mesh: fine for game assets of tens of thousands of faces. Tools working on huge meshes may later keep it up to date incrementally, without changing the storage.
- Faces are still cut into triangles as fans, which is right only for convex faces. Ear clipping is still due before tools can make faces with dents (Phase 4).
