# 0152. Weld by distance

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack and its recipe), [0150](0150-quadify.md) (Quadify: a modifier and a one-off in Mesh → Clean up, the pattern followed here), [0051](0051-connect-bridge-weld.md) and [0128](0128-weld-polygons.md) (Weld, M and Ctrl + M, whose melting this reuses), [0085](0085-bevel-control.md) (`weld_close`, the bevel's clean-up), [0032](0032-edits-find-their-points-again.md) and [0059](0059-symmetry-for-every-edit.md) (recorded edits find their points again, and work with symmetry), [0098](0098-seams.md) (seams), [0110](0110-controllable-automation.md) (automatic tools show their settings and give the same result every time), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0151](0151-collision-hulls.md) (the development file format moves to version 53)

## Problem

Phase 10's fourteenth milestone is **Weld by distance**: points closer together than a **Distance** (recommended 0.1 mm) melt into one, with the options **Only at open borders** and **Keep UV seams**, as a modifier and as a one-off command in the Mesh menu's Clean up group, using a grid hash, shaped so the array modifier (the next milestone) can call it to join its copies.

A shape can look whole and be in pieces: a model from another program or a game file is split along every UV seam and hard edge (glTF and most engines store it that way), pieces snapped edge to edge touch without joining, and scaling to zero or bevelling leaves points almost on one spot. Each such pair is a crack: loop cuts and selections stop at it, Subdivide pulls it open, it shades as a line, and the mesh check calls the shape open.

Left to decide: the names (Weld, M, already exists), which point a group keeps and where, whether close points chain, what *Keep UV seams* means when welding never moves a UV, what *Only at open borders* takes, the one-off's modes and settings, where the modifier sits in the list, the Distance field's units, and the function the array calls.

## Options

The names:
1. **"Weld by distance" for the modifier, "Weld by distance now" for the one-off (chosen).** The app already says *Weld* for melting points (decision 0051) and *Weld the middle now* for symmetry's (0059); "by distance" says what's new. "Now" sets the one-off apart from the modifier in command search, as *Quadify now* does.
2. "Merge by distance" (Blender's name): reads clearly, but brings a second word for the same act into the app, against one meaning per label (0046, 0126).

Groups:
1. **Lowest id first, no chains (chosen).** Points are taken in order of id; each one not yet taken keeps its place and takes every untaken point within the distance of it. A point never travels further than the distance, and the result is the same every time. This is what Blender's *Merge by Distance* does.
2. Chains (as `weld_close`, the bevel's clean-up, does): points within the distance of each other, and so on along a chain, melt at their middle. On a dense shape, a row of points closer than the distance would collapse to one, however long: fine for bevels' tiny leftovers, wrong for a setting a person raises to close a gap.

Where the kept point goes:
1. **It stays put (chosen):** the others come to it. Copies laid side by side keep the first copy's points exactly where they were; and nothing moves by more than the distance.
2. The middle of the group: symmetric, but moves every point of every join a little.

*Keep UV seams* (welding never changes a corner's UVs, so the texture never moves either way):
1. **Joins where the UVs don't meet are marked as UV seams (chosen),** on a shape that has seam marks. Unwrap cuts along the marked seams when a shape has any (`uv::unwrap::cuts_of`), so without the mark a later Unwrap would join two pieces the person laid out apart. A shape with no seam marks at all gets none, since Unwrap already cuts it where its UVs split, and a single new mark would make Unwrap ignore every other split.
2. Points whose corners' UVs differ aren't welded at all: keeps the shape open along every UV seam, which is how it came from the game file; and copies of an array would never join (the right edge of one copy is at u = 1, the left of the next at u = 0).

*Only at open borders*:
1. **Only points on an open border (an edge with one polygon) are candidates, and only weld with each other (chosen).** Cracks are always on open borders; small details elsewhere never melt, whatever the distance.
2. Weld a border point with any point: closes a border onto a surface it touches, but that makes edges of three polygons and isn't what a crack is.

The one-off's modes:
1. **The Model mode (whole shapes) and the Points, Edges and Polygons modes (the selected elements' points, welding only with each other) (chosen).** Points are what weld, and a person often has edges or polygons selected along a crack.
2. The Model and Points modes only: Quadify uses two modes, but greying out Edges and Polygons here would only send people to press 1.

The Distance field: the length fields show down to a millimetre (`ui::units::Units::decimals`), so 0.1 mm would read "0".
1. **Millimetres whatever the units (chosen):** a field of its own (`stepper::millimetres`, two decimals, arrows step 0.1 mm), as the distance is almost always tiny.
2. More decimals for every length field: changes how every other field reads.

## Decision

### The work (`mesh::weld_distance`, layer 0)

- `WeldOptions` (`Copy`, `Default`, `clamped`): **Distance** in centimetres (standard **0.01**, 0.1 mm; 0 to 100 cm, values that aren't numbers become the standard), **Only at open borders** (standard **off**), **Keep UV seams** (standard **on**).
- `weld_groups_of(mesh, points, options)`: the candidates (the given points, or all; at real places; on an open border with *Only at open borders*) go into a **hash grid of cells `distance` wide** (at least 1e-6), so each point is compared only with its own and the 26 neighbouring cells; taken points leave their cell as the work goes on, so a crowded cell empties. Lowest id first as above. Each group is listed kept point first.
- `weld_by_distance(mesh, points, options) -> usize` welds the groups in place, all in one pass, through Weld's own melting (`join::weld_groups`, made `pub(crate)` with its `Group`): polygons keep their ids, every corner keeps its own values (UVs and the rest), corners repeated next to each other go, polygons left with fewer than three corners or folded onto themselves go, points left with no polygon go, and the edges that went to the kept point take their values (hard, seam, crease, bevel weight). Then *Keep UV seams* as above. Returns how many points went (0: the mesh is unchanged).
- `welded(mesh, options)` is the modifier's: refused, with a reason saying what to change, when nothing is close (`WeldProblem::NothingClose`) or *Only at open borders* is on and the shape has none (`NoBorders`).
- Never panics: points not at real places are skipped, coordinates too large for the grid share its end cells (the distance test still sorts them out), a distance of 0 welds only points exactly on one spot, a huge one is clamped to 1 m; empty meshes, n-gons, open borders and edges of three or more polygons are handled. Deterministic.
- Timing (release build): 90 000 points of a grid exploded into separate quads, 67 000 of them welded, in 70 ms.

**For the array modifier.** Array places its copies side by side in one mesh, then calls `mesh::weld_distance::weld_by_distance(&mut mesh, None, &WeldOptions { distance, only_borders: true, keep_seams: true })` (its own *Merge* distance): only the points where copies touch are border points close enough to weld, and each copy's UVs stay as they were with the joins marked as seams. It can pass `Some(points)` to weld only the points of the copies' touching ends.

### The modifier

`ModifierKind::WeldByDistance(WeldOptions)`, family **Clean up**, after Quadify on the palette and in Mesh → Modifiers, following decision 0142's recipe. It reads no other object and gives the viewport and exports the same. `work` is a fifth of the input's points (`WELD_POINTS_PER_POLYGON`), so from about 25 000 points the viewport welds on another thread. A new Weld by distance **goes before** any Decimate, Quadify, Live bevel, Subdivide or Weighted normals in the list (`goes_before`): each reads how polygons join, and a crack spoils it (Subdivide pulls it open, Decimate and Quadify keep it as a border, Weighted normals shades it as a hard edge). Up and Down still move it.

### The one-off: Weld by distance now

`Operation::WeldByDistance { points, options, anchors }`: the points (none for the whole shape), the settings, and where the points were (0032, anchored as points whatever mode chose them). Its undo label is "Weld by distance" (whole shapes) or "Weld 3 points by distance". With symmetry on, chosen points weld together with their mirrors (0059); a whole-shape weld already covers both sides. Its settings are `EditSettings::weld`, shown under Last edit (and in an edit opened from the edit list) by the same rows as the modifier's, and remembered for the next one (`AttributesPanel::weld`). It doesn't mend UVs: every corner keeps its own (`mends_uvs` false).

In the interface, `CommandId::WeldByDistanceNow` ("**Weld by distance now**") joins Quadify now in **Mesh → Clean up**, first, and command search (under Mesh): in the **Model mode** it welds each selected shape whole (instances and curves left out), in the **Points, Edges and Polygons modes** the points of the selected elements; in the UV mode it is greyed out with the reason. One edit per object, one undo step; the status bar says how many points went. It is recorded even when nothing is within the distance (as long as two or more points were chosen), so the Distance can be raised under Last edit.

### Saving

Development file format **version 53**: version 52's model with `KindV43::WeldByDistance(WeldV53)` (the three settings, clamped on reading) and `OperationV37::WeldByDistance(WeldEditV53)` (points, settings, anchors; a distance that isn't a number refused as damaged). New sample `sample-v53.avtl`; files before 53 never hold either.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** The modifier: the palette button, **Mesh → Modifiers → Weld by distance** (under *Clean up*) and command search. The one-off: **Mesh → Clean up → Weld by distance now** and command search; not on the E ring. Weld (**M**, **Ctrl + M**) is unchanged.
- The names **Weld by distance** and **Weld by distance now** (rather than Blender's *Merge by distance*), matching the app's *Weld*.
- Standard **Distance 0.1 mm**, **Only at open borders** off, **Keep UV seams** on, as recommended. The Distance is shown **in millimetres** whatever the project's units, from 0 to 1 m.
- **The lowest-numbered point of each group stays where it is**, the others come to it, and points are **never pulled along a chain** (Blender's way).
- **Keep UV seams** means: where joined pieces' UVs don't meet, the join is **marked as a UV seam** (on shapes that have seam marks), so Unwrap keeps the pieces apart. Welding never changes UVs either way.
- **Only at open borders** welds border points only with each other.
- Weld by distance now works in the **Model mode** (whole shapes) and the **Points, Edges and Polygons modes** (the selected elements' points, welding only with each other).
- A new Weld by distance goes **before** any Decimate, Quadify, Live bevel, Subdivide or Weighted normals in the list.
- Weld by distance now is **recorded even when nothing is close**, so the Distance can be raised under Last edit; its settings are remembered.
- The modifier with nothing to weld shows the shape unchanged with an amber reason, as Quadify does.

## Consequences

- Projects with a Weld by distance modifier or a Weld by distance now edit need this version of the app or later.
- The array modifier (milestone 15) joins its copies by calling `weld_by_distance` with *Only at open borders* on, rather than writing its own merge.
- An imported model can be closed in one step, then quadified (Quadify now) to edit as quads.
- Not yet: marking the joins of an import's hard-edge splits as hard edges (the shading follows the object's Smoothing once welded; Blender's *Sharp edges* option), and welding a border onto a surface it touches.
