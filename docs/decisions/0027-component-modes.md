# 0027. Points, Edges and Polygons modes: what is edited, and finding what is under the mouse

- **Status:** Accepted
- **Date:** 2026-10-11

## Problem

Phase 3 needs the Points, Edges and Polygons modes before anything can be selected or moved. Each mode must show the parts (elements) of the shapes being worked on and tell which one is under the mouse, quickly enough to follow the mouse in a heavy scene. Four questions needed answers: which objects are being edited, which keys switch modes, how the element under the mouse is found, and how the elements are drawn.

## Options

**Which objects are edited**
1. Every visible editable mesh. A busy scene would be covered in dots.
2. **The selected objects.** This is the usual rule in 3D programs, and it works for several objects at once.

**Keys**
1. Halberd's mode keys. Rejected by [decision 0019](0019-only-the-tool-keys-come-from-halberd.md): only the tool keys come from Halberd, and `Q` stays the Select tool, as the project lead confirmed.
2. **`1`, `2`, `3`** for Points, Edges and Polygons, as in several common 3D programs, and free here. Pressing the active mode's key again goes back to Model, so one key both enters and leaves a mode.

**Finding the element under the mouse**
1. Draw every element with a unique colour into an off-screen image on the GPU, and read the pixel under the mouse. It's exact, but it costs a GPU round trip each time the mouse moves, and it's awkward to test.
2. **Look it up on the CPU** (`tools::PickScene`):
   - The scene's faces are cut into triangles in world space once per scene change.
   - Each mouse move then projects the edited objects' points and edges to the screen.
   - The closest within a small radius wins (10 points for points, 8 for edges). For polygons, the first face the mouse ray hits wins.
   - A ray test against the triangles rejects anything hidden behind a surface.

**Drawing**
1. Rebuild the scene's line geometry with the elements in it.
2. **A separate overlay layer** in the renderer:
   - Points and thick lines are drawn as screen-sized quads (`sprite_vs`), so their size doesn't change with zoom.
   - Hovered faces get see-through tinted triangles.
   - Everything is depth-tested against the faces, so hidden elements stay hidden. Nothing is written to depth.
   - A mouse move only rebuilds this small layer.

## Decision

Option 2 in each case:
- The selected objects are the ones edited.
- Keys `1`, `2`, `3` switch modes, and the same key again goes back to Model. The keys are shown in the Modes menu and the palette tooltips.
- The element under the mouse is found on the CPU, counting visible elements only.
- Elements are drawn on a separate overlay layer in the accent colour.

Details:
- Hovered elements use the accent colour, like everything else that is active.
- The status bar names the hovered element ("point 4 of Cube").
- The shape's red selection outline is hidden while editing, because its own edges are drawn instead.
- Primitives can be looked at in these modes, since their mesh is already built. Edits will record on top of the base ([0021](0021-object-stack-foundation.md)) when the move tools arrive.

## Consequences

- The CPU lookup tests every triangle once per candidate element. That is fast for game assets (thousands of faces), but a scene of millions of faces would need a spatial index (a BVH). That can be added inside `PickScene` without changing its users.
- Selecting by click, rectangle and lasso (the next milestone) reuses `PickScene`. Rectangle and lasso will also need the "visible only" test, which is already there.
- Selection of elements isn't stored yet. The next milestone adds it to the scene, with undo.
