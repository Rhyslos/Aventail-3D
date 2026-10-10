# 0015. A choice of navigation controls

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

The viewport moved only with Alt + mouse. The project lead wants to try other ways of moving the view (for example, without holding a key) before choosing the standard, and asked for a picker in the viewport's bottom-right corner. They also asked that schemes are not named after other programs.

## Options

1. **Fully custom bindings:** any button and key for any movement. Flexible, but a lot of settings to understand for someone rusty at 3D, and easy to break.
2. **A few ready-made schemes**, named by what they do, plus a couple of switches.

## Decision

Option 2, kept in `tools::controls`, separate from the movements themselves (`tools::navigation`):

| Scheme | Orbit | Pan | Zoom |
|---|---|---|---|
| Alt + mouse (standard for now) | Alt + left | Alt + middle | Alt + right, wheel |
| Mouse only | right | middle | wheel |
| Mouse only, middle zooms | right | Shift + middle | middle, wheel |
| Middle pans, Alt orbits | Alt + middle | middle | wheel |
| Middle orbits | middle | Shift + middle | wheel |

- **Alt + mouse works in every scheme** (on by default), wherever the scheme doesn't use the same keys itself.
- **Invert orbit direction**, off by default.
- **A plain left drag never moves the view**, in any scheme: it stays free for selecting and the modelling tools.
- **A right click without dragging** still works for menus later, because a drag only starts once the mouse moves.
- Every scheme zooms with the wheel. The choice is remembered between sessions.

## Consequences

- Trying a scheme is one click, and choosing the standard later is a one-line change.
- Picking the pivot from the point under the mouse (the next milestone) applies to every scheme at once.
- If people later want their own bindings, a "Custom" scheme can be added without changing the others.
