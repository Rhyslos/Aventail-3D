# 0075. Export presets for Godot and Substance Painter

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** [0071](0071-obj-export-and-import.md), [0072](0072-gltf-export.md), [0073](0073-mikktspace-tangents.md) and [0074](0074-material-slots.md)

## Problem

The Export window has the right switches, but you have to know which ones each program wants. Getting one wrong gives a model that is 100 times too big, lying on its back, or baked against different triangles from the game's. The two programs the project lead uses most should be one click.

## Options

- **Where the presets live:**
  - extra File menu entries ("Export for Godot…");
  - **a "Made for" choice at the top of the Export window** (chosen). The project lead chose the File menu with no keys for exporting, so the presets go inside the window it opens.
- **Godot:**
  - **glTF** (chosen): Godot's favourite format. Each object arrives at its pivot, with its materials, and Godot re-imports it by itself.
  - OBJ: no pivots.
- **Substance Painter:**
  - glTF, which Substance Painter reads too;
  - **OBJ, metres, Y up, cut into triangles** (chosen). It is the most common way into Substance Painter, its `usemtl` names become texture sets, and cutting on export guarantees the bake uses the viewport's triangles, the same ones the glTF for the game holds.
- **Changing a setting by hand:** keep showing the preset, or **leave it ("Your own settings")** (chosen), so the window never claims a preset it isn't following.
- **Export scale:** the presets choose the units (metres for both). A separate scale factor isn't needed while every program's unit is metres or centimetres; Unreal Engine (centimetres) comes with engine export paths.

## Decision

1. `fileio::export::ExportPreset` (`Godot`, `SubstancePainter`) with a name, a one-sentence description, and the `PresetSettings` it chooses (format, units, up axis, triangles).
2. The Export window's first row, **Made for**: Godot, Substance Painter, or Your own settings. A preset sets the kind of file and the size and direction below it; *What* (everything shown or the selection) is left as it was. Changing any of those settings by hand switches to *Your own settings*. For glTF, only the kind of file counts, since glTF fixes the rest.
3. Godot is the standard. The window remembers its choice while the app is open, and **Export again** repeats the preset.
4. The status bar names the preset: *Exported 3 objects for Godot (240 triangles, 2 materials) to crate.glb.*

## Consequences

- Engine export paths (next milestone) build on the presets: a project's folder for an engine plus that engine's preset.
- More presets (Unity, Unreal Engine, Blender) are one entry each.
