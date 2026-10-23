# 0133. The key audit: three changes, three keys kept; Ctrl snaps one loop cut

- **Status:** Accepted
- **Date:** 2026-10-23
- **Builds on:** [0028](0028-ask-about-controls.md) (ask about controls), [0047](0047-loop-cut.md) (loop cut), [0055](0055-basics.md) (slide, hide and isolate), [0126](0126-rework-and-first-use.md) (the rework)

## Problem

Phase 9 milestone 8 is a key audit: every key the app has, checked for clashes and for what newcomers expect. Six changes were proposed in a shared document, and the project lead ticked the ones to make. Separately, the project lead asked for a quick way to put one loop cut dead centre: until now that took three cuts and deleting two.

## Decision

The project lead's ticks:

- **Slide** moves from Alt + drag on the Move gizmo to **Shift + Alt + drag**. Alt + drag is how the view moves, so starting a drag on the gizmo with Alt out of habit slid points instead. Now Alt + drag always moves the view, even over the gizmo.
- **Show everything** gets a second key, **Ctrl + Shift + H**, beside Alt + H.
- **On a Mac every text says Cmd** where it says Ctrl, through the language layer ([0132](0132-one-language-layer.md)). The Keys window already did.

Not ticked, so they stay as they are:

- Frame selected stays on **Z** and Loop cut on **F** (not F and Ctrl + R).
- **Ctrl + wheel** still grows or shrinks the selection when no tool is in use (not Ctrl + Plus and Minus).
- Weld onto a point stays on **Ctrl + M** (not Shift + M).

The audit also kept A for Connect and Space for command search.

**Ctrl snaps one loop cut** (the project lead's own design): with Cuts at 1, holding Ctrl while pointing, clicking or dragging puts the cut on the nearest of nine places along the edge, a tenth apart; the fifth is the middle. It never lands on an end. Several cuts are spread evenly already, so Ctrl changes nothing for them. Ctrl + wheel still changes the number of cuts.

## Consequences

- People who slid with Alt + drag now add Shift; the Keys window, the tooltips and the manual say so.
- Holding Ctrl to turn the wheel while Loop cut is in use also snaps the cut shown, which does no harm: the snap only matters for the click.
- Milestone 8 is done.
