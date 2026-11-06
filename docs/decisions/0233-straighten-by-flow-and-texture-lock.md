# 0233. Straighten by flow, and texture lock

- **Status:** Accepted
- **Date:** 2026-11-05

## Problem

Phase 8 milestone 5. Straighten only worked on pieces made entirely of quads in a clean grid: a sphere's piece (triangle fans at the poles), a cylinder with triangle caps, or a pole where five quads meet were skipped. And moving points always dragged the UVs along, stretching the texture over the changed polygons.

## Decision

- **Straighten by flow** (`uv::unwrap::flow_grid`): where the piece is a full grid, the grid layout as before. Otherwise walk the rows and columns of quads out from the biggest quad, giving each corner a place on a grid of whole numbers (one step across or up per quad); corners reached two ways that disagree (rows meeting at a pole) clash, and their quads are left out. Each column gets the average width of its quads on the shape, each row the average height, so every kept quad is an exact rectangle (a ball's quads near the poles come out as wide as their column, as a straight grid must). The kept quads' corners are held while least squares conformal maps solves the rest round them (`conformal_pinned`). If nothing is kept, or the rest would fold, the piece is left alone.
- **Texture lock** (the project lead's choice, 2026-11-05: a palette toggle next to snapping, no key): `Operation::MovePoints` and `TransformPoints` gain `keep_texture`. With it, after the move the polygons round the moved points are relaxed (`uv::relax`, 40 steps), so their UVs fit their new shape while corners shared with polygons beyond, and pinned corners, are held. The flag is recorded with the move, so a replay does the same whatever the switch says later (decision 0110). The switch is `ViewOptions::texture_lock`, remembered, off as standard. Combined drags (decision 0126) take the newer move's flag.
- **File format 85**: a move with texture lock is stored as `OperationV37::MoveKeepingTexture` (the move as version 2 stored it); moves without it are stored as before.

## Consequences

- Straighten now does something useful on round shapes; the manual says so.
- Texture lock relaxes rather than re-projecting: a big move of one point can still change the texture's spacing a little round it, but it doesn't stretch.
