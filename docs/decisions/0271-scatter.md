# 0271. Scatter

- **Status:** Accepted
- **Date:** 2026-11-13

## Problem

Milestone 9 of Phase 12 ([the plan](../plans/phase-12.md)): "Place copies of a shape or a material in a grid or at random, with random size, turn, colour and height. This covers pebbles, grass tufts and scales." Repeat copies a picture in a neat grid of equal copies; Cells and Tiles make one shape per cell. Nothing could place many different-looking copies of something you made, or a whole material, as ground cover needs.

## Options

**One box or several:**

- Separate boxes for shapes, colours and materials ("Scatter colour"). Rejected for the reason decision 0270 gave Transform and Repeat their three plugs: a wire's colour says what it carries, and one box that moves any kind is easier to find.
- **One Scatter box** (chosen) with a **Shape**, a **Colour** and a **Material** input (none needed), and outputs for what is plugged in (`BoxType::run_outputs`, empty outputs refused with why, as in 0270), plus a **Height**, **Copies** (where any copy is) and **Random** (a grey per copy) that are always made.

**With nothing plugged in:**

- Refuse, as Transform and Repeat do (`needs_an_input`). Rejected: Scatter makes something useful on its own.
- **Round pebbles** (chosen): a dome per copy. Dropping a Scatter shows what it does straight away, and it is offered as a maker of heights and shapes.

**A copy's outline:** the Shape input's greys (a height there becomes each copy's height; a copy covers where they aren't black, fading out over the darkest tenth). With only a colour or a material, each copy is the whole picture, square, with a one-pixel soft edge so it isn't jagged.

**Where copies overlap:** the one highest at that pixel is on top (a heap of pebbles), the Height output taking the highest everywhere; square copies (no height to compare) stack in a random order. Colour and material come from the copy on top, faded by how much it covers into black (colours) or a black, low, flat material between copies: Mix materials through **Copies** lays them over a ground.

**Placing:**

- **At random:** about one copy per square **Spacing** wide (`1 / spacing²` copies), uniform places from the seed.
- **In a grid:** Spacing fitted to a whole number across so the grid tiles; **Loose** strays each copy from its place; **Shift rows** moves every other row along, and makes the rows even so it still tiles (scales, roof tiles).
- **Keep apart** (off to start with): copies are tried in order and one whose round outline would touch one already placed is left out (at random, up to four tries per wanted copy). Shrinking copies to fit was considered and left: it changes sizes the person chose.
- **Where** (a shape input, called Where as on Blend and Mix materials): a copy lands where a random number is under its grey at the copy's middle, so greys thin copies out.
- Sizes in **centimetres** (Spacing, Size), through the real-world size, as every length (decision 0259). Size varies makes copies smaller only, so Size is the biggest copy's width; Turn and Turn varies are degrees; Height varies lowers, Colour varies lightens, darkens and tints a little.

**Seamless and the same every time:** places are 0 to 1 and wrap; distances for Keep apart are taken the short way round the wrap. Placing runs on one core in a fixed order from the box's seed; only the pixels are shared over the cores.

**Speed:** a grid of lookup cells about as wide as the biggest copy reaches, each listing the copies reaching into it, with the copy's middle moved by whole textures for copies reaching over an edge (so a copy bigger than the texture, seen from several sides, works too). Each pixel looks only at its cell's copies, first by a square bound, then turned back into the copy. Limits: at most **16 384 copies**, and never so many that their areas add up to more than **32 textures** (the rest left out), so a dense pile of big copies can't take minutes. At 2048 pixels on two cores (release, `scatter_speed`): pebbles 0.13 s, a material 0.63 s, scales in a grid 0.35 s, 16 384 grass copies 1.9 s, the deepest pile allowed 3.8 s.

## Decision

- **matgraph:** `boxes::scatter` (the box, in the **Detail** group, its first) and `boxes::scatter::placing` (`Plan`, `place`, `Placed`, `Lookup`, `MAX_COPIES`, `MAX_GRID`, `MAX_DEPTH`). Plug keys: inputs `shape`, `colour`, `material`, `where`; outputs `material_out`, `colour_out`, `out`, `copies`, `random` (a box's picture is its first output made: the material or colour when plugged in, else the height). Setting keys: `arrange`, `spacing`, `size`, `size_vary`, `turn`, `turn_vary`, `height_vary`, `colour_vary`, `apart`, `loose`, `shift`, `seed`. `moving::plain` is shared for the filled `run`.
- **Starters:** the **Ground** starter's pebbles are now a Scatter (kept apart, sizes varied, Random colouring each) instead of Cells' tallest bumps through Levels: rounder pebbles of different sizes that never overlap. Its file and card were written again and looked at.
- No new keys or mouse actions, and no file format change: a new box kind and its settings are ordinary graph data (an older app keeps it as a box from a newer app, as decision 0257 set out).

## Consequences

- Pebbles, gravel, scales, studs, grass tufts and leaf litter are one box, and the copies can be anything a graph makes.
- Stamp images (milestone 22) can reuse `placing` (places, the lookup) and the copy-on-top rule, with an image instead of the Shape input.
- A copy is the whole input picture made small: a shape touching its picture's edges gets square sides (the manual says to keep the shape smaller than its picture).
- The look of Scatter depends on the order of random numbers from the seed: changing that order later changes every graph using it, so it stays as it is.
