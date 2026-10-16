# 0054. Symmetry: a live mirror while moving, rotating and scaling points

- **Status:** Accepted; extended by [0059](0059-symmetry-for-every-edit.md) (every edit mirrored; the axes moved to the Symmetry menu); replaced for new work by [0088](0088-live-mirror.md) (the live mirror)
- **Date:** 2026-10-14
- **Follows:** [0028](0028-ask-about-controls.md) (ask about controls), [0030](0030-transform-gizmos.md) (the gizmos), [0032](0032-edits-find-their-points-again.md) (anchors)

## Problem

Most props are symmetrical. Modelling one half and having the other follow halves the work and keeps the halves matching. This needed a switch, a choice of axes, and a precise meaning.

## Decision

1. **Controls** (the project lead's choice): a **Symmetry** switch on the top palette (also **Tools → Symmetry**), no key. While it is on, the Attributes panel shows a **Symmetry** section at the top with **X**, **Y** and **Z** switches. X is on as standard. Both are remembered with the view's other choices.
2. **What it does.** In the Points, Edges and Polygons modes, moving, rotating or scaling points (with the gizmo or the Coordinates panel) also moves their **mirrors**: the points at the mirrored position across the shape's own X, Y or Z plane, through its own middle (its origin).
   - The mirror is matched by position within 0.01 cm, once, when the change begins.
   - A mirror moves the mirrored way: a point moved right takes its mirror left.
   - A point on a mirror plane stays on it, so the middle never splits open.
   - With two or three axes on, every combination is mirrored (X and Y: four copies).
   - If both a point and its mirror are chosen, the one with the lower number leads and the other follows, so the shape stays symmetrical.
   - A point with no mirror (the shape isn't symmetrical there) just moves.
   - Moving whole objects, and the other tools (extrude, bevel and the rest), are not mirrored yet.
3. **The edits.** Each group of points that moves the same way is recorded as its own move or transform (`MovePoints` or `TransformPoints`, with its points' anchors), all in one undo step. No new kind of edit, so the file format is unchanged.

## Consequences

- Half a shape can be shaped by moving points, and the other half follows.
- The modelling tools still work on one side only; mirroring them, or a mirror generator that builds the other half from one (Phase 7), would come later.
- Matching by position means a shape that isn't symmetrical where you work moves only the chosen points, which is what you'd expect.
