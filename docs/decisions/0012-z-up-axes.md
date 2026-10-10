# 0012. Z is up

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

Every 3D program picks which axis points up. Cinema 4D, Maya, Godot and glTF use **Y up**; Blender, 3ds Max, Unreal and Source (and so Halberd) use **Z up**. The project lead wants X and Y to lie flat on the ground and Z to point up. It has to be decided before the viewport exists, because the grid, the camera, the gizmos and every stored position depend on it.

## Options

1. **Y up, as Cinema 4D:** matches the app the layout is based on, and glTF and Godot need no conversion.
2. **Z up, as 3ds Max and Blender:** X and Y form the ground plane, Z is height. Matches the project lead's preference, 3ds Max, Blender and Halberd.

## Decision

**Z up, right-handed**, as in 3ds Max and Blender:

- **X** points right, **Y** points away from the viewer (into the screen in the front view), **Z** points up. The ground grid lies in the X–Y plane.
- Axis colours stay the universal ones: X red, Y green, Z blue.
- Positions are shown in centimetres by default (units become a setting later).
- **Rotation keeps Cinema 4D's H, P and B** in the Coordinates panel, defined for Z up: **heading** turns around the vertical Z axis (looking left and right), **pitch** tilts up and down, **bank** rolls sideways.
- **Export converts automatically.** glTF and Godot are Y up, so the glTF exporter turns Z-up into Y-up (the standard conversion: a −90° turn around X). Substance Painter receives the same converted files. The export presets (DESIGN.md §8) handle it, so the user never has to think about it.

## Consequences

- The viewport, grid, gizmos and primitives are built Z up from the start (Phase 1 and 2).
- Models exported to Godot arrive standing upright with no manual rotation; a round-trip test in Phase 5 checks it.
- Anyone used to Cinema 4D's Y-up will notice the difference in the Coordinates panel (height is Z, not Y); the manual says so.
