# 0081. Bends keep their shape on stretched objects

- **Status:** Accepted, extended to extrudes, insets and bevels by [0082](0082-edits-keep-their-shape-on-stretched-objects.md)
- **Date:** 2026-10-15
- **Follows:** [0050](0050-pipe-bends.md) (pipe bends) and [0062](0062-spirals.md) (spirals)

## Problem

The project lead bent a pipe on a prop and the pipe widened as it turned, ending in an oval: the one thing a bend promises not to do.

A bend turns the pipe's end around an axis without changing its shape, but it does so in the object's own space. An object stretched in the Object mode (scaled more along one axis than another) is drawn with that stretch on top. A round pipe turned into the stretched direction is then drawn stretched across its width: wider, with an oval end. Straight extrudes hide this, because the end only moves; a bend turns it.

A second, separate fault turned up while testing: bending several polygons with **Keep together** off followed the copies of shared corners for only one of the polygons, so the rest came out bigger and out of shape.

## Options

1. **Tell people to avoid stretching objects:** stretch the points in the Points mode instead, or add an "apply scale" command. It's what Blender does. But it leaves the trap in place, and the project lead had already fallen into it.
2. **Work the bend out with the object's current stretch, every time the shape is worked out.** This is always right on screen, but it means re-making the shape whenever the object is scaled, and the shape would change with the object's scale (the rest of the shape stretches, but the pipe wouldn't).
3. **Record the stretch with the bend (chosen).** The bend remembers how the object was stretched when it was made, or when its settings were last changed. It is worked out in that stretched space and taken back into the object's own space. This gives the same result every time and is saved with the project. Changing the bend picks up the object's stretch as it is then.

## Decision

1. **Mesh:** `extrude_bent_stretched` stretches the points by the object's stretch, bends, and takes them back. The stretch is the object's scale on each axis over their average. Points the bend doesn't move stay exactly where they were. No stretch (or one that isn't a real number, or is beyond 1000 to 1) is the plain bend.
2. **The operation:** `Operation::ExtrudeBent` holds `stretch`. The extrude tool sets it from the object's shape matrix when it records a bend. The Attributes panel's bend settings and **Ctrl + wheel** refresh it whenever they change the bend. Mirrored bends keep the same stretch, and work out their mirrored direction as seen.
3. **File format 21** stores a bend on a stretched object (on its own or mirrored) with its stretch. Every other edit, and every bend on an object scaled evenly, is stored as in version 20. Older files open with no stretch, so their bends look as they did.
4. **Fix:** bending polygons each on its own follows each polygon's copies apart, as the shaped extrude already did.

## Consequences

- A pipe on a stretched object stays round through its bends, which is what you see and expect.
- A bend made earlier keeps the stretch it was made with. If the object is stretched differently later, that bend is stretched with it until its settings are changed. The manual says so.
- Other edits that work across a shape (bevel, inset) still work in the object's own space; on a strongly stretched object their widths differ by direction. If that turns out to matter, the same stretch can be given to them.
- Projects move to file format 21.
