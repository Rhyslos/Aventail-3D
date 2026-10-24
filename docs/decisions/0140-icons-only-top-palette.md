# 0140. The top palette icons only, with a Transform button

- **Status:** Accepted
- **Date:** 2026-10-24
- **Replaces part of:** [0126](0126-rework-and-first-use.md) (words beside the top palette's main tools)

## Problem

The rework (Phase 9) wrote names beside the top palette's select tool, Move, Scale, Rotate, Snap and Symmetry when the window was wide. Trying it, the project lead wanted "just the selection symbols as it used to be". They also missed Transform (all three gizmos, T), which had a key and a menu entry but no button, and noticed that the select tool list didn't look like the app's other menus.

## Decision

- The top palette is icons only, at every width; hovering still names each button. The mode palette's names that slide out on hover stay.
- **Transform** gets its own button after Rotate. Only the tool in use is lit (before, Transform lit Move, Scale and Rotate together).
- The select tool list is an ordinary menu, like the viewport's Shaded menu: the tool in use highlighted, not a typed-in tick. A reference picture keeps it so.

## Consequences

- The palette takes less room, and its names live in the tooltips, the Keys window and command search.
