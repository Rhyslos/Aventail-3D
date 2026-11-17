# 0300. The sculpting plan (Phase 17)

- **Status:** Proposed — chosen by Claude while the project lead was away; to review
- **Date:** 2026-11-17
- **Builds on:** [0216](0216-sculpting-in-alpha-out.md) (sculpting for stylised creators), [0006](0006-base-plus-operation-stack.md) (base plus operation stack), [0033](0033-heavy-mesh-safeguard.md) and [0035](0035-heavy-meshes.md) (heavy meshes), [0028](0028-ask-about-controls.md) (asking about controls), [0142](0142-modifier-stack-and-remesh.md) (Remesh), [0187](0187-mirror-painting.md) (mirror painting)

## Problem

Phase 17 is sculpting, and the only thing decided was its aim: stylised creators (0216). On 2026-11-17 the project lead asked for its milestones to be added, but couldn't answer questions that day. A plan needs choices: how sculpted detail is kept alongside the edit layers, how a surface gets more polygons, which brushes, which keys, how it stays smooth on millions of polygons, and how a sculpt becomes a game model.

## Options

The main ones (every choice, with its alternatives and the cost of changing it, is in the plan's [Choices to review](../plans/phase-17.md#choices-to-review)):

How sculpted detail is kept:
1. **Every stroke recorded as an edit to replay.** Fits the stack literally, but thousands of strokes over millions of points would make opening a project or changing a lower layer take minutes.
2. **A Sculpt layer in the stack holding the result** (how far each point moved, measured along the surface, in chunks), each stroke one undo step keeping only the chunks it touched, as paint strokes keep tiles (chosen).
3. **Sculpting only on a plain mesh with no stack.** Simplest, but base settings stop being editable (wish W4).

How the surface gets more polygons:
1. **Dynamic topology** (polygons added under the brush). Loose and quick, but uneven triangles, no detail levels, and the hardest to make fast and crash-free.
2. **Remesh for sculpting plus detail levels** (chosen): an even surface that merges blocked-out pieces, then subdivision levels for detail, clean quads to bake from.
3. Either one alone.

Where the brushes run:
1. **On the processor, all cores, in chunks** (chosen): exact, testable, undoable.
2. On the graphics card: faster for huge brushes, much harder to keep exact and undoable.

## Decision

- **Two parts, six milestones each** (working plan: [docs/plans/phase-17.md](../plans/phase-17.md)):
  - **Phase 17a, sculpting the forms:** Sculpt mode, Sculpt layers, Draw and Smooth; the brush set for forms; pens, strokes and alphas; symmetry and masks; Remesh for sculpting; speed on 3 million polygons. Gate: block out, remesh and sculpt a stylised creature or stump, smooth on the heavy shape.
  - **Phase 17b, detail and the game-asset finish:** detail levels; stylised hard edges (Trim, Polish, Line flatten, Box and Lasso cut); sculpt groups; low-poly and modelling on the sculpt; baking the sculpt; automatic quads (the riskiest, last, can be dropped). Gate: a stylised prop sculpted, made low-poly, baked, painted and seen right in Godot.
- **A Sculpt mode and layout**, between Polygons and UV in the palette, with brushes that work like the paint brushes (brush bar, shelf, pen pressure, falloff, symmetry on L).
- **Sculpt layers** keep the result in chunks, measured along the surface, so a change below carries the detail; a changed shape below is fitted again by position with a warning badge. A new `sculpt` crate (core layer) holds the data and brushes.
- **Remesh for sculpting and detail levels**, no dynamic topology.
- **Targets on 3 million polygons:** a stroke with 200 000 points in reach without visible stutter, undo under 50 ms, remeshing to 1 million in under 5 s in the background; the heavy-mesh check gains sculpting steps.
- **Controls** (recommended, not yet asked): 1–9 pick brushes (Draw, Clay strips, Inflate, Crease, Flatten, Grab, Snake hook, Trim, Mask), hold Shift to smooth, Ctrl while stroking inverts, [ ] and Ctrl + right drag for size and strength, L symmetry, Ctrl + I invert mask, Alt + M clear mask, Ctrl + wheel grow mask, Page Up and Page Down for detail levels, Ctrl + R remesh, Shift + H / Ctrl + H / Alt + H for sculpt groups, D smoothing, E the brush ring, Esc leaves. Halberd's Q, W, R, S, T and B keep their jobs; W, R, S and T move the unmasked part.

## Consequences

- Sculpting fits undo, saving, the Edit Layers stack and scripting like every other edit, and base settings stay editable under a sculpt.
- Heavy meshes become ordinary use in Sculpt mode, so chunks run through everything (data, undo, normals, drawing).
- A sculpt can become a game model without leaving the app, through the existing decimate, quadify, shrinkwrap, unwrapping, baker and painter.
- Dynamic topology and painting colour while sculpting are left out; either can be added later as a milestone of its own.
- Every choice is to be reviewed with the project lead before milestone 17a.1 starts; changing the data model (choices 4 and 5) after that milestone would be costly, the rest cheap.
