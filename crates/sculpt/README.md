# sculpt

**Layer 1 · Core**

Sculpting: a Sculpt layer's data, the brushes, and the live sculpt a stroke works on.

## Purpose

Holds what a Sculpt layer keeps and everything a stroke does, as plain data and plain functions that need no window: how far each point of a shape has moved, measured along its surface, in chunks shared between copies (so a stroke's undo step keeps only the chunks it touched); fitting a sculpt to a shape whose points changed, by position; what a dab of each brush does (Draw and Smooth, and the brushes for forms of decision 0306); a tree of boxes round the polygons that follows moving points; and the live sculpt the interface runs strokes on. See [decision 0304](../../docs/decisions/0304-sculpt-mode-and-layers.md) and the plan in [docs/plans/phase-17.md](../../docs/plans/phase-17.md).

## Must never

- Know about objects, the scene, undo, files, the GPU or the interface: the scene keeps a `Sculpt` in an edit and evaluates it; the interface turns strokes into dabs
- Change a `Sculpt` once made: a stroke makes a new one sharing every chunk it didn't touch
- Crash on bad input: a dab with a radius or place that isn't a number, an empty shape, a sculpt made for another shape (fitted, not refused), parts from a damaged file (refused by `Sculpt::from_parts` and `Moves::from_chunks`)
- Give a different result for the same input: the same shape gives the same frames, to the last bit, wherever they are worked out

## Depends on

`mesh` (the shape, its topology, `parallel`), `words` (brush and falloff names), `glam`.

## Status

- **`Sculpt`** (`layer.rs`): the structure of the shape it was sculpted on, where its points were, and `Moves`; `new`, `from_parts`, `with_moves`, `fits`, `fitted_to` (nearest point by position through `nearest::Grid`), `apply` (only chunks with moves are looked at; `Fit::found_again`), `memory_apart` and `memory_size` for the undo budget. Equal sculpts compare by pointer first. `MAX_POINTS` 30 million.
- **`moves`**: `CHUNK` 16 384 points; a chunk is nothing until a point in it moves; `differing` compares only chunks that aren't shared.
- **`frames`**: a point's facing (its polygons' area-weighted facing) and a direction along the surface (towards its first neighbour); a loose point has the object's axes.
- **`brush`**: `BrushKind` (thirteen: Draw, Smooth, Clay strips, Clay, Inflate, Crease, Pinch, Flatten, Scrape, Fill, Grab, Elastic grab, Snake hook; `grabs`, names and plain explanations), `Falloff` (Smooth, Sharp, Flat, Spike), `BrushSettings` (radius on screen, strength, falloff, spacing; `standard` per brush, `clamped`), `Dab`, `draw` and `smooth`.
- **`forms`** ([decision 0306](../../docs/decisions/0306-sculpt-brushes-for-forms.md)): what a dab of each other brush does to a point: `Plane` (the plane under the brush) and `clay` (up to a level above it, hollows first; Clay strips measured to a square turned with the stroke, `square_distance`, `along_plane`), `inflate`, `crease`, `pinch`, `flatten` (with `Side` for Scrape and Fill), `grab`, `kelvinlet` (a regularised Kelvinlet, volume kept) and `elastic`.
- **`tree::FaceTree`**: boxes round polygons in Morton order (built in a fraction of a second for millions), `refit` and `refit_points` round moved polygons (all the boxes above at once when many moved), `ray_hit` (either side), `near`.
- **`LiveSculpt`** (`live.rs`): `new` (fitted, applied, the tree built), `hit`, `dab` (points in reach once each, their new places on every core above 16 384 points, the tree refitted; the plane and facing under the brush worked out once a dab; a grabbing brush takes hold on its stroke's first dab, `held` saying where, then goes to each dab's centre: Grab from where its points were, Elastic grab step by step, Snake hook gathering afresh), `shape` (shared, new only when changed), `finish` (moved points become moves; the shape set to exactly what `apply` gives), `cancel`, `follow` (undo and redo, chunk by chunk). Smooth keeps open edges in line.
- **Tests** (`brush_tests.rs` for the brushes for forms: each on a flat grid against known results, with Ctrl, at no strength and bigger than the shape, Grab and Snake hook following, Elastic grab keeping a ball's volume within 2%; `tests.rs` and each module's own): a dab moves only points in reach, by the falloff; Ctrl digs in; no strength does nothing; a finished stroke evaluates to exactly the live shape; a stroke keeps only its chunks; the edge of an open shape; one polygon and an empty shape; Smooth flattens a bump; cancel; undo and redo through `follow`; fitting a finer shape by position; detail riding a turned shape; damaged parts. `dab_benchmark` (ignored; run with `--release --ignored --nocapture`): on a million points, a dab of about 12 000 points takes about 1.4 ms and of about 140 000 about 19 ms on two cores; finishing the stroke about 35 ms. `brush_benchmark` (ignored): ten dabs of each brush with about 140 000 points in reach on a million points.
