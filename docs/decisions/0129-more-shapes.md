# 0129. Eleven more shapes

- **Status:** Accepted
- **Date:** 2026-10-22
- **Builds on:** [0095](0095-uvs-before-advanced-modelling.md) (every shape comes with UVs), [0126](0126-rework-and-first-use.md) (the development file format)

## Problem

The project lead asked for more ready-made shapes for faster modelling, "especially Icosphere". He picked: torus, capsule, dome, pipe elbow, arc/pie sector, gear, hexagon shapes (sphere and plane), stairs and arch.

## Options

For saving them: version 38's file model lists the first seven shapes by name, so a new shape can't be stored in it as itself.
1. **Store a newer shape in version 38's model as its mesh, and its settings in the development format's per-object settings (chosen).** Shapes are built the same way every time, so the stored mesh and the one rebuilt from the settings match point for point, and edits recorded on the shape find their points again.
2. A new file version for the shapes: the development format exists so new settings don't need one.

## Decision

- Eleven new kinds: Icosphere, Torus, Capsule, Dome, Pipe elbow, Pie sector, Gear, Hex grid, Hex sphere, Stairs and Arch, each with its own settings and limits, in the Create menu, the B list and command search, with its own icon.
- They're built by three families of builders in the `mesh` crate: shapes turned round the up axis (`lathe.rs`: torus, capsule, dome, elbow), outlines pushed out into solids (`prism.rs`: sector, gear, stairs, arch) and polyhedra (`polyhedra.rs`: icosphere, hex sphere, hex grid).
- Every shape stands on the ground with its pivot at the bottom centre, is watertight when closed, faces outward, and comes with UVs laid out at real size and seams where pieces part. Faces that would triangulate into slivers are avoided (stairs' sides are grids of quads, a gear's and a pie's faces are rings of quads and fans of triangles), so every shape passes Check for baking.
- Map-style (ball) UVs now give a face round the pole its own piece, and find poles relative to the ball's size.

## Consequences

- A project with a new shape needs this version of the app or later to open.
- The Create menu has 18 shapes; grouping it can come later if it grows further.
