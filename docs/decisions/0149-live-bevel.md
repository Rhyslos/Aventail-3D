# 0149. Live bevel, and bevel weights

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack and its recipe), [0141](0141-improved-bevel.md) (the improved bevel: rounded corner patches, Stop at neighbours), [0085](0085-bevel-control.md) (even width, welding), [0143](0143-subdivide-and-creases.md) (creases: an edge layer set by a recorded edit, a field in the Edges mode), [0148](0148-weighted-normals.md) (Weighted normals, best last; custom normals dropped by later modifiers), [0053](0053-hard-and-soft-edges.md) and [0126](0126-rework-and-first-use.md) (hard edges and each object's Smoothing), [0032](0032-edits-find-their-points-again.md) (recorded edits find their elements again), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0148](0148-weighted-normals.md) (the development file format moves to version 50)

## Problem

Phase 10's eleventh milestone is a **Live bevel** modifier: the improved bevel (0141), but worked out again whenever the shape changes, on **edges by angle** (recommended 30°), **only hard edges**, or **only marked edges**, marked with a "Bevel weight" edge layer set in the Edges mode as creases are (0143), with **Width**, **Segments**, **Profile** and **Stop at neighbours**. Its place in the stack is the person's to choose, recommended before smoothing.

The game bevel (milestone 18) will combine it with Weighted normals (0148) in that order, Live bevel first and Weighted normals last, so the two must work well together: a bevelled cube must shade with flat big faces.

Left to decide: what a bevel weight means beyond "marked", which hard edges count, the standard settings, where a new Live bevel goes in the list, the bevel's other options (even width, weld, skew), what happens when there's nothing to bevel, the controls, cost, and saving.

## Options

What a bevel weight means:
1. **A share of the width (chosen):** 0 not marked, 1 bevelled at the full width, 0.5 at half. Blender's bevel modifier does this with its *Weight* limit; it lets one Live bevel give a panel's front edges a big bevel and its back a small one. The improved bevel already works out each edge's width on its own (for even width and skew), so a per-edge factor is one multiplication (`bevel::bevel_edges_weighted`).
2. A plain mark (any weight above 0 bevels at full width): simpler, but then the field's numbers between 0 and 1 mean nothing.

Which "hard edges":
1. **Those the object's shading shows hard (chosen):** its Smoothing (0126) is either by angle or by its marked hard edges, the same rule Decimate and Weighted normals follow (`ModifierContext::hardness`). What looks hard is what's bevelled.
2. Only edges marked hard: on objects smoothed by angle (every new shape) the marks don't show, so the modifier would bevel invisible marks (a primitive marks its box edges hard).

Where a new Live bevel goes:
1. **Before the first Subdivide or Weighted normals in the list, else at the end (chosen):** the recommended order (bevel, smooth, then weighted normals last) without the person moving it, and the game bevel can add a Live bevel to a shape that already has Weighted normals. `ModifierKind::goes_before` is the rule, so a later kind can say the same.
2. At the end like every other kind: a Live bevel added after Weighted normals would make the amber "move Weighted normals to the end" note appear at once.

The bevel's other options:
1. **Fixed: improved, even width and welding on, no skew (chosen),** as a new Bevel tool bevel; the weld distance is 1 mm, or a quarter of a row for a very fine bevel so its rows aren't welded together. The milestone asked for Width, Segments, Profile and Stop at neighbours; skew needs a chosen side per chain of edges, which a rule-chosen set doesn't have.
2. Every Bevel tool option: more rows for little gain.

## Decision

### The work (`mesh::live_bevel`, layer 0)

- `LiveBevelOptions` (`Copy`, `Default`, `clamped`): **Edges** (`BevelEdges::ByAngle`, standard; `Hard`; `Marked`), **Angle** (0 to 180°, standard **30°**), **Width** (0 to 100 m, standard **2 cm**), **Segments** (1 to 64, standard **2**), **Profile** (−1 to 1, standard **1**, round) and **Stop at neighbours** (`clamp`, standard **on**). Values that aren't numbers become the standard ones. `shape()` gives the `BevelShape`.
- `chosen_edges(mesh, options, hardness)`: by angle through `hardness::sharp_edges` (edges whose polygons meet more sharply than the angle; flat and border edges never); hard through `hardness::shading_hard_edges` (custom normals' breaks left out); marked through `bevel_weight::bevel_weights`, each with its weight.
- `live_bevel(mesh, options, hardness)`: the chosen edges bevelled with `bevel::bevel_edges_weighted`; nothing chosen is `LiveBevelProblem` (*no edge is sharper than the angle…*, *the shape has no hard edges…*, *no edge has a bevel weight…*), so the shape goes on unbevelled and the Attributes panel says why. A width of 0 gives the shape as it is. Edges the bevel can't take (open borders, edges of three or more polygons, points without a single fan) are skipped by the bevel, as with the tool. Deterministic; ids as the bevel makes them.
- **Bevel weights** (`mesh::bevel_weight`): the `bevel_weight` edge layer (0 to 1, only where not 0), carried by the builder like every edge layer. Creases and bevel weights now share their code (`mesh::edge_weights`).

### In the scene

`ModifierKind::LiveBevel(LiveBevelOptions)`, family **Generate**, second on the palette and in Mesh → Modifiers (after Subdivide), following decision 0142's recipe. It reads no other object and gives the viewport and exports the same. `work` is 4 per bevelled edge (`live_bevel::edge_estimate`) plus the polygons ÷ 20: one bevelled edge costs about four polygons of a boolean (20 000 edges of a 10 000-polygon sphere in 0.32 s, 240 edges of a 2 400-polygon box in 4.5 ms, release build), so from about 1 200 bevelled edges the viewport works it out on another thread. Like every kind but Weighted normals, its result carries no custom normals (0148), so a Weighted normals after it weights the bevelled shape.

`Command::AddModifier` puts a new modifier before the first one it `goes_before` (Live bevel: Subdivide and Weighted normals), else at the end.

`Operation::SetBevelWeights { edges, weight, anchors }` sets weights (0 removes them; a weight outside 0 to 1 or not a number is refused), mirrored with symmetry, finding its edges again after earlier edits change (0032). Undo labels "Bevel weight 3 edges", "Clear bevel weight 3 edges".

### With Weighted normals

Tested in the scene, on a cube with its edges softened (as the game bevel will) and Weighted normals added first: the Live bevel goes before it, and with the object shaded **by angle (30°)** and **by its marks**, the big faces' corner normals are within 0.05° of their facing in the viewport and the export, while the bevels' lean more than 10° (they shade round). Read back from a `.glb`, the big faces' normals are within 0.05°; with Weighted normals switched off they lean by more than a degree. `files_for_outside_checkers` writes `live-bevel.glb` (a cube and a cylinder with both), for the glTF Validator.

### In the interface

- `CommandId::LiveBevel` ("Live bevel", its icon a block with its top left edge rounded and a circling arrow, drawn in code) on the modifiers palette, in **Mesh → Modifiers** under *Generate* and in command search. **No keys or mouse actions.**
- Its Attributes rows (`attributes/live_bevel_rows.rs`): **Edges** (By angle, Hard edges, Marked edges), **Angle** (only by angle), **Width** (in the scene's units), **Segments**, **Profile** (a percentage, as the Bevel tool shows it) and **Stop at neighbours**.
- **The Bevel weight field:** the Attributes panel's **Selected edges** section (Edges mode, edges selected) now has **Crease** and **Bevel weight**, each 0 to 1, showing the first edge's value with a note when they differ. A drag records one edit at its start and changes it as it goes, one undo step, exactly as the Crease field (the two share `ui::edge_values::EdgeValue`).
- **Drawn** lime green (`OverlayStyle::BevelWeight`) in the Points, Edges and Polygons modes, as thick as hard edges, as creases are drawn magenta.

### Saving

Development file format **version 50**: version 49's model with `KindV43::LiveBevel(LiveBevelV50)` (edges as a number, anything unknown by angle; angle, width, segments, profile, stop at neighbours; clamped on reading) and `OperationV37::SetBevelWeights { edges, weight, anchors }` (a weight outside 0 to 1, or an edge from a point to itself, refused as damaged). Bevel weights on a plain mesh are its `bevel_weight` layer, saved as every layer is. New sample `sample-v50.avtl`.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** The palette button, **Mesh → Modifiers → Live bevel** (under *Generate*), command search, and the **Bevel weight** field in the Attributes panel. No menu commands for marking (unlike creases' Crease edges and Uncrease): the field sets any weight, and the brief asked for a field.
- Standard settings: **Edges by angle**, **Angle 30°**, **Width 2 cm**, **Segments 2**, **Profile 100%** (round), **Stop at neighbours on**; the same width and segments the game bevel will use.
- A **bevel weight is a share of the width** (1 full, 0.5 half), not just a mark.
- **Hard edges** follow the object's **Smoothing** (by angle or by its marks), as Decimate's and Weighted normals' do.
- A new Live bevel goes **before any Subdivide or Weighted normals** already in the list; Up and Down still move it anywhere.
- Even width and welding are always on, skew off; Profile shown as a percentage like the Bevel tool.
- Weighted edges drawn **lime green** whenever points, edges or polygons are edited, with no switch.
- Nothing to bevel shows the shape unbevelled with an amber reason, rather than silently.

## Consequences

- Projects with Live bevel or bevel weights need this version of the app or later.
- Bevel weights are edits like creases: they survive later edits and edit layers, and show in the edit list.
- `ModifierKind::goes_before` lets later kinds (the LOD generator, triangulate preview) choose their place when added.
- The game bevel (milestone 18) can add a Live bevel and Weighted normals in one command; it still has to soften the bevelled edges' hard marks itself, since a hard bevelled edge keeps hard borders (0040) and Weighted normals keeps hard edges.
- Not yet: per-edge widths from a vertex group or a limit by edge length, the Bevel tool's skew, and a clamp on overlap with non-bevelled geometry beyond Stop at neighbours.
