# 0099. Unwrapping, live as seams change; Relax and Straighten

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0095](0095-uvs-before-advanced-modelling.md) (seams from menus with live unwrapping, the project lead's choice), [0098](0098-seams.md) (seams and suggested seams), [0097](0097-editing-uvs-by-hand.md) (the UV ring), [0006](0006-base-plus-operation-stack.md) (edits as operations)

## Problem

Seams say where to cut; something must lay the pieces flat. The project lead chose live unwrapping: the UVs follow as seams change. Hard-surface assets also need strips (a cylinder's side, a bevel) laid out perfectly straight, pieces at their real sizes against each other, and tools to even out (relax) or straighten pieces afterwards. All of it must be edits on the stack, so it holds when earlier edits change and is undone in one step.

## Options

- **How a piece is laid flat:**
  - **least squares conformal maps** (LSCM, Lévy et al. 2002; Blender's "Conformal", chosen): each triangle keeps its angles as well as it can, solved as one sparse least-squares problem with two points pinned. Our own small solver (conjugate gradients on the normal equations, a diagonal preconditioner): no outside library, fine for game-asset sizes (a 2,000-polygon piece in about 40 ms, 8,000 in 0.4 s, 33,000 in under 4 s, release build);
  - angle-based flattening (ABF++): better on very curved pieces, much more code; can come later behind the same function;
  - xatlas (C++): a large dependency for what we need.
- **Pieces that can't lie flat** (a tube, a closed ball, because seams are missing): **cut them open first with the suggested-seam cut (0098), and mark the cut as a seam** (chosen), so seams and splits stay in step and nothing overlaps itself.
- **Strips:** a piece made only of quads with corners within 25° of square is laid out as a **straight grid** by following each quad from its neighbour (Blender's "Follow active quads", automatic), when **Straighten strips when unwrapping** is on (standard). Otherwise LSCM.
- **Where a new layout goes:** turned so its box is smallest (its sides lined up with the square), then a quarter turn at a time to match how its longest side ran before, scaled to the **object's texel density** (the UV area per real area of the polygons not being unwrapped; else of the pieces before; else about 40% of the square), and centred where the piece was. Pieces can overlap until packing arrives.
- **Live unwrapping:** marking or clearing seams (by hand, from hard edges or suggested) also records an Unwrap of the polygons on either side of the changed edges, in the same undo step, when **Unwrap as seams change** is on (standard, the project lead's choice). Off: only the marks change, until Unwrap.
- **Relax:** springs along every triangle side pull the chosen polygons' corners towards each side's real length (at the piece's density), 60 steps; other corners stay. Simple, predictable, and evens out a hand-bent grid.
- **Straighten:** the grid layout on its own, for quad pieces; others are skipped and the status bar says so.
- **What they work on:** the pieces holding the selected polygons (or the polygons around selected edges or points); the whole object when nothing inside it is selected, and in the Model and Object modes. Relax moves only the selected polygons' corners.
- **Keys:** none (menus, as chosen for the UV tools): the UV ring (E over the UV Editor, and in the UV mode) and the UV menu, where the two switches also are.

## Decision

1. `uv` crate: `chart` (wedges: a point appears once per side of a cut), `solve` (sparse conjugate gradients), `unwrap` (`unwrap`, `straighten`, `relax`, `SOLVE_UP_TO` 60,000 wedges (bigger pieces are projected flat on their main plane), `SQUARE_WITHIN` 25°, `RELAX_STEPS` 60).
2. `mesh::uvs::set_uvs` writes every corner's UV at once.
3. `scene::Operation::Unwrap { faces, strips, anchors }`, `StraightenUvs { faces, anchors }`, `RelaxUvs { faces, anchors }`; not mirrored by symmetry. `scene` now depends on `uv` (same layer).
4. **File format version 30** (`project/v30.rs`): `OperationV30::Unwrap`, `StraightenUvs`, `RelaxUvs`. Sample in `tests/data/sample-v30.avtl`.
5. UI: `workbench::unwrapping` (`UvSettings { live_unwrap, straighten_strips }`, remembered by the app under its own key; `run_uv_tool`; `live_unwrap`); the UV menu's switches; Unwrap, Relax and Straighten built in the command list.

## Consequences

- Seams now change the UVs as they're placed, as the project lead asked, and every piece keeps its real size against the others.
- An Unwrap low in an object's edit list runs again when an earlier edit changes; big pieces make that slower (seconds at tens of thousands of polygons).
- Overlapping pieces are expected until packing (the next milestones after texture size).
