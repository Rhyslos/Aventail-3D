# 0050. Pipe bends: an extrude that turns a corner and keeps its size; file format version 8

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0038](0038-extrude-and-inset.md) (extrude), [0048](0048-ctrl-wheel-changes-a-tools-count.md) (Ctrl + wheel), [0028](0028-ask-about-controls.md); the 2026-10-13 plan for pipe bends that keep their size

## Problem

The project lead needs pipes that bend for the simple prop that tests the Phase 4 gate. In most programs, turning a pipe's end ring pinches the pipe: the joint narrows. The plan had a "keep round" idea and moved from Phase 7 to Phase 4 at the project lead's request.

## Options

1. **Extrude with a bend** (chosen): the usual extrude, plus Angle, Direction, Bend radius and Segments in the Attributes panel.
2. **Rotate with Keep round**: turning a ring with R, with a switch that keeps it round. Offered, not chosen.

For the bend's direction: settings in the Attributes panel (chosen) or dragging sideways.

## Decision

1. **Controls.**
   - Select the pipe's end polygon, press `D` and drag the length as usual.
   - **Angle**, **Direction**, **Bend radius** and **Segments** show in the Attributes panel:
     - for the next drag, while Extrude is in use;
     - as the Last edit afterwards. A straight extrude given an Angle there becomes a bent one.
   - Ctrl + wheel changes Segments while Extrude is in use, following 0048. Extrude now has a count, so Ctrl + wheel no longer grows the selection while Extrude is in use.
2. **The shape.** An extrude with a bend first turns through the elbow, then goes on straight for its Distance (0: just the elbow).
   - **With a bend radius:** the turn is made of Segments rings, each the piece's border turned rigidly around a point Bend radius away from its middle, in the bend's Direction. Every ring keeps the full size, so nothing pinches.
   - **With no radius:** the turn is a sharp mitre joint. The piece's old border moves onto the plane halfway between the two directions, stretched across the turn by 1 / cos(half the angle). It stays on the pipe's side and meets the new piece exactly.
   - **Limits:** Angle 0 to 180°, or up to 150° without a radius, because the stretch grows without limit towards 180°.
   - **Direction 0** turns up: for an upright pipe, towards +X. Angles then go round the pipe from there.
3. **The edit.** `Operation::ExtrudeBent` holds polygons, distance, grouping, a `Bend` and anchors.
   - Each piece (Keep together) bends around its own middle.
   - The size guard counts every ring.
   - A bend that doesn't turn is a plain extrude.
4. **File format version 8.** Version 7's operations, plus bent extrudes. On opening, every number must be real; the bend is brought into range.

## Consequences

- Pipes, rails and bent tubes can be modelled by extruding a cylinder's end again and again, straight or bent. Each bend can be changed as the Last edit.
- A mitre joint moves the old end ring. That is what makes the joint clean, but the previous piece changes length at its edge.
- A bend radius smaller than the pipe's own radius folds the inside of the turn, as it would in reality. The tip in the manual says so.
- Not done: rotating a ring with Keep round, and bends along curves. The second can come with curves and splines in Phase 7.
- Projects saved now need this version of the app or newer.
