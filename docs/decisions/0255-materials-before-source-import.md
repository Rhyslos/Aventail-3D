# 0255. Materials before Source import; Source import last

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

With Phase 8 finished, the next thing on the roadmap was Source engine import ([0222](0222-source-import.md)), then materials with a node editor (Phase 12). The project lead finds materials more important.

## Options

Where Source import goes once materials move to the front (asked of the project lead):

1. **After bug fixes and polish** (Phase 13), before scripting.
2. **Right after materials.**
3. **With rigging**, building all its stages together just before Phase 15.
4. **At the very end**, after effects (Phase 18). Chosen.

## Decision

Materials with a node editor (Phase 12) is built next, after its planning talk. Source engine import moves to the end of the roadmap, after Gauntlet (Phase 18). Its plan in [0222](0222-source-import.md) stands otherwise. Since rigging and animation will exist by then, its three stages (shapes and materials; bones and weights; animations and flexes) can be built one after another rather than waiting for those phases.

## Consequences

- The next planning talk is about materials and the node editor (DESIGN.md §10, "To be designed: Texturing and a material builder").
- Source models can't be opened for a long while; OBJ and glTF stay the ways in.
- Nothing already built changes.
