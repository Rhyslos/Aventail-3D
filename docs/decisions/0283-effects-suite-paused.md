# 0283. The effects suite is paused

- **Status:** Accepted
- **Date:** 2026-11-13

## Problem

Phase 18, **Gauntlet**, was planned as the app's effects suite: physics, particles and simulations, baked for game engines. The project lead is building an effects suite in their game engine, Solis, and doesn't need two.

## Decision

- **Gauntlet is paused**, not cancelled: it moves to the later list in DESIGN.md, and comes back only if the project lead asks for it.
- **Sculpting stays Phase 17**, in the same place (the project lead: "I will need a sculpting suite"). Nothing else moves.
- **Source engine import** stays last, now after sculpting rather than after effects.

## Consequences

- The roadmap after Phase 12 and seam blending is: engine shader translators, bug fixes and polish (Phase 13), scripting (14), rigging and skinning (15), animation (16), sculpting (17), then Source engine import.
- Effects for games made with the app come from the game engine. Decision 0216 (sculpting) is unchanged.
