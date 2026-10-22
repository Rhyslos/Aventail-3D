# 0125. Advanced modelling before the rest of the UV phase

- **Status:** Accepted
- **Date:** 2026-10-22
- **Replaces part of:** [0121](0121-rework-before-the-rest-of-phase-8.md) (Phase 8 continuing straight after Phase 9)

## Problem

Phase 8 (UV powerhouse and baking) is paused at 7 of 28 milestones for the rework (Phase 9). Decision 0121 had it continue straight after. The project lead found unwrapping already good enough for his work, and wants modelling tools (booleans, better bevel, curves, smoothing) first: "modelling takes priority".

## Options

1. **Advanced hard-surface (Phase 10) right after Phase 9, then the rest of Phase 8 (chosen, his request).**
2. Keep the order: 21 more UV milestones before any new modelling tool.

## Decision

The order becomes Phase 9 (rework), Phase 10 (advanced hard-surface), the rest of Phase 8 (UVs and baking), then Phase 11 (texture painting). Phase numbers stay as they are, so links and records don't change; the plan says which comes next.

## Consequences

- The baker (in Phase 8) still comes before texture painting, which starts from baked maps.
- Phase 10's tools carry UVs and seams along from the start (decision 0043), so models made with them unwrap as before.
- Phase 8's detailed plan (`docs/plans/phase-8.md`) waits as it is.
