# 0259. The first material boxes

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 12's third milestone ([the plan](../plans/phase-12.md), [decision 0256](0256-material-graph-plan.md)) gives the material graphs their first full set of boxes, about 25, so a stylised stone-tile material can be made from an empty graph. Milestone 1 ([decision 0257](0257-material-graphs-core.md)) had made seven, which are released in projects: their box, plug and setting ids can't change. A few things were left to choose: how sizes are given, how Gradient map's colours are kept, what the colour-to-shape box the refusals suggest does, and how the patterns stay seamless.

## Options

- **Sizes** (a bevel's width, a blur's reach): as a share of the texture, as pixels, or as **centimetres on the finished material** (chosen). The graph already has a real-world size (Material out's), so a length in centimetres means the same thing at any texture size and on any model: a 2 cm bevel stays 2 cm whether the texture is 256 or 4096 pixels across, or covers 1 m or 3 m. Counts that must be whole for the texture to tile (tiles across, cells across, repeats) stay counts.
- **Gradient map's colours:** separate colour and place settings (no file change, but no real gradient editor), or a **gradient setting** (chosen): 2 to 8 colours, each at a place from 0 to 1, with its own editor row in Attributes. This needs a file format version.
- **Rotation:** any angle, or **quarter turns and flips only** (chosen) in Transform: any other angle would break the tiling. Basic shape's Turn is any angle, because each shape is joined with its neighbours' and stays seamless.
- **Overlap with milestone 1's boxes:** new boxes beside them, or **extending** them (chosen) where they overlap. Tiles gains a **Random** output (a grey per tile) and a Seed; Bricks is its own box (gaps the same width all round, rows made even); nothing is renamed.

## Decision

- **23 boxes**, in the working order:
  - **Make:** Noise, Clouds (soft, billowy or ridged), Cells (distance, bumps, edges or flat, and a grey per cell), Gradient (up and down, ramp or round; across, down or diagonal), Bricks, Tiles, Planks (random board ends, a grey per board, grain), Basic shape (circle, square, polygon, star).
  - **Shape:** Blur, Warp (by a height, or its own noise), Levels, Edge finder (inside, outside or both), Bevel (width, roundness, height; edge Round, Sharp or Steps), Transform (move, quarter turns, flips), Repeat (copies in a grid, shifted rows, each a little darker at random), Blend.
  - **Colour:** Colour fill, Gradient map, Hue and saturation (through a shape if given), **Colour to shape** (by lightness or closeness to a colour; a shape and the same greys as a height).
  - **Finish:** Material from height, Mix materials (by a shape or by height, every map at once), Material out.
- Each has a plain name, "What does this do?", plugs of the right kind, settings with ranges, and "usually comes next". Colour to shape is the box the refusal "a colour has no inside or outside" now offers to put between.
- **Lengths:** a new setting kind, `SettingKind::Length` (centimetres, held as a number). `Job::length_px` turns it into pixels with the graph's real-world size, which the evaluator reads from Material out (100 cm without one). Only boxes with a length are worked out again when the real-world size changes. Attributes shows lengths as sliders in cm.
- **Gradients:** `SettingKind::Gradient` and `SettingValue::Gradient(ColourGradient)`: 2 to 8 colours kept in the order added (so the list doesn't jump while a place is dragged), read sorted by place. Attributes shows the gradient as a strip, each colour with its place and a button to take it out, and **Add a colour** (put in the widest gap).
- **Seamless and repeatable:** every pattern repeats a whole number of times across the texture, random numbers come from hashing wrapped cell positions with the seed, and every box reading neighbours wraps round. Distances to an outline (Bevel, Edge finder) use an exact distance transform (Felzenszwalb and Huttenlocher) on rows lengthened by wrapping round. The maths is the well-known algorithms, written for the app; nothing is copied from Material Maker or any other program, so no licence notice is needed.
- **File format 96:** version 95 with gradient settings. A gradient without 2 to 8 colours is refused as damaged; places and colours out of range are brought into range, as other settings are.

## Consequences

- The stone-tile graph of the gate (Tiles → Warp → Bevel → Blend with noise → Gradient map → Material from height → Mix materials with moss → Material out) runs end to end; a test checks it tiles and is the same on any number of cores.
- Transform and Repeat take heights (shapes convert); colour pictures can't be moved yet. Moving colours can come with a later box if needed.
- At small sizes, gaps narrower than a pixel disappear (Tiles' standard gap at 32 pixels), so Bevel sees one big shape. The pictures on the canvas are at least 256 pixels, where the standard gaps are wider.
- The heavier boxes take a few hundred milliseconds at 2048 pixels on two cores (Noise and Planks about a second); the GPU milestone, if needed, can move them.
- Projects saved with a gradient need this version or newer to open.
