# 0070. Extrude straight out as standard, with direction, end size, steps and a flat end; the Scale squares scale equally; file format version 17

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** [0038](0038-extrude-and-inset.md) (extrude and inset), [0048](0048-ctrl-wheel-changes-a-tools-count.md) (Ctrl + wheel), [0030](0030-transform-gizmos.md) (the gizmos)

## Problem

Modelling a cylinder, the project lead selected a patch of polygons on a sloped side and extruded it. Instead of going straight out of the model, it "went forwards and backwards in a really awkward way".

The cause: an extrude moved each point along its own facing (the polygons around it), stretched to keep the thickness even, as decision 0038 chose. On a curved or sloped patch those facings differ, so the points fanned out and the piece sheared. He asked for the extrude to be "buffed out" with more tools, "so we can extrude properly with total freedom".

Separately, he found the Scale gizmo's squares awkward: they scaled their two axes in the ratio the mouse moved, which "fucked me up a few times". He wants them to scale both axes equally.

## Options

Offered, and chosen by the project lead (all in the Attributes panel, before dragging and under Last edit; no new keys):

- **Direction choice** (chosen): Straight out (new standard), Along normals (the old way), Along X / Y / Z.
- **End size**, a taper (chosen).
- **Steps** along the walls, with Ctrl + wheel (chosen).
- **Move or turn the end with the gizmo** (chosen): already possible, since the end stays selected; documented.
- He added: "Extrude from normals? Follow curvature, flatten?" These became **Along normals** (follows the curvature) and a **Flat end** switch.

For the squares: **always equal in both axes** (chosen), or equal with a key for the old mix.

## Decision

1. **`ExtrudeShape`** (mesh): a direction, an end size, steps and a flat end, held by every polygon extrude (`Operation::ExtrudePolygons::shape`) and made by `extrude_shaped`.
   - **Straight out** (`ExtrudeDirection::Straight`, the standard): every point of a piece moves the same way, along the piece's facing averaged by polygon size.
   - **Along normals:** the first way, each point along its own facing.
   - **Along X, Y, Z:** the shape's own axis, out the way the piece faces. A mirrored piece then goes the mirrored way, so symmetry works.
   - **End size** (0–1000%) scales the end across the way it goes, around its middle.
   - **Steps** (1–64) build that many rows, evenly between the start and the end, so a taper is straight.
   - **Flat end** lays the end on the plane square to the way it went.
2. **Old work keeps its shape.** Extrudes recorded before this (in files up to version 16) are `ExtrudeShape::EARLIER`, Along normals with nothing else, and are worked out exactly as before. Edits are made again when a shape's settings change, so changing how they work would have changed old models.
3. **The panel:**
   - Direction, End size, Steps and Flat end sit under Keep together while Extrude is in use, and on the last extrude.
   - They are greyed out while a bend is set, because a bent extrude keeps its own way.
   - A bend taken back to 0° on the last edit makes a straight extrude shaped as the tool is.
   - With Keep together off, each polygon is shaped on its own.
4. **Ctrl + wheel** changes Steps, or the bend's Segments while a bend is set (decision 0048).
5. **The Scale gizmo's squares** scale their two axes equally: the drag is measured along the square's diagonal, so away from the centre grows and towards it shrinks, whichever way the mouse goes. A box still stretches one axis.
6. **File format 17:**
   - Extrudes shaped the first way are stored as before.
   - Any other is stored with its whole shape, on its own or mirrored (`OperationV17`).
   - The file around the edits is version 16's, now generic over its edits (`ProjectWithPlaces<E>`).
   - Real numbers are checked, and settings are brought into range.

## Consequences

- A patch on a curved or sloped part comes straight out, as in Blender and 3ds Max, and the old even-thickness look is one choice away.
- An extrude with steps or a taper is one edit, still changeable afterwards.
- The axis choices use the object's own axes (edits live in the shape's own space). For a turned object, that is its turned X, Y or Z.
