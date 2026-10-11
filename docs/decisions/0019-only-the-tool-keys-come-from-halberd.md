# 0019. Only the tool keys come from Halberd

- **Status:** Accepted; amended by [0028](0028-ask-about-controls.md) (the project lead now chooses other keys before they are built)
- **Date:** 2026-10-10
- **Narrows:** [0016](0016-controls-follow-halberd.md)

## Problem

Decision 0016 said every key and mouse action copies Halberd. The project lead has clarified what they meant: Halberd already has the tool keys they want here (`Q` select, `W` move, `S` scale, `R` rotate, `T` all transform gizmos, and `B` for a new shape), and those are the ones to keep. Copying everything else from Halberd, and checking its keymap before every new key, was more than intended.

## Options

1. **Keep copying all of Halberd's controls** (0016 as written).
2. **Keep only the tool keys from Halberd**, and choose every other key on its own merits.

## Decision

Option 2, from 2026-10-10 on:

- **From Halberd:** `Q` select tool, `W` move, `S` scale, `R` rotate, `T` all transform gizmos together, and `B` to make a new shape. (They arrive with the tools, from Phase 3.)
- **Everything else** is chosen for this app: keys that are common in 3D modelling programs and easy to remember, that don't clash with the list above, explained in plain words in the pull request that adds them so the project lead can ask for a change. Halberd's keymap is no longer checked.
- **Another Halberd feature or key** is copied only when the project lead names it.
- **Keys already built stay**, as they are what most programs use anyway: `Ctrl + Z`, `Ctrl + Y` and `Ctrl + Shift + Z` (undo, redo), `Delete` and `Backspace` (delete), `Esc` (select nothing, cancel), `H` and `F` (frame all, frame selected), `F1` to `F5` (views), `Alt + G` and `Shift + G` (group, ungroup), and `Ctrl + Q` (quit). File keys follow the same common convention when they arrive (`Ctrl + N`, `Ctrl + O`, `Ctrl + S`, `Ctrl + Shift + S`).
- **No longer planned** (they came from 0016, not from a request): Halberd's right-mouse fly camera with `WASD`, its Keybinds viewer, `Ctrl + W` for picking inside groups, and its keys while drawing a shape. Any of them can come back if the project lead asks.
- Viewport navigation keeps its own schemes ([0015](0015-navigation-schemes.md)).

## Consequences

- Fewer constraints when choosing keys; each new key is explained where it is added.
- The two Rhylib tools share the keys that matter most day to day: picking and transforming.
