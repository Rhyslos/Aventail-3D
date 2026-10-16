# 0083. Groove, and more control over Bevel (plan)

- **Status:** Accepted (planned, not built yet)
- **Date:** 2026-10-15
- **Follows:** [0040](0040-bevel.md) (bevel), [0082](0082-edits-keep-their-shape-on-stretched-objects.md) (edits on stretched objects)

## Problem

The project lead asked for two things while modelling a prop.

1. **A quick way to cut thin lines into a shape, or raise them out of it** (panel lines, seams, raised ribs). Today this takes two steps: a bevel turns the edge into a thin strip, and an extrude pushes the strip in or out.
2. **More control over Bevel.** Bevels often go uneven, with jagged lines where they meet and no way to fix it. The project lead asked for a "weld points that overlap" option and a skew slider that puts more of the bevel on one side of the edge than the other.

## Options

- **Name of the new tool (asked, the project lead chose):**
  - **Groove** (chosen): clear whether it goes in or out;
  - Panel line;
  - Crease. In 3D programs a crease usually means an edge kept sharp when a model is smoothed, which the planned smoothing modifier will have, so the name would clash.
- **Its key (asked, the project lead chose):**
  - Shift + V, next to Bevel;
  - **K** (chosen): one of the last free letters. In Blender K is the knife tool; here cutting along a line is Slit (X), so it is free to mean Groove;
  - no key, menu only.
- **Weld overlapping points (asked):** **on by default** (chosen), or off by default.
- **When (asked):** **next, right after the open pull requests are merged**, as a small milestone before Phase 6 (chosen); or within Phase 7's improved bevel.

## Decision

**Milestone "Bevel control and Groove"**, two pull requests, Bevel first because Groove builds on it:

1. **Bevel control.** New settings in the Attributes panel, changeable afterwards under Last edit:
   - **Weld overlapping points**, on by default, with a small distance: after bevelling, points that land on top of each other become one.
   - **Skew**, −100% to +100%: more of the bevel on one side of the edge, less on the other. A **Flip side** button swaps the side, since a single edge has no natural left and right.
   - **Even width:** the width measured straight across the edge, so bevelled edges look equally wide whatever angles their polygons meet at. This is the main cure for uneven bevels.
   - **Stop at overlaps:** a wide bevel stops where it would cross a neighbouring one instead of folding over.
2. **Groove** (key **K**, also in the Tools menu and the E menu). Select edges (one, a line or a loop) and drag. Each edge becomes a strip as wide as **Width**, pushed in (a groove) or out (a ridge) by **Depth**, which the drag sets. Its **Sides** are straight (a crisp panel line) or sloped (a V cut). Where a line of edges ends inside a surface, the groove closes off neatly. It is a recorded edit: changeable under Last edit, made on both sides with Symmetry, worked out as seen on stretched objects, keeping materials.

The details (exactly how skew picks a side along a loop, the weld distance, Groove's ends) are worked out and tested when each is built, and recorded in its own decision then.

## Consequences

- Phase 7's "Improved bevel" shrinks to mitre patterns and rounded corners; clamping comes here as Stop at overlaps.
- K is taken, which leaves U as the only free letter (likely for UVs in Phase 8).
- Both edits change the project file format when built.
