# 0119. The roadmap after the UVs

- **Status:** Accepted
- **Date:** 2026-10-18
- **Replaces part of:** [0117](0117-language-support.md) (language support as Phase 9)
- **Follows:** [0011](0011-lua-scripting.md) (Lua scripting), [0108](0108-uv-powerhouse-and-baking.md) (Phase 8)

## Problem

The first testers found the app overwhelming, and the project lead brought new, bigger ideas: Solis (the app's own renderer, real-time and path traced), Gauntlet (effects), LustScript (his own scripting language) and an alpha release. He set the order himself, and changed his mind on language support: not a phase of its own yet, but the app built so languages can be added later without a rework.

## Options and choices

The order is the project lead's (2026-10-18). Where there were choices to make, these were made:

- **When the tester feedback is handled: a brief phase right after the whole UV phase (Phase 8), before advanced modelling (his choice).** It covers:
  - a side palette that slides out to show names;
  - high contrast for edges, points and the selection;
  - a dropdown arrow on the selection mode button;
  - labels on everything;
  - tidier menus.
- **Language support:** only English ships for now. It is built as **the one language package** in the same brief phase, through one translation layer, with a check that no text skips it. Adding a language later is then a matter of adding a package. The rest of 0117 (a language setting, local numbers, fonts, translations) waits until languages are wanted.
- **Symmetry:**
  - The live mirror (symmetry) stays as it is.
  - **Mirror becomes a modifier** that can be turned on per object, in advanced modelling. The two are separate and both work: the modifier mirrors one object's shape, while the live mirror makes edits work on both halves.
- **Solis:**
  1. A general renderer to rival Cycles or Arnold. That is years of specialist work, and Blender already gives Cycles away.
  2. **A real-time renderer made to show game assets as the game engine will (the same lighting and tone mapping as Godot first), with a stylised toon mode, on the GPU through `wgpu` (chosen for v1).** This is something the other free tools don't do well, and it serves the app's game-first aim and the texture painting that follows it.
  - The path tracer comes much later (Phase 20).
  - Solis lives in its own crate, so it could be used outside the app.
  - **A Blender add-on stays possible, not a goal.** It's decided at Phase 20, once Solis v1 has shown what it does differently. Blender's own renderers (Cycles, EEVEE) are strong, so an add-on is only worth it for what Solis does that they don't.
- **Scripting:**
  - **LustScript is the default scripting language, and plain Lua works in LustScript files.**
  - Decision 0011's design (Luau through mlua) is revisited when scripting is planned.
  - LustScript's licence must be permissive (MIT or Apache-2.0, say) for the app to depend on it, as every dependency must be (§9). The project lead says it will be free and open source; the exact licence is still open.
- **Gauntlet (effects):** physics, particles and simulations, driven by scripts and exported baked for game engines. It is planned from research when its time comes, as the project lead is new to effects.

## Decision

The roadmap after Phase 8 (DESIGN.md §10):

| Phase | What |
|---|---|
| 9 | First use, and English as a language package |
| 10 | Advanced hard-surface and modifiers (with Mirror as a modifier) |
| 11 | Solis v1: real-time rendering |
| 12 | Texture painting |
| 13 | Materials with a node editor |
| 14 | Alpha release (the project lead's website, maybe Steam) |
| 15 | Bug fixes and polish |
| 16 | Scripting (LustScript, and Lua) |
| 17 | Rigging and skinning |
| 18 | Animation |
| 19 | Gauntlet: effects |
| 20 | Solis, full: path tracing |
| Later | Other languages |

## Consequences

- Phase numbers change again: scripting moves from 11 to 16, and rigging and animation from 12 to 17 and 18. Older decision records keep the numbers they were written with.
- "Rendering beyond a good-looking viewport" is no longer out of scope.
- From Phase 9 on, every pull request that adds text adds it to the English package.
