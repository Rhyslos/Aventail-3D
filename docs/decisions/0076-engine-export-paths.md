# 0076. Engine export paths

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** [0075](0075-export-presets.md) (export presets)

## Problem

Going back and forth between the app and a game engine means exporting, finding the engine's folder, and replacing the file every time. The engine should pick up a change as soon as it is saved.

## Options

- **Where the folder is kept (asked, the project lead chose):**
  - **per project, saved with it, with Export on save switched on once a folder is chosen** (chosen);
  - in the app's settings, for every project;
  - per export, asked each time.
- **Which settings:**
  - ask for an engine;
  - **work the engine out from the folder** (chosen): a Godot project has `project.godot`, a Unity project `Assets` and `ProjectSettings`, an Unreal Engine project a `.uproject` file, in the folder or one above it. Its preset is used; a folder in no known project gets Godot's.
- **Unity and Unreal Engine settings:**
  - OBJ;
  - **glTF** (chosen): pivots and materials survive. Unity reads it through its glTFast package; Unreal Engine 5 reads it and turns it into its centimetres and Z up.
- **Safety:** a project file can come from anyone and it names the folder. So the app only writes where it is safe:
  - **nothing is created** (the folder must already be there);
  - **files are named after the project** (`crate.avtl` exports `crate.glb`);
  - **only files this app wrote are replaced.** An OBJ's or MTL's first line, or a glTF's `generator`, must name the app. Anything else is left alone and the status bar says so.

## Decision

1. **Scene:** `ExportTarget { folder, on_save }` is kept on the scene, beside the units. `Command::SetExportTarget` changes it as an undo step ("Change the game project folder"), and an empty, over-long or multi-line path is refused. The scene only keeps the path as text; files are `fileio`'s and the app's work.
2. **`fileio::engine`:**
   - `detect` (the engine and the project's folder);
   - `may_replace` (the file isn't there, or the app wrote it);
   - `export_into` (the folder must exist; every file is checked before any is written; `TargetError` in plain words).
3. **Presets:** `ExportPreset` gains **Unity** and **Unreal Engine** (glTF). The Export window's Made for lists them.
4. **The Export window** has a **Game project folder** section:
   - the folder and the engine project it is in;
   - **Export on every save** (on once a folder is chosen);
   - **Choose folder…** and **Clear**.
   Choosing a folder sets Made for to its engine.
5. **File → Export to game project** exports there now, without saving.
6. **Saving** exports there too while Export on every save is on, once the save has succeeded. The status bar says both. A failed export never stops the save. Backup copies don't export.
7. **File format 19** stores the folder and the switch. Older files open with none.

## Consequences

- Godot and Unreal Engine re-import a changed file by themselves; Unity does too once glTFast is installed.
- A project opened on another computer keeps its folder; if it isn't there, the export says so and the save goes ahead.
- A file in the game project that the app didn't write is never overwritten, even if it has the project's name.
