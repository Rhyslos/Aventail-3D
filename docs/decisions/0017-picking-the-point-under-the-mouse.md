# 0017. Finding the point under the mouse

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

Orbiting and zooming around the point under the mouse (DESIGN.md W6) needs the 3D position of whatever surface is under it. Later, selecting points, edges and faces needs the same kind of answer. The answer has to be right for everything drawn, and getting it must never make the interface stutter.

## Options

1. **Cast a ray on the CPU** against the scene's meshes. Exact, but needs a fast search structure for every mesh, and has to match what the GPU draws (later: modifiers, symmetry, layers).
2. **Read the depth image the viewport already draws.** Its depth is multisampled and thrown away after each frame, and multisampled depth can't be copied back directly.
3. **Draw the scene's surfaces once more into a one-pixel depth image** whose camera covers only the pixel under the mouse, and read that single value back.

## Decision

Option 3, in `render::pick`:

- The faces are drawn with a depth-only pipeline into a 1 × 1 image, using the viewport's camera narrowed to the pixel under the mouse. One depth value is copied back and turned into a world position with the inverse of the camera's transform. Depth 1 (nothing drawn) means background.
- **Never waits:** the question is sent while the mouse hovers over the view, and the answer is collected on a later frame (usually the next). One question is in flight at a time, and a resting mouse asks nothing new, so it costs nothing when idle. A blocking version exists for tests.
- **The interface** (`ui`) uses the latest answer if it is about the spot where the drag or wheel turn started (within a few pixels). Orbit: the surface there becomes the pivot; over background the pivot stays. Zoom: towards the surface, else the ground under the mouse (if not too far), else along the line of sight.
- The grid and lines are not surfaces, so they are never picked.

## Consequences

- Whatever the GPU draws as a surface is pickable, with no extra search structures to keep in step.
- The answer can be one frame old; moving the mouse a long way and clicking within one frame falls back as if over background. In practice the drag threshold gives it time.
- Selection (Phase 3) can extend this: drawing element IDs instead of depth into a small image answers "which point, edge or face is under the mouse?" the same way.
