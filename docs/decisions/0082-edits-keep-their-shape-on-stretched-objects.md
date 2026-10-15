# 0082. Extrudes, insets and bevels keep their shape on stretched objects

- **Status:** Accepted
- **Date:** 2026-10-15
- **Extends:** [0081](0081-bends-keep-their-shape-on-stretched-objects.md) (bends on stretched objects)

## Problem

Decision 0081 made bends keep their shape on an object stretched in the Object mode. The project lead then found the same fault in bevels: on a stretched object, a bevel came out wider on some edges than on others. Insets, extrudes of edges and straight extrudes have the same fault. They measure across the shape in the object's own space, and the stretch is added on top afterwards. An inset's border ends up wider along the stretched side, a bevel's width differs by direction, and "Straight out" leans.

## Options

1. **Give each tool its own correction.** This means a different fix in every tool, each with its own tests, and more of the same for every future tool.
2. **One way to work any edit out as seen (chosen).** The points are stretched by the object's stretch, the edit runs unchanged, and the points are taken back, exactly as 0081 does for bends. Each edit that measures across the shape records the stretch it was made with.
3. **Store the stretch once per recorded edit**, beside the edit rather than in it. That would change how edits are kept and found again, and moves and turns of points must not be stretched. It is more change for no gain.

## Decision

1. **Mesh:** a `stretch` module: `as_seen(mesh, stretch, edit)` runs any edit on the points as seen and takes them back. Points the edit doesn't move stay exactly where they were. `usable` turns a stretch that isn't real, or is beyond 1000 to 1, into none. `extrude_bent_stretched` now uses it.
2. **Edits:** `ExtrudePolygons`, `ExtrudeEdges`, `InsetPolygons`, `BevelEdges` and `BevelPoints` hold a `stretch`, as `ExtrudeBent` already did:
   - `Operation::keeps_stretch`, `stretch` and `with_stretch` work on any of them, mirrored or not.
   - `EditSettings::stretch` changes it.
   - A straight extrude given a bend, and back, keeps it.
3. **Recording:** the modelling tools record the object's stretch with each of these edits. The Attributes panel's Last edit refreshes it on any change, and so does Ctrl + wheel. Moves and turns of points (already in the object's own space) and cuts (which don't measure) don't keep one.
4. **File format 22:** an edit with a stretch is stored as version 21 stores it without one, with the stretch beside it (`OperationV22::Stretched`). Edits without a stretch are stored as before. Version 21's stretched bends keep their stretch, and older files open with none.

## Consequences

- Insets, bevels, straight extrudes and bends all look the same on a stretched object as on one scaled evenly.
- Each edit keeps the stretch it was made with. Stretch the object again later and earlier edits stretch with it, until their settings are changed. This is the same rule as 0081, and the manual explains it once, on the extrude page.
- A new tool that measures across the shape needs only a `stretch` field and the `as_seen` call.
