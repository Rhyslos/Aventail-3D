# 0145. Decimate

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack and its recipe), [0143](0143-subdivide-and-creases.md) and [0144](0144-lattice-cage.md) (the latest modifiers; the object's Smoothing in the modifier's context), [0053](0053-hard-and-soft-edges.md) and [0126](0126-rework-and-first-use.md) (hard edges, each object's own Smoothing), [0043](0043-attribute-layers-and-the-builder.md) (UVs, seams and edge layers), [0074](0074-material-slots.md) (material slots), [0035](0035-heavy-meshes.md) (heavy work off the interface thread), [0110](0110-controllable-automation.md) (automatic tools show their settings and give the same result every time), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0144](0144-lattice-cage.md) (the development file format moves to version 46)

## Problem

Phase 10's seventh milestone is Decimate: a modifier with a **Ratio** (recommended 50%) or a **Target polygons**, quadric error collapse (Garland and Heckbert) with attribute-aware costs, keeping UV seams, hard edges, material borders and open borders (a switch each, all on) and the shape's silhouette, triangles out with an optional "keep quads where flat", deterministic, and quick: 100 000 to 10 000 polygons in under a second in a release build. The LOD generator (milestone 17) will make several levels from one shape with it, so the function has to suit that too.

Left to decide: what is counted, the kind of collapse, what "keep" means for each kind of line, the attribute costs, the checks that keep the surface whole, the quads, which hard edges count, speed, the Attributes rows, and the shape of the function for the LOD generator.

## Options

What the target counts:
1. **Triangles (chosen):** what a game engine draws and what a budget means; a quad is two. The result is triangles anyway, so a Ratio of 50% halves what the engine sees. The setting is called **Target triangles**, not *Target polygons*, so it says what it counts.
2. Polygons: a 50% ratio of a quad mesh would give as many triangles as it had quads, the same weight in a game.

The collapse:
1. **Half-edge collapse (chosen):** one end point moves onto the other, so every point left is one of the shape's own, never moved. The result lies exactly on the surface, UVs stay on their own corners, there is no solving for a new place (which can fail on flat or degenerate input), and levels made one after another are nested: a coarser level's points are some of the finer one's, so levels of detail swap without the outline swimming.
2. Full collapse to the quadric's best place: a little closer to the shape for the same count, but new places and new UVs to make up at every collapse, and nested levels drift.

What "keep" means:
1. **A kept line stays a line (chosen):** a point on one kept line (two kept edges of the same kind meeting there, turning by no more than 30°) may only slide along it, onto its neighbour on the line, weighed against planes standing on the line so a straight line stays straight; where lines meet, end or turn more than 30° (a square border's corner) the point never moves. The line keeps its course and corners, and still loses points where it can spare them. This is what most game tools do (meshoptimizer, Simplygon).
2. Every point on a kept line frozen: exact, but heavily seamed shapes (a UV sphere's poles and seam, a model with many pieces) would hardly reduce.
3. Only a higher cost: lines would wander under a strong ratio.

Which hard edges:
1. **Those the object's shading shows hard (chosen):** marked hard edges on an object shaded by its marks, edges sharper than its Smoothing angle on one smoothed by angle (`mesh::hardness::shading_hard_edges` with the context's `hardness`, decision 0143). The edges that *look* hard are kept: a box stays a crisp box whatever its smoothing.
2. Marked edges only: on the standard smoothing (by angle) the marks don't show, and a box's visible edges could be collapsed across.

The attribute costs:
1. **A UV slope quadric per UV corner group (chosen):** for each triangle the UV is a slope across its plane; a point moving with its UV costs the squared difference between that UV and what the original triangles gave at its new place, weighted by area (Garland and Heckbert 1998's idea, as meshoptimizer keeps it beside the place quadric). Collapses that would smear the texture come later. Positions are scaled into a box of size 1 first, so place and UV errors weigh alike whatever the shape's size.
2. A 5-dimensional quadric over place and UV together: the same idea, heavier to keep per point, and it mixes the two errors in a way that is harder to tune.

Quads:
1. **Triangles out as standard, Quads where flat as a switch (chosen):** with it on, pairs of triangles lying flat together (within 1.5°), with the same material and UVs running on across the edge between them, no kept line or mark on it, and no dent in the quad they make, are joined, the squarest first. The count stays in triangles. A full re-quadding is Quadify's job (milestone 12).
2. Quads where flat on as standard: tidier, but the notes call for triangles out first.

## Decision

### The work (`mesh::decimate`, layer 0)

`decimate(mesh, &DecimateOptions, hardness) -> Result<Mesh, DecimateProblem>`:

1. **Set up** (`decimate/setup.rs`): the polygons are cut into triangles (`face_triangles`, the cut drawing and export use); each point's corners are grouped by UV (a hundred-thousandth apart counts as the same) into *wedges*; each edge's triangles are found by sorting (deterministic, no hash order). An edge is an open border (one triangle), tangled (three or more: always kept), a seam (marked, or the UVs split across it), hard (as above) or a material border (two materials). The kept kinds (`Keep`) become lines, with planes standing on them (ten times a triangle's weight, by length²). Each point gets the planes of its triangles weighted by area; each wedge its UV slopes.
2. **Classify** each point: **Free**, **Line** (exactly two kept edges, the same kinds, turning by 30° or less) or **Locked** (lines meeting, ending or turning more; a tangled edge; or a point where the surface touches itself, with an open border count other than 0 or 2). Classes are worked out again round a collapse that touched a line or border.
3. **Collapse** cheapest first: each point's cheapest target among its neighbours its class allows (a Line point only along its line), by the place error at the target, the UV error of its wedges moved onto the target's (the one beside it across the edge, or the nearest in UV), and a tiny length term (a thousandth of the mean triangle area per length²) so flat areas, which cost nothing, go short edges first and stay even. The heap orders by cost, then point number: **deterministic**. Before a collapse it is checked: the **link condition** (the only neighbours the two points share are the far corners of the triangles on the edge), no **pinching** (two border points joined across the surface), no triangle turned by more than 75° or left with no area, and no **triangle made twice**. A candidate that fails is replaced by the cheapest that passes. Lines shorten as they go: a kept line's next edge, and the marks along it (hard, seam, crease), move to the point that is left.
4. **Output** (`decimate/output.rs`): the points left in their order, the triangles (or quads where flat), each corner's own UV, each triangle's polygon values (material and the rest) from the polygon it was cut from, and hard edges, seams and creases on the edges still there. Point values and other corner values aren't carried; loose points are left out.

A target at or above the triangles the shape has gives the shape unchanged. A point of a polygon that isn't at a real position is refused (`DecimateProblem::NotANumber`); a loose one doesn't matter. Settings: **Ratio** 0.1% to 100% (standard **50%**), **Target triangles** 4 to 10 million (standard **1 000**), and `target()` is at least one triangle; a ratio that isn't a number is the standard one.

### For the LOD generator (milestone 17)

`Decimator` is the work split in two, so several levels cost one decimation:

- `Decimator::new(mesh, keep, hardness)` sets up once (triangles, wedges, lines, quadrics, the heap);
- `reduce_to(triangles)` collapses until no more than that many are left, and can be called again with a smaller number to go on from where it stopped;
- `mesh(quads_where_flat)` gives the shape as it is now, and can be called between steps.

`decimate_levels(mesh, keep, hardness, &[targets], quads)` does exactly that for a list of targets, largest first. The LOD generator calls it with each level's target (`DecimateOptions::target` turns a ratio into triangles of the level-0 shape: 50%, 25%, 12%), and gets nested levels: each level's points are some of the one before's, and each level is what decimating to it alone would give (tested). Three levels of a 100 000-triangle shape take about as long as the smallest alone. The generator's settings can reuse `Keep`, and its export names each level's mesh.

### The modifier

`ModifierKind::Decimate(DecimateOptions)`, family **Clean up**, after Remesh on the palette and in Mesh → Modifiers, following decision 0142's recipe. It reads no other object, and gives the viewport and exports the same. `apply` passes the context's `hardness`; the stack's cache already redoes the step when the object's Smoothing changes. `work` is the input's triangle count (0 when nothing would go), so from 5 000 triangles the viewport works it out on another thread, showing the last result meanwhile.

### In the interface

`CommandId::Decimate` ("Decimate", its icon a square of small triangles becoming two big ones, drawn in code) on the modifiers palette, in **Mesh → Modifiers** under *Clean up* and in command search. **No keys or mouse actions.** Its Attributes rows (`attributes/decimate_rows.rs`): **Reduce by** (Ratio, Target triangles), then **Ratio** (a percentage) or **Target triangles** (a large-number field, `stepper::big_count`: a drag moves it by about half a percent of its value, the arrows by 100, Shift 1 000), **Keep UV seams**, **Keep hard edges**, **Keep material borders**, **Keep open borders**, **Quads where flat**, and **Triangles shown**: the viewport's shape's triangles after all the modifiers, when they are worked out (`Scene::shown_shape_if_ready`, which never works anything out itself), so a target can be checked against the result.

### Saving

Development file format **version 46**: version 45's model with `KindV43::Decimate { by_triangles, ratio, triangles, keep_seams, keep_hard_edges, keep_materials, keep_borders, quads_where_flat }`, its settings clamped on reading. New sample `sample-v46.avtl`; files before 46 never hold it.

### Timings

Release build, a sphere of 101 376 triangles with UVs and a seam: to 50 688 triangles 0.23 s, to 10 136 0.50 s, to 2 056 0.70 s; three nested levels (50%, 25%, 12%) 0.66 s. Setting up is about 85 ms and making the mesh about 30 ms of that.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** The palette button, **Mesh → Modifiers → Decimate** (under *Clean up*) and command search.
- **Reduce by** a **Ratio** (standard **50%**) or **Target triangles** (standard **1 000**); counted in **triangles** (what a game counts), so the setting is named *Target triangles* rather than *Target polygons*.
- **Keep UV seams**, **Keep hard edges**, **Keep material borders** and **Keep open borders**, all **on**: a kept line's points may slide along it, and stay where it turns by more than **30°** or where lines meet or end. So a target can't always be reached; *Triangles shown* says where it stopped.
- The hard edges kept are the ones the **shading shows** (marked, or sharper than the object's Smoothing angle).
- **Triangles out**; **Quads where flat** (within 1.5°) **off** as standard.
- A target at or above the shape's triangles leaves the shape **unchanged** (its own polygons, not cut into triangles).
- UVs are weighed as much as places (in a shape scaled to size 1).

## Consequences

- Projects with Decimate need this version of the app or later.
- The LOD generator (milestone 17) builds on `Decimator` / `decimate_levels`; Quadify (milestone 12) can start from Decimate's triangles.
- `mesh::subdivide::carry_face_values` is shared within the crate (Decimate carries polygon values the same way).
- Not yet: a weight for how much UVs count, symmetry kept across the mirror plane, and keeping chosen points (a point group) still.
