# 0085. Bevel control: even width, skew, stop at overlaps, weld

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0040](0040-bevel.md) (bevel) and the plan in [0083](0083-groove-and-bevel-control.md)

## Problem

Modelling a prop, the project lead found bevels hard to control. Lines came out uneven, there was no way to put more of a bevel on one side, and where bevels met they left a mess. He asked for **weld points that overlap** and a **skew** slider (0083).

Looking at why bevels go uneven turned up two more causes:
- A bevel reaches the same distance into each polygon, so its new polygon is `2 × width × cos(θ / 2)` wide, where θ is the angle between the polygons' facings. A flatter edge's chamfer comes out much wider than a right-angled one's.
- A wide bevel's corners pass each other and fold over.

## Options

- **Skew's side:** an edge has no natural left and right, and the polygon "forward" along an edge's stored direction flips from edge to edge.
  - **The side follows each chain of selected edges round, with a Flip side switch** (chosen), so a loop's skew stays on one side.
  - A side per edge, picked from the stored direction; it would zigzag round a loop.
- **Even width:**
  - **sized by the edge's angle so its new polygon is a right-angled edge's width** (chosen); a box looks exactly as before;
  - the width as the new polygon's width, which would change the meaning of every existing Width.
- **Stop at overlaps:**
  - **narrow the whole bevel until its corners meet halfway along edges between bevelled points, and stop short of the end of plain edges** (chosen, as Blender's clamp);
  - narrow only the corners that cross, which leaves the bevel uneven again.
- **Weld:**
  - **weld the bevel's new points within a distance, 1 mm as standard** (on by standard, the project lead's choice);
  - weld every point of the shape, which could join unrelated parts.

## Decision

1. **`BevelOptions`** on `BevelShape`: `skew` (-1 to 1, reaching `SKEW_REACH` 0.9 so neither side shrinks to nothing), `flip_side`, `even`, `clamp`, `weld`, and `weld_distance` (0 to 10 cm, `WELD_DISTANCE` 1 mm).
   - New bevels default to even, clamp and weld on, with no skew.
   - `BevelOptions::EARLIER` (all off) is what older bevels keep.
2. **The bevel** works out each edge's width on each polygon: the width, times the even factor (`cos 45° / cos(θ / 2)`, the cosine kept at least 0.25), times 1 ± the skew on the chain side (`chain_sides`). Corners and slides use the two edges' own widths.
   - With clamp, a first pass finds how much narrower everything must be to fit, and the bevel is made at that scale.
   - Then, with weld, `join::weld_close` welds the new points within the distance. Its grid of cells finds close points fast, and points join in chains.
3. **`join::weld`** now shares `weld_groups` with `weld_close`, welding several groups in one pass.
4. **The Attributes panel** shows the settings under Profile, for the next bevel and under Last edit:
   - Skew is a slider with a Flip side button beside it;
   - Even width, Stop at overlaps and Weld points are switches;
   - Weld distance is a length, greyed out without welding.
   - `EditSettings::bevel_options` carries them.
5. **File format 23** stores a bevel of edges with any of these as version 22 stores it without them, with the settings beside it. Older writers refuse such a bevel rather than drop the settings, and older files open with `EARLIER`.

## Consequences

- Bevels on curved and angled props come out even and clean by standard, and old projects look exactly as they did.
- Phase 7's improved bevel is now mitre patterns and rounded corners; clamping is done.
- Bevelling points keeps working as before; these settings are for edges.
- Groove (0083) can build on a skewed, even bevel.
