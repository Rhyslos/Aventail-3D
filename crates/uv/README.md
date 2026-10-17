# uv

**Layer 1 · Core**

UV mapping: seams, unwrapping, relaxing, straightening and packing.

## Purpose

Turns a mesh's surface into flat UV islands and arranges them in the texture square: seams from hard edges, conformal unwrapping (LSCM), relax, straighten, packing with padding, texel density. See DESIGN.md §7.

## Must never

- Know about scenes, undo, files, the GPU or UI
- Change the mesh's shape: it only works out UV coordinates and seams
- Hang or allocate without bound on degenerate input (zero-area faces, islands with no area)

## Depends on

`mesh`, `glam`.

## Status

- **Seams** (`seams`, [decision 0098](../../docs/decisions/0098-seams.md)): `pieces` (the polygons left joined once seams are cut), `is_disk` (whether a piece lies flat in one go: its Euler characteristic once cut open is 1), and `suggest` (seams for a whole shape: the hard edges given and corners sharper than `CORNER_ANGLE`, then each piece cut to a disk: a piece with no border opened along the least seen path from its lowest point to its highest, then a spanning tree of its polygons keeping the most visible edges joined, the rest cut and trimmed back; "seen" is higher facing up or to the front, across flat stretches and on outward corners). Tests in `seams/tests.rs`: disks and non-disks, a cube's twelve edges, a cylinder's one line, a sphere's bottom-to-top cut at the back, every primitive, random starting cuts, empty and stray input.

The basics every part of the app reads (UVs per corner, islands, seam marks, moving UVs) are in `mesh::uvs`, and the primitives' ready-made UVs in `mesh::primitives`. Unwrapping, relaxing, straightening and packing arrive in the next milestones of Phase 7.
