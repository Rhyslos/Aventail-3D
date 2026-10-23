# 0137. Phase 10 built biggest and hardest first

- **Status:** Accepted
- **Date:** 2026-10-24
- **Builds on:** [0136](0136-more-modifiers.md) (more modifiers for Phase 10)

## Problem

Phase 10 has 23 milestones, from huge (curves, a rounded-corner bevel, remesh) to small (weld by distance, a triangulate preview). In what order should they be built?

## Options

1. **Biggest and hardest first (chosen, the project lead's choice):** the risky work is done while there's the most time and attention for it, and the phase ends with quick wins.
2. Small ones first, for quick new tools to test.
3. The order of decision 0136, roughly by theme.

## Decision

Biggest and hardest first, except that a milestone another one builds on comes before it. By size: curves and splines; improved bevel; remesh; smoothing with creases; lattice cage (its deformer groundwork is reused by bend, twist and taper); decimate; scatter; curve deform and shrinkwrap (after curves); weighted normals; live bevel (after the improved bevel); quadify; collision hull; **weld by distance, moved up** because array uses it to join its copies; array and mirror; shell; LOD generator (after decimate); game bevel (after live bevel and weighted normals); displace and noise; bend, twist and taper; wireframe; modular kit helpers; triangulate preview.

## Consequences

- The README's Phase 10 list is in this order.
- Each milestone still asks about its controls first ([0028](0028-ask-about-controls.md)).
