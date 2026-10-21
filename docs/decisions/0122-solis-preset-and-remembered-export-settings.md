# 0122. A Solis export preset, and the last export remembered

- **Status:** Accepted
- **Date:** 2026-10-21
- **Follows:** [0075](0075-export-presets.md) (export presets), [0076](0076-engine-export-paths.md) (engine export paths)

## Problem

The project lead has started Solis, a game engine of his own, in a separate repository. He asked for it to be a built-in choice in the Export window. While choosing, he also asked that the app remember the last export's settings: today the Export window starts at Godot on every launch, so someone who always exports for one place sets it up again each time.

## Options

- **Where Solis goes:**
  1. **One more preset in Made for, with Godot still first and the standard on first use (his choice).**
  2. Solis first, as the standard.
- **Recognising a Solis project folder:** Godot has `project.godot`, but Solis has no project file yet (its Phase 2).
  1. **No recognition yet (his choice).** A game project folder no engine claims gets Made for's engine.
  2. Agree on a `project.solis` file now and have Solis create it later.
- **What is remembered:**
  1. **The last export's Made for and file settings, kept by the app like its other settings (chosen).** Not what goes out (everything or the selection): that belongs to the moment, and a remembered "Selected only" would make the next project's first export empty.
  2. The window's choices whenever they change, even without exporting. Rejected: he asked for "the last thing you exported", and a look at the list followed by Cancel shouldn't change the next project.

## Decision

1. `fileio::export::ExportPreset::Solis`: glTF, metres, Y up, triangles, each object at its pivot, the same file as for Godot but without Godot's collision endings in names (`godot_names` off). Listed after Unreal Engine, before Substance Painter. `ExportPreset::is_engine`.
2. `ui::ExportSettings`, kept by the app under `export_settings`, set by every export (Export…, Export again, Export kit…), starting the window's choice on launch. A damaged value falls back to the standard (Godot's), and a preset's own settings win over settings that disagree with it.
3. A game project folder in no recognised engine's project exports with Made for's preset when that is an engine, otherwise Godot's (before, always Godot's).

## Consequences

- Solis reads the file through its own glTF import (Solis's Phase 1, milestone 2), so nothing in Aventail depends on Solis's code; the two stay separate (Solis's decision 0007).
- When Solis has a project file, `fileio::engine::detect` can recognise it and `Engine::Solis` joins Godot, Unity and Unreal Engine. If Solis grows needs of its own (collision names, its own units), its preset changes then.
- "Solis" means only the game engine: the app's own renderer of that name was cut from the roadmap ([0123](0123-no-renderer-for-now.md)). If the engine is renamed, the preset's name changes in `ExportPreset::name` and the manual.
- Other settings files from older versions open with Godot's settings, as before.
