# 0212. Live engine preview

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's last milestone ([0177](0177-texture-painter-plan.md)): "textures re-exported into the open game project as you paint, so Godot updates." The texture is only finished when it looks right in the game, under the game's light; saving and exporting after every stroke to check is slow.

## Options

- **A link into the running engine** (a plugin that receives textures): live to the frame, but needs a plugin per engine and a network protocol.
- **Export into the game project folder as you paint (chosen):** builds on the game project folder ([0076](0076-engine-export-paths.md)). Godot reimports changed files by itself when its editor has focus, and Unity and Unreal can watch their folders too.

## Decision

- **Export → Game project → While painting: Update the game live** (`ExportTarget::live`), off as standard.
- While it is on, a change to the painting (any texture set) is exported into the folder **once it has rested for 0.8 s**, never in the middle of a stroke or while files are being written, and not again while nothing changed. It is the same export as File → Export to game project: the whole file, with the engine's settings, named after the project.
- Only painting triggers it; modelling still goes out on save or by hand, so a half-made shape doesn't land in the game.
- **Saving:** project format 82 stores the switch with the folder.
- **No keys:** one tick box.

## Consequences

- Paint in the app, glance at the game: Godot shows the new texture within about a second of the brush stopping.
- Each update writes the whole file; for very large textures that takes a moment on another thread, during which the status bar says so.
