# 0222. Importing Source engine models (QC with SMD or DMX)

- **Status:** Accepted (planned, not built)
- **Date:** 2026-10-29

## Problem

The project lead wants the app to open Valve Source engine models, as the Blender add-ons for Source Filmmaker do, built in and without licensing trouble. Those models are kept as source files: a **QC** script (the model's name, its bodies, LODs, collision model, skins and animations) pointing at **SMD** or **DMX** files that hold the meshes, bones and weights. Valve's **studiomdl** compiles them into the game's MDL files.

## Options

What to bring in (asked of the project lead):

1. **Import QC with its SMD and DMX files** (chosen): the source files modellers keep and share.
2. **Export to Source** (SMD or DMX, a QC, and compiling with the user's own studiomdl). Not chosen for now.
3. **Import compiled MDL** (with its VVD and VTX), straight from SFM or a game. Not chosen: the format is only known by reverse engineering, and changes between games.

How to read the files:

1. **Our own readers, written from the formats' published descriptions** on the Valve Developer Community wiki (SMD, VTA, QC, DMX models). Chosen.
2. **Copying a Blender add-on's code** (Blender Source Tools, SourceIO). Not chosen: Blender add-ons are written in Python against Blender's own API, so little would carry over. Blender Source Tools is GPL, and the project keeps everything it builds on permissive (decision 0004), to keep later options open.
3. **A Rust crate for DMX** (`datamodel`, which reads binary and keyvalues2 DMX). Used only if its licence is permissive (MIT or Apache-2.0) when the milestone starts; otherwise our own reader.

## Decision

Planned as its own milestone, built after the project lead's testing and the rest of the UV phase (Phase 8), in three stages that follow the app's own features:

1. **Shapes and materials** (when built): File → Import… takes a `.qc`, reads its `$body`, `$model`, `$bodygroup` (each choice an object, the others hidden), `$lod` (into the Levels panel), `$collisionmodel` (as the object's collision shape), `$cdmaterials` and `$texturegroup` (skins), `$include`, `$scale` and `$origin`; and the SMD and DMX files they name: positions, normals, UVs, polygons (DMX keeps quads and n-gons) and material names. Materials are named as in the files; their VMT and VTF textures are read when a permissive VTF reader exists, or written ourselves. Bones and weights are kept in the object as data the app doesn't show yet. Source units are inches: 1 unit is 2.54 cm. Source is Z up, like the app.
2. **Bones and weights** (with Phase 15, rigging): the kept skeleton and weights shown and editable.
3. **Animations and flexes** (with Phase 16, animation): `$sequence` SMDs as animations, VTA and DMX delta states as shape keys.

Nothing from Valve is put in the app: no studiomdl, no game files, no Valve logos. The formats are named plainly (QC, SMD, DMX, Source), as file formats are.

## Consequences

- Files from SFM and Source modders can be opened, fixed and exported again to glTF or OBJ.
- Models only kept compiled (MDL) won't open; a decompiler such as Crowbar turns them back into QC and SMD first.
- What people import is theirs to use: game models stay under their makers' terms, which the manual will say when the import is built.
- Reading a published file format to work with other programs is ordinary practice, and the plan copies nothing. This record isn't legal advice: a check with a lawyer is worth it before a commercial release.
