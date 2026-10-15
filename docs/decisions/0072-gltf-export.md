# 0072. glTF export: binary `.glb`, each object at its pivot

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** [0071](0071-obj-export-and-import.md) (OBJ export, exports prepared once), [0068](0068-pivot-editing-and-scene-units.md) (pivots)

## Problem

Godot and Substance Painter read glTF best, and glTF can keep where each object's pivot is, which OBJ can't. Pivot editing was built for exactly this.

## Options

- **`.gltf` text with a separate `.bin`,** or **one binary `.glb` file** (chosen). One file is easier to copy around and drop into a Godot project.
- **Write the JSON by hand,** or **use `serde_json`** (chosen: MIT or Apache-2.0, the most used Rust JSON library). The `gltf-json` crate was not needed for what we write.
- **Nodes:**
  - one node for the whole scene, baked in place;
  - **one node per object at its pivot, holding its shape around the pivot** (chosen);
  - the full group hierarchy, which can come later if wanted.

## Decision

1. **`ExportFormat::Glb`** is the Export window's standard kind of file.
2. **glTF fixes its own space.** glTF is always metres with Y up, and only holds triangles, so `ExportFormat::options` sets those whatever the window says. The window greys those choices out with a note.
3. **The file:**
   - **Nodes:** each exported object is a node named after it, with a `matrix` placing its pivot (none when it sits at the origin). Rounding crumbs are swept to 0.
   - **Meshes:** each node holds a mesh of the object's shape around its pivot.
   - **Vertices:** one per distinct point, normal and UV, so points split where shading or UVs break, as every engine needs.
   - **UVs:** turned to glTF's way (V counts down).
   - **Triangles and materials:** triangles are listed per material slot, with one grey material until material slots arrive.
4. **Checked:** a sample with three shapes passes the Khronos glTF validator with no errors, warnings or hints. Tests read files back the way an engine does: the container, the JSON, the numbers, and buffer views aligned to 4 bytes.

## Consequences

- Godot places, turns and snaps each exported object by the pivot set in the app.
- Objects come out flat, side by side under the scene, even when grouped in the app. Groups as parent nodes can come later.
- Tangents (MikkTSpace) and material slots are added to this writer in their own milestones.
