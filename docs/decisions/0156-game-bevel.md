# 0156. Game bevel, and Soft bevels

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0149](0149-live-bevel.md) (Live bevel; "the game bevel must soften the bevelled edges' hard marks itself"), [0148](0148-weighted-normals.md) (Weighted normals, best last; Keep hard edges follows the object's Smoothing), [0142](0142-modifier-stack-and-remesh.md) (the modifier stack and its recipe), [0141](0141-improved-bevel.md) (the improved bevel), [0040](0040-bevel.md) (a hard bevelled edge keeps hard borders), [0053](0053-hard-and-soft-edges.md) and [0126](0126-rework-and-first-use.md) (hard edges, each object's Smoothing, marking an edge on an object smoothed by angle keeps its look as marks), [0135](0135-live-booleans.md) (booleans mark their cut edges hard), [0104](0104-uvs-that-survive-modelling.md) (a bevelled seam moves to the strip's far border), [0073](0073-mikktspace-tangents.md) (MikkTSpace tangents), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0155](0155-levels-of-detail.md) (the development file format moves to version 57). Fills in DESIGN.md §6's W15 (*game bevel*).

## Problem

Phase 10's eighteenth milestone is the **game bevel** (DESIGN.md W15: "bevel, weighted normals and hard edges and seams set in one step", for the mid-poly workflow, where a model is bevelled and shaded well instead of baked). One command in the Mesh menu, on the palette and in command search adds to the selected objects a Live bevel (edges by angle 30°, 2 cm, 2 segments), Weighted normals at the end, hard edges switched off on the bevelled edges, and seams along the bevels' borders if that helps the UVs; one undo step, every setting shown and editable afterwards.

Decision 0149 left a warning: a bevelled edge marked hard keeps hard borders on its strip, so Weighted normals with Keep hard edges would shade those strips flat. Left to decide: how the bevelled edges are made soft, so the shading is right (big faces flat, bevel strips round); whether seams help; what happens to modifiers already there; what the command is; saving.

## What was measured

On a standard cube and cylinder with a standard Live bevel and Weighted normals after it, counting the edges where the final shading breaks (`hardness::shading_hard_edges`, which with custom normals lists where they part):

| Object's Smoothing, marks | Cube | Cylinder |
|---|---|---|
| By angle 30° (every new shape) | 36 breaks | 32 breaks |
| By marked edges, marks as the angle had them | 24 | 64 |
| By marked edges, the bevelled edges un-marked | **0** | **0** |

By angle, the bevel itself is the trouble: a round 2-segment bevel of a right-angled edge turns 22.5°, 45°, 22.5°, so its middle step is sharper than 30° and each strip shades with a crease along it. No angle fixes that for every edge and segment count (1 segment on a 135° edge is a 67.5° step). Shaded by marks, the bevel's own edges take their marks from the edge bevelled, so a soft edge gives a soft strip.

But un-marking edges once, as a recorded edit, isn't enough: on a cube with a hole cut by a boolean, 18 breaks were left, because a boolean marks its cut edges hard *after* the object's edits (decision 0135), where no recorded edit reaches; and an edge made later (an extrude after the game bevel) would be bevelled live but keep its mark.

## Options

How the bevelled edges are made soft:
1. **A Live bevel setting, Soft bevels (chosen):** the edges the bevel takes count as soft, worked out every time with the bevel. It softens boolean cuts and later edges alike, and lives where the bevel's settings are, so it's seen and changed with them. Together with Smoothing by marked edges (switched from by angle as marking an edge by hand does, keeping the look as marks first).
2. Un-mark the bevelled edges once (a recorded Soft edges edit) and switch to marked edges: fails for boolean cuts and later edges, as measured.
3. Weighted normals with Keep hard edges off: every edge smoothed over, including the hard ones the bevel doesn't take (edges it skips, edges left unbevelled on purpose with *Marked edges*).
4. Keep smoothing by angle and raise the object's angle: no single angle works (above).

Seams along the bevels' borders:
1. **None (chosen).** A seam doesn't change UVs; it only says where they may part. The bevel already moves a seam on a bevelled edge to its strip's far border (decision 0104), so every piece of the layout stays whole and apart; seams on the other borders would split UVs that are the same on both sides, only adding vertices to the export. With the bevels soft there are no hard edges for the bake check to ask seams for. What a strip lacks is UV *room* (it takes its edge's UVs), which only a new unwrap gives: the manual says to bake and unwrap again if the bevels are painted in detail.
2. Seams on every strip border: each strip its own piece of no area, worse for packing and painting.

Modifiers already there:
1. **Reuse them (chosen):** a Live bevel already on the shape keeps its settings and gets Soft bevels (switched on if none is on); a Weighted normals moves to the end and is switched on. The command then means "make this shape game-bevelled", and running it twice changes nothing.
2. Always add new ones: two bevels on top of each other.

## Decision

### Soft bevels (`mesh::live_bevel`)

`LiveBevelOptions::soften` (standard **off**, so Live bevels behave as before; the game bevel turns it on): before bevelling, the chosen edges with a polygon on each side are marked soft on the bevel's copy of the shape (an edge the bevel skips keeps its mark and stays crisp). Nothing happens with a width of 0. Shown in the Attributes panel as **Soft bevels**, after Stop at neighbours.

### The command (`scene::Document::game_bevel`)

Not a modifier: a command made of existing commands, run as one undo step (every command executed with one history key, `Document::execute_labelled`), so undo, the edit list and saving need nothing new. For each selected object with a shape (groups and lattice cages skipped; an instance refused, as for any modifier):

1. If it is smoothed by angle: `Operation::HardEdgesByAngle` at its angle (its look kept as marks), then `Command::SetHardness(Marked)`. A curve can't take recorded edits, so it only switches.
2. A Live bevel: added with the standard settings and Soft bevels on (`game_bevel_options`), before any Subdivide or Weighted normals (`goes_before`); or each existing one switched to Soft bevels, the first switched on if none is.
3. Weighted normals: added at the end, or the last existing one moved to the end and switched on.

A refusal part-way (too many modifiers) takes back what was done (`Document::abandon`), so nothing changes. `GameBevelDone` tells the status bar how many objects, bevels kept and smoothings switched.

### Two fixes the tests found

- **Corners fanned by a boolean.** At a box corner a boolean left as a fan of thin triangles, the improved bevel kept the corner point inside the corner patch (polygons touching no bevelled edge kept it), which folded: a normal 111° from its triangle in the export. Plain edges fanned between bevelled ones at a closed corner now slide too, out to where they leave every bevel strip in their polygon's plane (`mesh::bevel::fans`). Only for the Live bevel (`bevel_edges_weighted`): the Bevel tool's recorded bevels replay exactly as before, so saved shapes and the edits after them don't change.
- **Tangents.** MikkTSpace gives no tangent for a corner whose triangles have no UV area (a narrow bevel's corner on a cylinder with a square hole); glTF requires unit tangents and the Khronos validator failed the file. Such a corner now gets a unit tangent square to its normal (`fileio::gltf`); nothing is painted there.

### Tested

On a cube, a cylinder, an arch, a cube with a round hole and a cylinder with a square hole (booleans): flat polygons' corner normals within 0.1° of their facing (measured: 0°), no shading breaks, bevels leaning over 10°, in the viewport and the export (`scene/src/game_bevel_tests.rs`); read back from `.glb` files, at least the same share of the area shaded flat as in the scene, against none without Weighted normals, no normal more than 80° from its triangle, every tangent a unit vector (`fileio/src/export/game_bevel_tests.rs`); Bake into shape exports the same file. `files_for_outside_checkers` writes `game-bevel.glb` (the five shapes), which the Khronos glTF Validator 2.0.0-dev.3.10 passes with no errors or warnings, as all 27 sample files do.

### In the interface

`CommandId::GameBevel` ("Game bevel", its icon a block with both top corners rounded, short arrows up from its flat top and a tick, drawn in code) on the modifiers palette and in **Mesh → Modifiers** after Weighted normals under *Game ready*, and in command search. **No keys or mouse actions.** The status bar says what it did and that one Ctrl + Z takes it back.

### Saving

Development file format **version 57**: version 56's model with `soften` in `v50::LiveBevelV50` (`#[serde(default)]`: versions 50 to 56 read it as off). The bump makes an older app say it needs updating instead of dropping the switch. New sample `sample-v57.avtl` (a cube and an arch given the game bevel, the arch's bevel widened).

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** The palette button (after Weighted normals), **Mesh → Modifiers → Game bevel** (under *Game ready*) and command search.
- The standard **Live bevel settings** (edges by angle **30°**, **2 cm**, **2** segments, round, Stop at neighbours) and Weighted normals' (by area, 75%, 100%, Keep hard edges), as the notes recommend.
- **Soft bevels** as a Live bevel setting (off unless the game bevel turns it on), rather than un-marking edges once.
- Shapes smoothed by angle **switch to marked edges**, their look kept as marks (as marking an edge by hand does).
- **No seams added** (they wouldn't change the UVs).
- **Existing Live bevels and Weighted normals are reused**: their settings kept, switched on, Weighted normals moved to the end.

## Consequences

- Projects with a Live bevel using Soft bevels need this version of the app or later.
- Live bevels at boolean corners look different (cleaner) in projects made before this; Bevel tool edits don't change.
- The game bevel is ordinary modifiers and edits: everything it does is seen and changed in the Attributes panel and the edit list.
- Not yet: rounded corner patches where plain polygons fan between the bevelled edges (such a corner is closed with one polygon, not folded but not as round as a box's); giving bevel strips UV room of their own (they take their edge's UVs); the Bevel tool's bevels sliding fanned corners (it would need a recorded-edit flag).
