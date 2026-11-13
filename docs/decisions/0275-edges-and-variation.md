# 0275. The edge randomiser, varied copies and veins

- **Status:** Accepted
- **Date:** 2026-11-13

## Problem

Milestone 13 of Phase 12 ([the plan](../plans/phase-12.md)), the project lead's idea for foliage artists ("razor leaves with different shapes dynamically"): an **edge setting per shape** (jagged, serrated, wavy, torn) with amount, size and a seed; **Vary copies**, giving every copy its own seed, size, angle and colour shift so no two leaves match, even linked ones; and **veins**, lines on the canvas becoming veins in the height and colour. Edges must stay sharp at any size, come out the same for one seed, tile, and follow the mirror line. Milestone 12 ([decision 0274](0274-assemblies.md)) left linked copies sharing one shape, each placed on its own.

## Options

**Where the edge is made.** (a) Roughen the filled picture (a noise warp of the mask); (b) **roughen the path before it is filled** (chosen). (b) keeps the result a vector shape, so it is as sharp at 4096 pixels as at 64 and tiles exactly (it wraps like any other path), the canvas's outline shows the teeth live, and clicking hits the toothed shape. (a) would blur at large sizes and leave the outline smooth.

**How the outline is roughened.** The outline (curves followed to a hundred-thousandth of the texture) is walked in even steps, and each step pushed out or in along the outline's outward side by a pattern that **repeats a whole number of times round it**, so the start and end meet with no seam. Teeth fit the outline: their number is its length over the tooth size, rounded. Five kinds: **Smooth**, **Jagged** (sharp teeth of uneven height: corners), **Serrated** (saw teeth rising slowly to a tip three quarters along, then dropping), **Wavy** (waves of slightly uneven height, smooth points with handles so they stay round at any size) and **Torn** (four scales of smooth noise, the coarsest a tooth wide). Amount 1 makes teeth as deep as they are wide, but never deeper than a quarter of the shape's narrower side, so a thin blade can't turn inside out. At most 2048 points per outline (wider teeth past that). Amount 0 or Smooth returns the path untouched. The edge is computed, never stored: reshaping the leaf keeps its teeth.

**Where the settings live.** The edge **belongs with the shape**: linked copies share it (changed through the link, refused in a file when linked copies differ). So does **Vary copies** (one tick varies a whole twig) and a line's **vein**. Each piece has its **own seed**, from its id when made or pasted (so copies differ at once) and rolled by **↻ New look**; the edge has a seed too (↻ beside it).

**How a copy varies.** With "each copy differs" on, a piece is sized (Size ±, up to 50%) and turned (Angle ±, up to 180°) about its middle after its place, its colour shifted (lighter, darker or tinted, as Scatter's copies are) and its teeth seeded by the edge's seed mixed with its own. **A group varies as one**, by its bottom piece's settings and seed about the group's middle, so a leaf's veins stay on it.

**The mirror line.** Without varying, a mirrored piece's mirror image is the exact mirror of the toothed shape (serrations mirror). With varying, the mirror image is varied as a copy of its own (its own size, turn and teeth). **A piece touching the mirror line** (half a leaf drawn against it) is treated as half of one leaf: its teeth fade out within a tooth of the line (so the halves meet with no notch), and when varied it is sized about the line and never turned (turning would open the middle).

**Veins.** Each Line piece has a vein: **Width**, **Depth** (below 0 a groove, above 0 a ridge, a share of the full height) and **Shade** (darker or lighter). The vein's profile is worked out per pixel from the exact distance to the line (rounded: 1 − (d / half-width)²), in short steps, wrapping round the texture, the strongest vein winning where they meet. It is added to the Shape box's Height and shades its Colours **only where the shape is** (times the mask). Stroking the line several times at narrowing widths was considered and left: it bands at large sizes. New lines start as a gentle darker groove; lines from older files open with no vein, so they look as they did.

**Controls.** No new keys or mouse actions: the ↻ buttons, the edge kinds and the sliders are in the pieces list.

## Decision

- **`shapes`:** a new `edge` module (`Edge`, `EdgeKind`, `edged`, `AMOUNT`, `SIZE`, `MAX_POINTS`); `drawing/variety.rs` (`Vary`, `Vein`, `Variation`, `first_seed`, `next_seed`, `Drawing::variation`, `drawn_colour`, `with_edge`, `with_vary`, `with_vein`, `with_new_seeds`; `drawn_paths` moved here: placed, varied, toothed, mirrored); `Piece` gains `seed`, `edge`, `vary`, `vein`, and `same_shape` (what linked copies share); `Path::flattened`; `raster::veins` (`Veins`) and `raster::layers` using each piece's drawn colour. `Drawing::key` covers all of it.
- **`matgraph`:** the Shape box adds the veins to its Height and Colours; no new settings or outputs.
- **`fileio`:** **file format 102**: `v100::PieceV100` gains `seed`, `edge` (`v102::EdgeV102`), `vary` (`VaryV102`) and `vein` (`VeinV102`), each left out when as new; older files open smooth, unvaried, with the seeds their ids give and lines without veins.
- **`ui`:** the pieces list's selected-pieces rows gain **Edge** (the five kinds, Amount, Tooth size in centimetres from the graph's real-world size, Seed and ↻), **Vary copies** (Each copy differs, Size ±, Angle ±, Colour shift, ↻ New look) and **Vein** (Width, Depth, Shade); each change goes to every selected piece as one undo step. The canvas keeps the drawn outlines until the drawing changes.

## Consequences

- A twig of linked leaves can look hand-grown: one shape to reshape, every leaf its own.
- **Milestone 14 (foliage output)** gets toothed, varied leaves in the Mask (opacity) and veins in the Height (normals) and Colours with nothing more to do; an atlas of several twigs can vary each one by seed.
- The **Leaves** starter graph (left by milestone 8 until this milestone) can now be written; it is left for milestone 14, which adds the opacity and atlas it should show.
- Because varying happens where a piece is drawn, the gizmo and the selection's middle still follow the pieces' places, not their varied look.
