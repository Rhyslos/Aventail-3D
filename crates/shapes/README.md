# shapes

**Layer 1 · Core**

Vector shapes for the shape canvas: paths with straight and curved sides, cut-outs and a mirror line, turned into sharp masks at any size.

## Purpose

The drawings behind the Material mode's **Shape** box (Phase 12 milestone 11, [decision 0273](../../docs/decisions/0273-shape-canvas.md), the plan in [docs/plans/phase-12.md](../../docs/plans/phase-12.md)): what a drawing is, every change to it (checked, saying why when it can't be made), and filling it into pictures at any texture size, wrapping round the texture's edges so they tile. Everything here can be tested without a window or a GPU.

## Must never

- Know about graphs, scenes, undo, files, the GPU or UI (`matgraph` runs drawings in the Shape box; `fileio` saves them; the interface draws and edits them)
- Change a drawing except through its checked `with_…` changes and `from_parts`, or refuse one without a plain-words `Problem`
- Make a picture that doesn't tile, or allocate one over `raster::MAX_SIZE` across
- Keep a reference image over `reference::MAX_SIDE` pixels each way, or let it change what a drawing gives (`Drawing::key` leaves it out)
- Panic on a damaged or odd path (no points, one point, sides of no length, sides crossing over)

## Depends on

`words` (the language layer: every name and reason goes through it), `glam`. Outside libraries: `kurbo` (curves: area, inside or not, nearest place; MIT or Apache-2.0) and `tiny-skia` (filling and stroking with antialiasing; BSD-3-Clause; without its PNG feature).

## Status

Phase 12, milestone 11 (the shape canvas):

- **Paths** (`path`): `Knot` (a point with incoming and outgoing handles, as offsets; `corner`, `smooth`), `Path` (points in order, open or closed; `sides`, `bounds`, `middle`, `area`, `contains` (non-zero winding, so sides crossing over are inside), `nearest` (side, how far along, how far away), `point_on_side`, `with_knot`, `without_knot`, `with_knot_on_side` (de Casteljau: the shape kept), `changed`, `mirrored` (in an upright line, running the same way round), `problem`: at most `MAX_KNOTS`, every point and handle a real number within `MAX_PLACE` of the texture). `Change` (move, turn, size about a point; `flips`).
- **Basic shapes** (`basic`): `Basic` (circle of four curves, square, polygon and star with `SIDES` sides or points) as a closed path round a middle.
- **Drawings** (`drawing`): `Drawing` (pieces in order, ids never reused, the `Mirror` line, the `Reference`), `Piece` (id, name, path, `PieceKind` Add, Cut or Line, mirrored), changed only by `with_piece`, `with_path`, `with_kind` (a Line opens its path), `with_name`, `with_mirrored`, `with_change`, `without`, `with_order`, `with_mirror`, `with_reference`, each checked: an open path can't add or cut (`Problem::OpenPath`), a closed one needs room inside (points not all on one line), a line two points, names up to `MAX_NAME`, at most `MAX_PIECES` pieces and `MAX_ALL_KNOTS` points. `drawn_paths` (a piece and its mirror image), `key` (what it gives, not the reference), `memory_size`, `to_parts` / `from_parts` (`DrawingParts`, everything checked). `Problem::describe` says why in plain words.
- **Reference images** (`reference`): `Reference` (RGBA pixels at most `MAX_SIDE` each way, a name, an opacity; `fitted` averages a bigger picture down; `with_opacity`).
- **Pictures** (`raster`): `mask(drawing, size)` (each piece filled with antialiasing, Adds added and Cuts taken out in order, each copy a whole texture over wherever it crosses an edge) and `lines(drawing, size, width)` (the Line pieces with round ends), as coverage from 0 to 1, up to `MAX_SIZE`.
- Tests in each module, `drawing/tests.rs` (every refusal, changes, order, the mirror line, parts round trip and damage, the limits, a fuzz run of random changes) and `raster/tests.rs` (shares at many sizes, sharp edges smoothed along curves, cut-outs and their order, the mirror line, wrapping and seamless joins, lines, odd paths: sides of no length, figures of eight, one point); `mask_speed` (ignored) times 40 stars at 2048.

Not built yet (later milestones): pieces placed and linked as copies (assemblies, 12); the edge randomiser and veins from lines (13); opacity and atlas output (14).
