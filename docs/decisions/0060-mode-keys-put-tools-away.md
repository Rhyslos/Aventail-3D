# 0060. Mode keys put a tool away

- **Status:** Accepted
- **Date:** 2026-10-14

## Problem

While testing, the project lead kept finding a tool still in use when he wanted to select, and noticed he reaches for 1, 2 and 3 to get back to selecting. With a tool in use, though, the key for the mode he's in took him out to the Model mode, and a click still went to the tool.

## Options

1. **Tools put themselves away after one use.** Removes the cause, but repeating an edit needs its key every time.
2. **A key that swaps to the previous tool** (Space, as in Cinema 4D), and the pointer showing the tool. Good, but one more key to learn.
3. **The mode keys put a modelling tool away**, as he asked: with a tool in use, 1, 2 or 3 picks the Select tool and goes to (or stays in) that mode.

## Decision

Option 3, the project lead's choice. With a modelling tool in use (one with settings: Extrude, Inset, Bevel, Loop cut, Slit, Bridge, Weld), a mode key, or a mode button on the left palette, puts it away (finishing a Slit line) and selects in that mode, never leaving it. With no tool, or Move, Rotate or Scale (which select with a click anyway), the keys work as before: a mode, or back to the Model mode when pressed again. Tab still steps all the way back to the Model mode.

## Consequences

- Getting back to selecting is the key he already reaches for.
- Leaving an element mode with its own key needs the tool put away first (or Tab).
- Options 1 and 2 stay open if the friction remains.
