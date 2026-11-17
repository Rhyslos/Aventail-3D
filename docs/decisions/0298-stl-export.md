# 0298. STL export for 3D printing

- **Status:** Accepted
- **Date:** 2026-11-17

## Problem

The project lead and a friend who 3D prints asked (2026-11-17) for models to go straight from the app to a slicer (PrusaSlicer, Cura, Bambu Studio and the like). Slicers read STL. The friend's own routine today is to boolean-union everything into one object before exporting, because separate overlapping parts, open edges and flipped normals give errors in the slicer. STL has no units, and slicers read its numbers as millimetres, while one unit in this app is a centimetre.

Their requirements: binary STL only, export only (no STL import: the friend has had bad experiences with imported STLs), one file for everything going out, units with millimetres as standard, merging into one solid as an option, and a check before writing that says in plain words what would trouble a slicer, without stopping the export.

## Options

1. **A separate File → Export STL… command** with its own small window. Simple, but a second place to choose what goes out, and its settings would need remembering apart from the others.
2. **STL as a third format in the Export window**, with a **3D printing** entry in Made for. One place to export, the same Objects choice, the same remembered settings, Export again and Export kit… for free.
3. **Use a library** to write STL. The binary format is 84 bytes plus 50 per triangle; a library would add a dependency to save a few dozen lines.

For merging: **(a)** a new union written for printing, or **(b)** the app's own booleans (`booleans` crate, Manifold), which already refuse open shapes with a reason in plain words.

## Decision

Option 2, written by hand (no new outside library), merging with (b).

- **Where:** `ExportFormat::Stl` (`.stl`) and `ExportPreset::Printing` ("3D printing" in Made for, not an engine). Custom settings lists STL as a third file type. For STL the Export window's General section shows the Objects choice, the locked line *STL · binary · Z up · triangles* (or Custom settings' rows, greyed out), **Print units**, **Merge into one solid**, *What goes out* with the print's size, and the **Print check**; the texture, realism, normal map, pixel art and levels of detail rows are not shown, with one line saying they don't apply.
- **Shapes:** the same `prepare_for` path as OBJ, so modifiers, live mirrors and booleans are applied and polygons cut into triangles the viewport's way; cutters, outlines' shells, levels of detail, textures and Godot's names stay out.
- **Space:** the app and STL are both Z up, so nothing is turned; centimetres are multiplied into the **print units**: millimetres (standard), centimetres, metres or inches.
- **Normals:** worked out per triangle from its corners in the order they go round, unit length, zero for a triangle with no area. A mirrored object's polygons are already turned the right way by the export path, so a shape whose polygons face out writes normals that face out.
- **Header:** the app's name and version (from `branding.rs`), never starting with "solid" (some readers then take a binary file for text). Export into a game project and kit export recognise their own STL files by it.
- **Merge into one solid:** on as standard, used when more than one shape goes out. The shapes, already in the world, are rebuilt as meshes (points shared, so a closed shape stays closed) and joined with a Union boolean on these copies; the scene never changes. If it fails, the export stops and says which shape and why in plain words; the Export window opens again with the reason and an **Export without merging** button that writes the same file unmerged.
- **Print check:** the app's own checks, shape by shape: `MeshReport` (open borders, edges shared by more than two polygons, polygons with no area) and `faces_facing_wrong` (what Fix normals turns), plus a flat sheet (an open shape all in one plane) reported as having no thickness. Each finding says what to do (Fill hole, Weld by distance, Fix normals, Shell). They are warnings: the export still goes ahead. It also says when a finding will stop the merge.
- **Remembered:** Print units and Merge are kept with the other export settings ([decision 0122](0122-solis-preset-and-remembered-export-settings.md)); settings saved before read as millimetres and merged.
- **No import, no text STL, no new keys or mouse actions.**

## Consequences

- One Export window still covers every kind of file; Export again and Export kit… work for STL too (a kit gives each piece its own merged STL).
- Merging needs closed solids, as every boolean does. A scene with open shapes (a plane, a cylinder with open ends) must be fixed first or exported unmerged; the check says so before exporting.
- The print check works out each shape's facing on every change while the Export window shows STL; for shapes of millions of polygons that takes a moment when the window opens (see the heavy-mesh check's STL lines).
- A merged print loses nothing a slicer reads: STL holds only triangles.
