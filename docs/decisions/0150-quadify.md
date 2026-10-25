# 0150. Quadify

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack and its recipe), [0145](0145-decimate.md) (Decimate and its *Quads where flat*, whose pairing this shares), [0149](0149-live-bevel.md) (the latest modifier; `goes_before`), [0052](0052-clean-up.md) (clean-up: Dissolve merges polygons keeping the lowest id), [0053](0053-hard-and-soft-edges.md) and [0126](0126-rework-and-first-use.md) (hard edges, each object's own Smoothing, the grouped Mesh menu), [0032](0032-edits-find-their-points-again.md) and [0059](0059-symmetry-for-every-edit.md) (recorded edits find their polygons again, and work with symmetry), [0110](0110-controllable-automation.md) (automatic tools show their settings and give the same result every time), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0149](0149-live-bevel.md) (the development file format moves to version 51)

## Problem

Phase 10's twelfth milestone is **Quadify**: joining pairs of triangles back into four-sided polygons (quads). Shapes often arrive as triangles: from another program's export, a game's files, a scan, Decimate, or a boolean's cut. Triangles are hard to keep editing: edge loops and loop cuts stop at them, and smoothing a triangle makes a lump. Most of those triangles come in pairs that were once a quad.

The notes ask for a modifier and a one-off command in the Mesh menu's clean-up group, settings **Max angle** between the triangles' normals (recommended 40°) and **Max shape distortion** (recommended 40°), switches to respect UV seams, hard edges and materials (on), and the best pairs first, as Blender's *Tris to Quads* does, deterministically. The one-off acts on the selected polygons in the Polygons mode, or the whole shape in the Model mode, as a recorded edit so undo works.

Left to decide: what the two limits measure, the order of pairing, which edges never go, what the one-off records (and where its settings live), where it sits in the Mesh menu and the modifier list, ids, and how it relates to Decimate's *Quads where flat*.

## Options

What *Max shape distortion* measures:
1. **How far the new quad's least square corner is from a right angle (chosen).** 0° takes only rectangles; 40° takes corners from 50° to 130°. Easy to picture and explain; a split square scores 0°, a pair of equilateral triangles (a 60°/120° diamond) 30°, a square's triangles paired the wrong way across a grid (a 45°/135° slant) 45°, so the standard 40° joins the first two and refuses the third.
2. Blender's own error sum (normals of both diagonals, opposite edges' parallelism, the two halves' areas): close in practice, but a mix of three things that is hard to explain to someone picking a number.

The order:
1. **Greedy, best first (chosen):** every candidate pair scored, sorted, and taken unless one of its triangles is already taken. The score is the quad's corners' total distance from right angles plus how far it is bent (radians), so square, flat quads go first. Ties go by the polygons' ids, so the result never depends on hashing or threads. This is what Blender's *Tris to Quads* did for years, and what Decimate's *Quads where flat* already did.
2. A global best matching (a graph matching that maximises the number of quads): a few more quads on awkward meshes, much more code, and its choices are harder to predict.

Which edges never go, beyond the switches:
1. **Creased and bevel-weighted edges always stay (chosen).** They mean something to Subdivide (0143) and Live bevel (0149); joining across them would silently drop the mark.
2. Only what the switches say: simpler, but would lose those marks without a word.

Which hard edges:
1. **Those the object's shading shows hard (chosen),** as Decimate, Weighted normals and Live bevel do: marked edges on an object shaded by its marks, edges sharper than its Smoothing angle on one smoothed by angle. Custom normals' breaks don't count (they are a result, not a choice).
2. Marked edges only: on the standard smoothing (by angle) the marks don't show.

The one-off's settings:
1. **Recorded in the edit, changeable under Last edit, and remembered for the next one (chosen),** as Hard edges by angle keeps its angle. The edit is recorded even when the settings join nothing, as long as looser ones would, so there is always a Last edit to loosen; nothing at all to join (no neighbouring triangles) records nothing.
2. A dialog before running: one more window, and no way to see the result while changing it.

The one-off's hard edges: a recorded edit is worked out without its object, so the object's Smoothing **is stored in the edit** (as it was when recorded): changing Smoothing later doesn't change which edges an earlier Quadify now kept.

## Decision

### The work (`mesh::quadify`, layer 0)

- `QuadifyOptions` (`Copy`, `Default`, `clamped`): **Max angle** (0 to 180°, standard **40°**), **Max shape distortion** (0 to 90°, standard **40°**), **Keep UV seams**, **Keep hard edges**, **Keep material borders** (all standard **on**). Values that aren't numbers become the standard ones.
- `pairs(mesh, faces, options, hardness)`: a candidate is an edge shared by exactly two triangles running opposite ways round (so they face the same way), with different far corners, both among `faces` when given, and not kept: a marked seam or an edge where the UVs split (Keep UV seams), a hard edge as `hardness` shows it (Keep hard edges), two different material slots (Keep material borders), or any crease or bevel weight. The two triangles must lean by at most Max angle, and the quad they make (a → d → b → c) must have no dent and no corner further than Max shape distortion from a right angle. Then `pairing::best_pairs`, as above.
- `quadify(mesh, faces, options, hardness) -> Outcome` joins the chosen pairs with Dissolve's merge (`cleanup::merge`): the quad keeps the **lower id** of the two, its corners keep their values (UVs and the rest, from the triangle that has them), the edge between them and its values go. Points never move or go; nothing else changes. `quadified` is the modifier's, refusing a shape with no triangles (`QuadifyProblem::NoTriangles`) or nothing that may join (`NothingJoins`), each with a reason saying what to change.
- Never panics: degenerate triangles (no area, a zero-length edge), points not at real places, open borders, edges of three or more polygons, neighbours facing opposite ways and n-gons are left as they are. Deterministic.
- **Shared with Decimate:** `pairing::quad_shape` (the bend, squareness and worst corner of a quad made of two triangles, `None` for a dent or no area) and `pairing::best_pairs` (best first, ties by number, each triangle once). Decimate's *Quads where flat* now uses both, with its own 1.5° flatness and its own idea of which edges are kept; its results are unchanged (its tests pass as they were).
- Timing (release build): a sphere of 50 688 triangles in 0.12 s.

### The modifier

`ModifierKind::Quadify(QuadifyOptions)`, family **Clean up**, after Decimate on the palette and in Mesh → Modifiers, following decision 0142's recipe. It reads no other object and gives the viewport and exports the same. Its hard edges are `ModifierContext::hardness`'s. `work` is half the input's triangles (about two triangles cost one polygon of a boolean), so from about 10 000 triangles the viewport works it out on another thread. A new Quadify **goes before** any Live bevel, Subdivide or Weighted normals in the list (`goes_before`): bevelling and smoothing work best on quads. Up and Down still move it.

### The one-off: Quadify now

`Operation::Quadify { faces, options, hardness, anchors }`: the polygons (none for the whole shape), the settings, the object's Smoothing when recorded, and where the polygons were (0032). Its undo label is "Quadify" (whole shapes) or "Quadify 4 polygons". With symmetry on, chosen polygons are joined together with their mirrors (0059); a whole-shape Quadify already covers both sides. Its settings are `EditSettings::quadify`, shown under Last edit (and in an edit opened from the edit list) by the same rows as the modifier's. It doesn't mend UVs: every corner keeps its own.

In the interface, `CommandId::QuadifyNow` ("**Quadify now**") in a new **Mesh → Clean up** group (Weld by distance, milestone 13, joins it) and command search (under Mesh): in the **Model mode** it quadifies each selected shape whole (instances and curves left out), in the **Polygons mode** the selected polygons; in other modes it is greyed out with the reason. One edit per object, one undo step; the status bar says how many pairs were joined. It is named apart from the modifier because command search lists both and one label means one thing.

### Saving

Development file format **version 51**: version 50's model with `KindV43::Quadify(QuadifyV51)` (the five settings, clamped on reading) and `OperationV37::Quadify(QuadifyEditV51)` (polygons, settings, the smoothing angle or none for "by its marks", anchors; settings that aren't numbers, or a smoothing angle outside 0 to 180°, refused as damaged). New sample `sample-v51.avtl`; files before 51 never hold either.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** The modifier: the palette button, **Mesh → Modifiers → Quadify** (under *Clean up*) and command search. The one-off: **Mesh → Clean up → Quadify now** and command search; not on the E ring (its places are full and stay put).
- The one-off is named **Quadify now**, so it isn't confused with the modifier in command search.
- Standard settings **Max angle 40°**, **Max shape distortion 40°**, **Keep UV seams**, **Keep hard edges** and **Keep material borders** on, as recommended. *Max shape distortion* is how far the **least square corner** of the new quad is from a right angle.
- **Creased and bevel-weighted edges always stay**, whatever the switches.
- **Hard edges** follow the object's **Smoothing** (by angle or by its marks); the one-off keeps the Smoothing it was made with.
- A new Quadify goes **before** any Live bevel, Subdivide or Weighted normals in the list.
- Quadify now in the **Points or Edges mode** is greyed out with a reason, rather than quadifying whole shapes.
- Quadify now's settings are changed **under Last edit** after running it, and remembered for the next one; it is recorded even when its settings join nothing, so they can be loosened there.
- The modifier with nothing to join shows the shape unchanged with an amber reason, as Live bevel does.

## Consequences

- Projects with a Quadify modifier or a Quadify now edit need this version of the app or later.
- Weld by distance (milestone 13) joins the Mesh menu's Clean up group the same way: a modifier and a one-off.
- Decimate and Quadify pair triangles the same way; a better pairing for one improves both.
- Not yet: joining triangles into quads across more than one step (a pass that rotates diagonals first, as some tools do), and a Quadify that makes n-gons from flat fans.
