# booleans

**Layer 1 · Core**

Booleans between closed shapes: union, subtract and intersect.

## Purpose

Works out a shape cut by other shapes (its *cutters*), as an ordinary polygon mesh with its values carried over: polygons rebuilt from Manifold's triangles, materials and UVs from the polygon each part came from (the cutter's own for its faces), hard edges and seams kept along original edges, and the new edges along the cut made seams (where there are UVs) and hard where the surfaces meet sharply. See [decision 0135](../../docs/decisions/0135-live-booleans.md). The scene uses it to show live booleans; this crate knows nothing about objects.

## Must never

- Know about objects, scenes, undo, files, the GPU or UI
- Change a mesh in place: `apply` takes meshes and returns a new one
- Crash on bad input: shapes that aren't closed solids, flattened or broken cutter placements, or a failure inside the library all come back as a `Problem` in plain words
- Give a different result for the same input

## Depends on

`mesh`, `glam`. Outside library: `manifold-rust` (Apache-2.0; a pure-Rust port of Manifold, so no C++ is needed to build), and through it `clipper2-rust` (BSL-1.0), `dashu` (MIT or Apache-2.0), `num-modular` (Apache-2.0), `num-traits` and `rustc-hash` (MIT or Apache-2.0).

## Status

- **`apply(kept, cutters, options)`** ([decision 0135](../../docs/decisions/0135-live-booleans.md)): each `Cutter` (a `Kind`: `Union`, `Subtract` or `Intersect`, its mesh and a matrix into the kept shape's space) is applied in turn. `Options { seam_angle, max_polygons }`. `Problem`: `KeptNotClosed`, `CutterNotClosed(i)`, `CutterFlat(i)`, `NotANumber`, `TooBig`, `Failed(why)`.
- **How** (`solid`, `rebuild`): the shapes are joined into one mesh with `mesh::parts::append` (so a mirrored cutter is turned the right way round and every value layer comes along); each shape's polygons are cut into triangles with `mesh::face_triangles`, each tagged with its polygon's id (Manifold's face ids), points at the same place along open edges merged (`MeshGL64::merge`), and imported strictly (`Manifold::from_mesh_gl64`: an open or non-manifold shape is refused). The library is called inside `catch_unwind`, so even a bug in it leaves the shape uncut instead of closing the app. Back from Manifold, the triangles from each polygon are grouped into pieces that hang together; a piece that is a disc (points − edges + triangles = 1, one outline) becomes one polygon; anything else (a ring round a hole) becomes convex polygons, its triangles joined across their longest shared edges first wherever the joined polygon stays convex (`convex_pieces`). Corners take their values with `mesh::Builder`: a copy at an original corner, a blend of the two ends along an original edge (so UVs stay in line along a cut edge), and mean value weights inside. Edges along an original edge copy its values (`Builder::copy_edge`); edges along the cut are hard above `seam_angle` and seams where the shapes have UVs.
- **Tests** (`tests.rs`): a notched cube (nine polygons, three six-cornered), a round hole through a cube (its rings convex polygons), union apart and touching, intersect overlapping and apart (empty), a cutter inside (hollow) and outside (unchanged), flush coplanar faces (no slivers), n-gon caps, ten cutters, a mirrored cutter, points split along a closed surface, open, non-manifold, flattened and broken input refused, empty cutters, UVs an exact affine map on every result polygon and seams along the cut, materials from each shape, hard edges, the same result twice, too many polygons, and a fixed-seed fuzz test of random shapes, damage and placements that never panics. `an_ordinary_cut_takes_milliseconds` times a 2 048-polygon sphere cut by a cylinder; the ignored `each_step_timing` times each step (run it with `--release --ignored --nocapture`).
