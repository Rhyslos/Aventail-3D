# 0071. OBJ export and import; exports prepared once for every format

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** [0012](0012-z-up-axes.md) (Z up), [0064](0064-automatic-hard-edges.md) (automatic hard edges), [0068](0068-pivot-editing-and-scene-units.md) (pivots and units)

## Problem

Phase 5 is about getting models out of the app and into Godot, Unity, Unreal Engine and Substance Painter. OBJ is the simplest common format, and reading it back lets models come in from other programs.

Before building, the project lead was asked about the controls and the shading. He chose:
- **the File menu only**, with no keys yet;
- shading **exported as seen**: with Automatic hard edges on, by angle; otherwise by the marked edges.

## Options

- **One writer per format,** each working out shapes and normals itself; or **one preparation step every format writes from** (chosen). The writers then agree on which objects go, the triangles, the normals, the units and the axes.
- **Points:** split per normal (simple, but a smooth surface falls apart into loose triangles, which spoils bakes); or **kept shared, with each corner naming its normal and UV** (chosen).
- **Import:** one object for the whole file, or **one per `o` object** (`g` when there are none) (chosen).

## Decision

1. **`fileio::export`:**
   - `prepare` takes the shown shapes (or the selection and what is inside it), with every edit applied, in the Objects panel's order.
   - Each object comes out twice: in the file's world (for OBJ), and around its pivot with the pivot's place (for glTF, next).
   - Each corner gets the shading normal the viewport works out. A mirrored object has its polygons run backwards, so they still face out.
   - `ExportOptions` set:
     - **units:** metres (standard) or centimetres;
     - **up axis:** Y up (standard) turns (x, y, z) into (x, z, -y), or Z up;
     - **hardness**, given by the window as seen;
     - **triangulate:** the viewport's own cut.
   - `export_bytes` / `export_to` write a format safely and say how many objects and triangles went out.
2. **`fileio::obj`:**
   - **Writing:** `write_obj` writes a comment naming the app (from the branding constants), a `# Units:` note, then `o`, `v`, `vt`, `vn` and `f` lines with indices across the file.
   - **Reading:** `read_obj` reads `v`, `vt`, `vn`, `f` (with any index form, negative ones included), `o` and `g`. It skips and counts what it doesn't use, leaves out and counts broken polygons, and refuses numbers that aren't real and files past the limits rulebook.
   - **Units on import:** files are read as metres with Y up, unless they carry this app's `# Units:` note.
   - **Hard edges** go where the two polygons at an edge have different normals.
   - **UVs** go into a `uv` corner layer (`mesh::attributes::UV`).
3. **The window:**
   - **File → Export…** opens an Export window: kind of file (OBJ for now), what, metres or centimetres, Y or Z up, and cut into triangles. It asks the app to export; the app asks where to save (`rfd`) and reports back.
   - **File → Export again** repeats the last export to the same file.
   - **File → Import…** asks the app to read a file; the objects are added with one new command, `Command::InsertMany` ("Import crate.obj", one undo step), selected and framed.

## Consequences

- glTF, presets and kit export reuse `prepare`; only their writers are new.
- An OBJ holds no pivots: each object is written where it is in the world. Formats with pivots use the local shape.
- The last export is remembered while the app is open; engine export paths (a later milestone) will keep it with the project.
- Imported objects are plain meshes: an OBJ has no settings to bring back.
