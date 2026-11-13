# 0296. Blending seams where two materials meet

- **Status:** Accepted
- **Date:** 2026-11-13
- **Follows:** [0293](0293-blend-seams.md) (Blend seams and the Seam blend layer), [0294](0294-seam-blend-brush.md) (the seam blend brush), [0295](0295-picking-seams-in-the-uv-editor.md) (picking seams in the UV Editor)

## Problem

Each material is painted on its own texture. Where two materials meet on a model (a metal band round a wooden handle, a cloth sleeve on a skin arm) the two paintings join at the edge, and often show a hard line there, even when the two sides' UVs match. Blend seams (milestone 1) left such edges out: they aren't seams of one texture. The project lead asked on 2026-11-13 for them to be blended too. This is milestone 4 of the [seam blending plan](../plans/seam-blending.md).

## Options

- **Which edges:** an edge between two materials is a join **whatever its UVs**: two textures meet there, so matching UVs still meet two different paintings. `paint::seams::seams_between(surface, mine, theirs)` finds them (one triangle in each material), this material's side first. `seams_of` is unchanged.
- **Who writes what:** (a) one layer writes both sides: it would have to change two texture sets from one layer. (b) **Each material's Seam blend layer writes only its own side**, blended towards the other material's painting (chosen): both meet half way on the edge, as one material's seams do, and each layer stays one material's.
- **What each reads of the other:** reading the other's Seam blend layer would make A's layer change B's input and B's change A's, for ever. **Each reads the other material's layers under its Seam blend layers** (`TextureSet::below_seam_blends`: under its lowest one, or all of them when it has none), never a Seam blend layer. Its own side is still read from the layers under its own layer.
- **Marks:** (a) store the seam once with the other material's name: a new field and file format, and the other material's layer couldn't see it. (b) **Mark it in both materials' layers, as the same seam id, in one undo step** (chosen). A seam id is an object and two point ids; whether it is a seam of one material or where two meet is found from the model when blending, so nothing new is stored: **no new file format** (111 stays). If the model's materials change so an edge becomes a seam of one material, the same mark keeps working.
- **Clicks:** the gestures already chosen. A plain click marks the line in place of the marks of the **material clicked** and adds it to the other's (its other marks aren't visible, so replacing them would surprise); Shift + click adds to both, or unmarks from both when the clicked material's layer marks it all already. **Unmark all** takes the material's seams with others off theirs too, so neither side is left half blended. **Every seam that shows** compares the two paintings (`seams::showing_between`, `advice::painted_differently` taking two images) and marks those on both. If the other material isn't painted yet, the click says to paint it first and marks nothing.
- **Saying which two meet:** pointing at such an edge (in the 3D view or the UV Editor) names the two materials in the status bar, once as the mouse comes to it, and the click's note names them too.
- **Sizes:** each side is laid out in its own texture's texels (`seam_blend::Side`), so two textures of different sizes blend at their own scale.
- **Remaking:** a layer's hash takes in, for each other material it meets on its marked seams, that set's size, realism switch, layout key, the settings of the layers it reads and their pixels on just the tiles read (`seam_blend::other_tiles`). Painting one side remakes the other's layer when the stroke ends; nothing is remade per frame or while a button is held.
- **The seam blend brush:** it mixes one layer's own paint with itself near seams. Across two materials there are two layer stacks, and a stroke paints one of them, so it would move only one side, towards a painting it can't change. It is left to the tool: the brush still blends only one material's seams.

## Decision

- `paint::seams`: `seams_between`, `showing_between`; `advice::painted_differently` takes each side's image.
- `paint::seam_blend`: `Across` (another material's slots on a shape and the seams between, the other painting by number), `BlendShape::across`, `Found` from `resolve`, `other_tiles`, `blend(below, others, shapes, chosen, width)`; `blend_side` takes two `Side`s (painting, shape, layout, side), shared with the brush. `TextureSet::below_seam_blends`.
- `ui::workbench`: `SeamShape` (a struct now, with `across: Vec<CrossSeams>`), `cross_seams_of` (kept per object and pair of materials while the shape and slots stay the same), the hash and the making in `seam_blending.rs`; `mark_across`, `seam_lists`, `unmark_across`, `put_seam_sets` (several sets as one undo step), the cross seams shown and picked in both views in `seam_picking.rs`.

## Consequences

Edges between two materials blend like any seam, from either view, in one undo step for both materials, and stay blended as either side is repainted. Nothing new is saved. A Seam blend layer deleted from one material leaves the other's side blended half way; marking the seam again makes it anew. Blending past the 0 to 1 square for trim repeats, and the brush across materials, are still left out.
