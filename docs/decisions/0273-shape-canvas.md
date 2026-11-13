# 0273. The shape canvas: a Shape box drawn with vectors

- **Status:** Accepted
- **Date:** 2026-11-13

## Problem

Milestone 11 of Phase 12 ([the plan](../plans/phase-12.md)), the project lead's idea: "A **Shape** box opens a canvas. Drawing: shapes with a pen (points with straight or curved sides), basic shapes, and cut-outs (one shape taken out of another). There is a mirror line for symmetrical shapes like leaves, and a reference image can sit underneath. What it makes: a sharp mask at any texture size (vectors, so never blurry), a rounded height (shapes puffed up a little), and its outline. Changing it later: you can go back and edit the shape at any time." The controls were chosen with the plan (Q/W/R/S/T, P pen, B basic shapes, Alt or the Add / Cut switch to cut out, M mirror line, double-click to edit points, Tab back, Esc only cancels). Assemblies with linked copies (12), the edge randomiser and veins (13) and foliage output (14) come after, so the data had to be ready for them without building them.

## Options

**Where the drawing lives.** A box's settings are small `Copy` values (`SettingValue`); making a drawing one of them would have touched every place settings are read. **The drawing is a field of the box** (`GraphBox::drawing`, chosen), changed only by a new checked edit, `GraphEdit::SetDrawing`, which only a box whose kind says `BoxType::takes_drawing` accepts. Undo, saving, copying and own boxes then follow the graph's own paths: a canvas change is one `Command::EditGraph`, a drag one undo step under one merge key.

**Vectors.** Drawing our own curve maths and filling was possible; **`kurbo`** (curves: inside or not, nearest place, area; MIT or Apache-2.0) and **`tiny-skia`** (filling with antialiasing at any size; BSD-3-Clause), both planned and both already in the lock file through the window library, were used in a new core crate, **`shapes`** (layer 1, no files, GPU or interface). `tiny-skia` is used without its PNG feature. `cargo deny` passes.

**Wrapping at the texture's edges.** Every material texture tiles (the `matgraph` rule). A shape crossing an edge could be clipped (a leaf card wants that) or wrapped. **It wraps** (chosen): each piece is drawn again a whole texture over wherever it reaches past an edge, so the Shape box tiles like every other box. Keeping a leaf whole is keeping it inside the square; the canvas shows the copies round the square faintly, so wrapping is seen while drawing. Foliage output (14) lays shapes on an atlas and can choose differently there.

**What a piece is.** A piece is a path (points with in and out handles, open or closed), what it does (**Add**, **Cut** out of the pieces under it, or **Line**), a name, an id never reused and a "mirrored" switch. Pieces are laid in order: each Add adds its inside, each Cut takes its inside out of everything under it, so a bite cut from a leaf, then a smaller leaf added over the bite, both work. Sides crossing over count as filled (non-zero winding), so a figure of eight fills both loops. An open path can't be filled: drawing one ends as a refusal offering **Close it**, **Keep it as a line** or **Throw it away**. Lines are kept now so veins (13) have something to become; until then they are drawn into the Outline output.

**The mirror line.** Mirroring each piece as it is drawn would make two pieces to keep in step. **The mirror line belongs to the drawing** (on or off, and where across the texture), and every piece with "mirrored" on is drawn on both sides while it is on; pen points near it snap onto it, so a half leaf drawn against it comes out whole. One upright line only (the leaf case); a level line can come later if wanted.

**What the box gives.** **Mask** (the pieces filled, sharp and smooth-edged at any size), **Height** (the mask puffed up: rising with the exact distance in from the outline, as Bevel's does, round, up to **Puff** centimetres in; **Height** sets the top) and **Outline** (a band **Outline** centimetres wide along the edge, inside and out, with the lines on it). Puff and Outline are lengths through the graph's real-world size, as every size is.

**The reference image** is kept in the drawing (made smaller to 1024 pixels at most when brought in, so projects stay small), shown under the canvas at a chosen opacity, saved with the project, and left out of what the box's results are keyed on, so changing it never works anything out again.

**Editing points later.** Double-click a piece: its points and handles show; drag them, double-click a point to switch it between corner and smooth, double-click a side to add a point there (the curve split so the shape stays), Delete removes the chosen point. A smooth point's handles stay in line when one is dragged. Tab leaves editing, and Tab again leaves the canvas.

## Decision

- **`shapes` crate** (new, layer 1; `words`, `glam`, `kurbo`, `tiny-skia`): `Path` and `Knot` (`path.rs`: points with handles, `Change` (move, turn, size), `mirrored`, `with_knot_on_side`, `contains`, `nearest`, `area`, `bounds`, limits), `Basic` (circle, square, polygon, star), `Drawing` / `Piece` / `PieceKind` / `Mirror` / `Problem` (`drawing.rs`: every change checked, `from_parts` for files, `key` without the reference), `Reference` (`reference.rs`, at most 1024 pixels each way, `fitted`), `raster::mask` and `raster::lines` (wrapping, any size up to 8192).
- **`matgraph`:** the **Shape** box (`boxes::drawn_shape`, id `drawn_shape`, Make; no inputs; outputs `mask` (shape), `height`, `outline` (shape); settings `puff` (length 0–50 cm, 3), `height` (0–1, 1), `outline` (length 0.05–20 cm, 0.5)); `GraphBox::drawing`, `GraphEdit::SetDrawing`, `BoxType::takes_drawing`, `Job::drawing` / `with_drawing`, the drawing's key in the box's cache key, `GraphBox::copy_edits` (settings and drawing, used by pasting and by opening your own box back up). 38 standard boxes.
- **`fileio`:** **file format 100**: `v95::BoxV95::drawing` (`v100::DrawingV100`: pieces with their points as six numbers each, the mirror line, the reference image as a block of bytes read only up to its largest size), checked by `Drawing::from_parts`; older files open with no drawings. `image_file::read_reference` / `open_reference` (fuzz-tested). Graph and box files carry drawings the same way.
- **`ui`:** `material::shape_canvas` (the canvas in the node canvas's place: the bar, the pieces list, the strip showing the three outputs, the basic shapes list, notes with fixes), opened by double-clicking a Shape box or **Open the canvas** in Attributes; `WorkbenchAction::ChooseReference` and `Workbench::set_shape_reference`.
- **Keys** (the plan's, decision 0028): Q, W, R, S, T, P, B, M, Alt, double-click, Tab and Esc as chosen. Two keys the plan didn't name, the most common choices and not clashing on the canvas: **Enter** finishes a pen line without closing it (then the note offers to close it or keep it as a line), and **Delete** removes the chosen pieces, or the chosen point while editing points.

## Consequences

- A leaf, a blade or a rune is drawn in the graph, sharp at any texture size, changed at any time, saved and undone like everything else.
- **Assemblies (12)** can give `Piece` a placement and a source for linked copies; ids and names are already there. **The edge randomiser (13)** can work on a piece's path before filling (`raster` takes the paths a drawing gives through `drawn_paths`), and **veins** can read the Line pieces. **Foliage output (14)** has the mask for opacity and the height for normals.
- The canvas's gizmos are drawn in 2D with the app's axis colours and work as the viewport's do (drags follow the pull); they don't share the 3D gizmo's code.
- Polygon sides and star points are set in the basic shapes list, not with Ctrl + wheel (the list isn't a tool in use).
- Speed: 40 seven-pointed stars fill at 2048 pixels in under 0.1 s (`mask_speed`, ignored, release).
