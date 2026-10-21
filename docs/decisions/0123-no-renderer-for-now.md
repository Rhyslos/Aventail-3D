# 0123. No renderer of the app's own, for now

- **Status:** Accepted
- **Date:** 2026-10-21
- **Replaces part of:** [0119](0119-roadmap-after-the-uvs.md) (Solis v1 as Phase 11, Solis's path tracer as Phase 20)

## Problem

Decision 0119 planned Solis as the app's own renderer: a real-time one first (Phase 11), made to show game assets the way the game engine will, and a path tracer much later (Phase 20). The idea was for it to share its renderer with the project lead's game engine, also called Solis, now started in its own repository.

On 2026-10-21 the project lead decided against it: a game engine's renderer is made for games (many objects, a steady frame rate, the engine's own scenes), and it won't really work as a 3D program's renderer. He asked to cut the rendering plan for now, at least version 1.

## Options

1. **Cut both the real-time renderer and the path tracer from the roadmap (chosen).** Nothing in the app depends on either; nothing was built.
2. Cut only the real-time renderer and keep the path tracer as Phase 19. Rejected for now: it was planned on top of version 1 and would be a renderer project of its own.
3. Keep a renderer but make it separate from the engine. Rejected: the app's purpose is game assets, and the engine itself is now the place to see them as the game will.

## Decision

- Phase 11 (Solis v1) and Phase 20 (Solis, full) leave the roadmap. Rendering beyond the viewport is out of scope again (DESIGN.md §1).
- The later phases move up one: texture painting 11, materials 12, alpha 13, polish 14, scripting 15, rigging 16, animation 17, Gauntlet 18.
- What served the texture painter stays with it: the toon (cel-shaded) preview and outlines in the viewport (DESIGN.md, Texturing).
- "Solis" means the project lead's game engine. Models reach it by export (the Solis preset, [0122](0122-solis-preset-and-remembered-export-settings.md)).

## Consequences

- No `render`-style crate for a separate renderer is planned; the viewport's renderer stays the app's only one.
- Seeing an asset exactly as the game will is done in the engine (Godot, Solis), not in the app.
- Older decision records keep the phase numbers they were written with.
- A renderer can come back later with a new decision, if it is wanted for its own sake.
