# uv

**Layer 1 · Core**

UV mapping: seams, unwrapping, relaxing, straightening and packing.

## Purpose

Turns a mesh's surface into flat UV islands and arranges them in the texture square: seams from hard edges, conformal unwrapping (LSCM), relax, straighten, packing with padding, texel density. Wraps xatlas (vendored, MIT) for automatic charting and packing. See DESIGN.md §7.

## Must never

- Know about scenes, undo, files, the GPU or UI
- Change the mesh's shape: it only writes UV coordinates and seam flags
- Hang or allocate without bound on degenerate input (zero-area faces, islands with no area)

## Depends on

`mesh` (planned).

## Status

Empty. UV tools arrive in Phase 6.
