# 0167. The mode palette's names no longer slide out

- **Status:** Accepted
- **Date:** 2026-10-25
- **Replaces part of:** [0126](0126-rework-and-first-use.md) (the mode palette's names slide out on hover) and [0140](0140-icons-only-top-palette.md) ("the mode palette's names that slide out on hover stay")

## Problem

Resting the mouse on the mode palette slid out a strip with each button's name. The project lead found it in the way: it covered part of the viewport whenever the mouse passed over the palette. Every button already names itself, and says what it does, in its tooltip, as the top palette has done since decision 0140.

## Decision

The strip is gone. The mode palette is icons only, like the top palette; hovering over a button shows its tooltip, which starts with its name. Clicking works as before. No keys change.

## Consequences

- The viewport is never covered by the strip.
- Someone new to the app reads the names one tooltip at a time rather than all at once. The manual's interface tour lists every button.
