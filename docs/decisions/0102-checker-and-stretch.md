# 0102. Checker texture and stretch heatmap, with explained fixes

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0095](0095-uvs-before-advanced-modelling.md) (the UV phase plan), [0063](0063-wireframe.md) (how shapes are drawn), [0100](0100-texture-size-and-texel-density.md) (texture size), [0035](0035-heavy-meshes.md) (meshes on the GPU)

## Problem

UVs need checking on the model: does a texture sit evenly, which way does it run, where does it stretch, and what should be done about it? Other programs show a checker texture and a stretch view, but leave the reading to the artist. The roadmap asks for both, with the fixes explained.

## Options

- **Where the views live:** **a Surface choice in the viewport's Display menu: Colours, Checker, Stretch** (chosen), beside the shading looks, remembered with the other view options, for every view. The UV Editor colours its polygons by stretch too while Stretch is chosen, so one switch shows both.
- **The checker:** **drawn by the shader from each point's UV** (chosen): one square per 128 texture pixels (16 across a 2048 texture, at least 4), light and dark, tinted red across and green up the square, so a piece turned or mirrored shows at once. A checker image file would need texture loading for no gain.
- **The stretch view:**
  - **worked out per pixel on the GPU** (chosen): how the UV and the world position change across the pixel give the map from the surface to the texture; its two stretches (largest and smallest, at right angles) say how **unevenly** the texture lies (their ratio) and how much **room** it gets (their product, against the object's texel density). Green where even, yellow at 1.5 times off, red at twice off; purple where there is no room. No per-polygon data, so nothing changes in how meshes are kept on the GPU beyond the UVs;
  - per-polygon colours worked out on the CPU: needs every polygon's vertices split apart.
- **Explaining:** the same measures, per polygon on the CPU (`uv::stretch`), drive the UV Editor: pointing at a polygon says, in plain words, what is wrong and what to do (no room: Unwrap; mirrored: Unwrap or flip; stretched one way: Relax, or a seam across it then Unwrap; too much or too little texture: Unwrap or Set texel density). Without a polygon under the mouse it says how many are 1.5 times off or more.
- **UVs on the GPU:** each mesh vertex now carries its UV, and vertices split where the UVs do (as they do where shading or materials break), so the views are exact at seams.

## Decision

1. `render`: `MeshVertex::uv` (vertices split by UV in `MeshGeometry::shaded`), `MeshGeometry::uv_density` (from `mesh::uvs::average_density`), `SurfaceDraw` (Plain, Checker { squares }, Stretch { density }) on `MeshDraw`, sent in the object uniforms (the density spread by the placement's scale); the shader's `surface_color`.
2. `ui`: `Surface` in `ViewOptions` and `ViewportView`, the Display menu's Surface entries, `checker_squares`; `SceneDrawing::texture_size`. The app passes each object's density from its uploaded geometry.
3. `uv::stretch`: `Stretch { uneven, size, flipped, empty }`, `off`, `explain`, `stretch_of`, `SOME` (1.5) and `FAR` (2).
4. The UV Editor: polygons coloured by `stretch_color` and the explanation over its notes while Stretch is chosen.

## Consequences

- Meshes with UVs take a few more vertices on the GPU (one per UV at seams).
- Bake readiness (a later milestone) can reuse `uv::stretch` to flag problem polygons before export.
