# 0107. The numbered checker

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0102](0102-checker-and-stretch.md) (the Checker and Stretch views), [0009](0009-theme.md) and [0014](0014-studio-look-and-accent.md) (the theme and its accent colours)

## Problem

The project lead asked for a UV checker map with coloured cells and numbers in them, like the one in Cinema 4D, but in a gradient from the app's own colour theme. The plain Checker (0102) shows stretching well, but not *where* on the texture a part of the model lies. Labels tell that at a glance, and show turns and mirrors as the text turns or reads backwards.

## Options

- **Where:** **a fourth Surface choice, "Numbered checker"**, beside Colours, Checker and Stretch in the viewport's Display menu (chosen); remembered like the others. While it's on, the UV Editor shows the same image behind the UVs, so the two views can be matched by their labels. Replacing the plain Checker was set aside: its squares follow the texture's pixels, which the numbered one doesn't.
- **What it looks like** (made by the app, no image copied from anywhere):
  - 8 × 8 cells across the texture square, labelled with a letter for the column (A at the left) and a number for the row (1 at the top), drawn with a small pixel font of the app's own;
  - each cell a 4 × 4 checker of a lighter and a darker shade of its colour, with a thin darker line round it and its label on a dark badge.
- **Colours:** **from the accent colour at the top left to a colour a third of the way back round the colour wheel at the bottom right** (chosen), blended round the wheel, so every cell stays vivid. This runs red → magenta → blue for the standard red accent, and blue → teal → green for the blue one. Straight mixing went grey in the middle; turning the other way (+150°) went through yellow and olive with the red accent. The image is made again when the accent changes.
- **How it gets on the shapes:**
  - **a real image** (1,024 pixels square, with smaller copies for far away) that the shader reads by the UVs (chosen). The UV Editor shows the very same image.
  - drawing letters in the shader: complicated, and nothing the UV Editor could show.

## Decision

1. `uv::grid`: `numbered_checker(size, from, to)` (RGBA, first row the top of the square), `label`, `CELLS` (8), `SQUARES_PER_CELL` (4); colours blended round the wheel.
2. `render`:
   - `SurfaceDraw::NumberedChecker` (surface mode 3);
   - `ViewportRenderer::set_grid_image` (an sRGB texture with every smaller copy, repeating, smoothed);
   - the object bind group gains the image and its sampler;
   - the shader reads it with `textureSampleGrad`.
3. `ui`:
   - `Surface::NumberedChecker`;
   - `theme::grid_colors(accent)`;
   - `ViewportView::grid_colors`;
   - `grid_image` and `GRID_IMAGE_SIZE` (1,024);
   - the UV Editor draws the image behind the UVs (dimmed by `theme::UV_GRID_IMAGE_TINT`) while the choice is on (`UvContext::grid`).
4. `app`: makes and uploads the image when the choice is first shown, and again when the accent changes.

## Consequences

- No new keys, no file format change: the image is never saved, only made.
- About 4 MB more on the GPU (and in egui) once the choice has been used.
