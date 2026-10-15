# 0073. Consistent triangulation and MikkTSpace tangents in exports

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** [0071](0071-obj-export-and-import.md) and [0072](0072-gltf-export.md) (OBJ and glTF export)

## Problem

A normal map baked in Substance Painter only looks right in a game engine if the two agree on:

1. **The triangles.** A quad or n-gon can be cut more than one way.
2. **The tangent space.** A normal map is read along each surface's tangents. MikkTSpace is the standard Substance Painter, Godot, Unity, Unreal and Blender all use.

## Options

- **Triangles:**
  - leave the cutting to each program, which differ;
  - **always cut the way the viewport cuts**, the app's own deterministic cut (chosen).
- **Tangents:**
  - write none, and let each program work them out;
  - **write MikkTSpace tangents**, so the file itself settles it (chosen).
- **Library** (all permissive):
  - `mikktspace` 0.3 needs `nalgebra` as a dependency;
  - **`bevy_mikktspace` 1.0** (chosen): safe Rust, no dependencies, byte-identical to the reference C code, licensed Zlib AND (MIT OR Apache-2.0).
- **Which UVs:**
  - the app's (V up);
  - **the file's (glTF's V down)** (chosen): a program that works the tangents out again from the file gets the same answer.

## Decision

1. **Triangles:**
   - Exports cut polygons with `mesh::face_triangles`, the viewport's own cut: always for glTF, and for OBJ when *Cut polygons into triangles* is ticked.
   - A test checks that a dented polygon's exported triangles are exactly the viewport's.
2. **Tangents:**
   - A glTF export of a shape with UVs carries a `TANGENT` per vertex: MikkTSpace on the file's triangles, normals and UVs, with the handedness in W.
   - Vertices split where tangents differ.
   - Shapes without UVs carry none, since tangents need UVs. Shapes get UVs from imported files now, and from the UV tools in Phase 8.
3. **Checked:** a grid with UVs passes the Khronos glTF validator with no errors or warnings. Its two notes are a tangent with no normal texture to use it yet, and UVs with no texture yet.

## Consequences

- Bakes match from the moment shapes have UVs. Nothing more to do in the UV phase but give them UVs.
- OBJ can't hold tangents: the reading program works them out, from the same triangles if they were cut on export.
