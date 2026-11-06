# 0253. Bakes for the pipeline

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 28, the last. A bake made one object's files at a time, into pictures only; a kit has many pieces, some games colour shapes by their corners, stacked and mirrored pieces (sharing texels) spoil each other's bakes, and the maps still had to be put into a Godot material by hand.

## Decision

- **A whole kit at once:** with several objects selected, the Bake window bakes each on its own (from itself, or from the high object chosen, or matched by name, 0251), its own files each. The kit is the selection, as for kit export (0078).
- **Stacked and mirrored copies left out** (**Leave out stacked and mirrored copies**, on as standard): a piece whose UVs lie mostly (90% of the smaller box) over an earlier piece's, as stacked copies and mirrored halves do, is taken out of the low shape for the bake, so the piece they share bakes once and cleanly. Left out rather than moved aside a texture width as the plan said: bakes repeat over the square (0202), so a piece moved by a whole width would still land on the same texels.
- **Into vertex colours** (**Bake into vertex colours**): the first ticked map that comes from the shape itself (occlusion, curvature, thickness, colour ID, position) sampled at each point instead of each texel (`bake::maps::bake_corners`), recorded as vertex colours (`Operation::PaintColours`), one undo step. Normal and height need texels.
- **A Godot material** (**A Godot material (normal map and ORM)**, off as standard, with Roughness and Metal): beside the maps, `<name>_orm.png` (occlusion in red from an occlusion bake, the roughness and metal given in green and blue, `fileio::bake_file::orm_png`) and `<name>.tres`, a Godot 4 `ORMMaterial3D` using it and the normal map. Texture paths are `res://` from the Godot project holding the folder (the nearest folder up with `project.godot`), else the file names beside the material. Written as a material file rather than into the exported glTF's material as the plan said: the glTF's textures are the painter's (0208), and a material file is Godot's own way to share a material across kit pieces.
- No keys.

## Consequences

- With a high object chosen and several pieces selected, each piece is baked from that one high object: right for a kit modelled as one high sculpt.
- The phase's heavy-mesh check numbers are in this milestone's pull request.

## Amended 2026-11-06, after a test pass

- **Stacked copies by where they lie, not their boxes:** pieces are compared by the cells of a grid (256 along the UVs' longer side) their polygons cover, so a piece packed in an L-shaped piece's empty corner, or a ring's hole, is no longer taken for a copy. The rule is otherwise the same (90% of the smaller piece over an earlier one).
- **Copies left out still shade:** instead of being taken out of the low shape, they are moved to a material slot of their own that isn't baked (`BakeOrder::left_out`), so their shadow still falls on the piece baked (a stacked tower's contact face stays dark). Only when all 64 slots are in use are they taken out, as before.
- **File names of their own:** in a kit bake, a name that would make the same file name as an earlier piece's (ignoring capitals, as Windows does) gets `_2`, `_3` and so on.
- **Vertex-colour Colour ID per polygon:** each corner takes its polygon's material colour, not its point's, so colours don't bleed across a material border.
- **The Godot material turns on ambient occlusion** (`ao_enabled = true`): Godot reads an ORM texture's red channel only with it on.
- The message after a bake counts maps only (the ORM and `.tres` are said apart as "a Godot material").
