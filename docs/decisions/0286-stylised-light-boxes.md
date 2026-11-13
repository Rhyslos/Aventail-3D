# 0286. Stylised light boxes: Toon bands, Painted highlight and Colour steps

- **Status:** Accepted
- **Date:** 2026-11-13

## Problem

Phase 12's eighth extra (milestone 23, [decision 0269](0269-material-extras.md)): boxes for a **hand-painted or cel look**: light quantised into a few **toon bands**, **highlights painted in from a chosen light direction**, and **colour steps** (posterising). They should work on colours and whole materials, tile seamlessly, keep to the project's palettes where asked, and build on the light maths the Directional light box and the painter's light-and-shade helper already share (`matgraph::light`, [decision 0272](0272-more-boxes.md)) rather than copy it.

## Options

- **One "Stylise" box with modes, or three boxes.** One box would hold three unrelated sets of settings (a sun, an edge stroke, a number of steps) and only ever do one of them; in a graph the three are usually used together, one after another. **Three boxes**, each with one job, read better on the canvas and in the box search.
- **Toon bands as a setting of Directional light.** Directional light's Softness 0 already gives one hard line, but two tones only, and its soft-light blend keeps the colour's own shading. Cel shading wants **N flat tones multiplied over the colour**, and a palette option: a box of its own, sharing Directional light's sun settings, slopes (`slope_normal`), shadows (`shadow`) and the painter's turn (`light::lit`).
- **Where the bands come from.** Cutting `lit` at its hard setting gives only two tones. Instead the light is worked out at its **softest** (`lit(facing, 1.0)`: a smooth ramp from shade to light, monotonic in how a place faces the sun), shadows taken off, then cut into Bands even steps; Softness blurs only near each line. Flat ground's band is then exactly the painter's light cut into bands (tested).
- **A palette for the bands.** Swatches per band (sorted dark to light) when nothing is plugged in; with a colour, the shaded colour **snapped to the nearest swatch** (Palette map's Nearest, Oklab), so shade and light keep both the material's hue and the game's colours.
- **Painted highlight: which edges.** Edge highlight's idea (the height above a soft blur of itself, Width wide) finds the edges; the slope of that blur gives each edge a **steady direction** (the raw height's slopes are noisy). Only edges whose outside faces the light (Around) are lit; **Spread** widens how far round. A sun height isn't needed: a painted rim or top light is about direction on the texture.
- **Colour steps in lightness.** Cutting the sRGB lightness and scaling the colour shifts hues and pushes channels past white. Instead the **Oklab** lightness is cut (each step the middle of its band, so colours never go fully black or white), hue and strength kept; a colour too strong to show that light is made **less strong, never lighter or darker** (`palette_map::from_oklab_kept`). **Each channel** cuts red, green and blue for the poster look. Dither is Palette map's 4 × 4 pattern.

## Decision

- **Toon bands** (`toon_bands`, Colour): inputs Height, Colour, Material (a material's own normals when no height is plugged in); outputs Colour, Material, Bands (which band, as a height); settings Around, Sun height, Depth (as Directional light), **Bands** (2 to 8, 3), **Softness** (0.15), **Colours** (Shade and light, Palette), **Shade colour**, **Light colour** (multiplied over the colour), **Palette**, **Shadows** (on), **Amount**.
- **Painted highlight** (`painted_highlight`, Colour): inputs Height (or a material's height), Colour, Material; outputs Colour, Material, Highlight; settings **Around** (90: from the top), **Spread** (0.35), **Width** (cm, 1), **Softness** (0 a crisp stroke), **Highlight colour**, **Amount**.
- **Colour steps** (`colour_steps`, Colour): Transform's plugs (a height in terraces, a colour, a material's colour); settings **Steps** (2 to 32, 4), **Mode** (Each channel, Lightness only: the standard), **Dither**, **Snap to a palette** (off), **Palette**.
- Toon bands and Painted highlight refuse, in plain words, when only a colour is plugged in ("needs a height or a material…"). A palette is read only when Colours is Palette or Snap is on; otherwise no palettes are needed. None has randomness (no Seed: nothing for the dice).
- **Shared maths:** `directional_light::slope_normal`, `shadow` and `degrees` are shared (Directional light unchanged); `palette_map::{nearest, bayer, from_oklab, from_oklab_kept}`.
- **Starters unchanged:** the Stylised wood starter already reads well; adding these would change its card without being clearly better. Using them is a tip in the manual instead.
- **No new keys or mouse actions; no file format change** (the settings are the existing kinds).

## Consequences

A cel-shaded stone is Bevel → Gradient map → Toon bands → Painted highlight → Colour steps → Material from height. The boxes tile because everything they read wraps and the dither repeats every four pixels. Speed at 2048 on two cores (matgraph optimised): Toon bands about 1.9 s with shadows, Painted highlight 0.4 s, Colour steps with a palette snap 0.9 s. Painted highlight finds edges by the height's steps, so a gentle height (soft noise) gets little highlight: the manual says to use it on a Bevel's height or after Levels. Pixel-art output (milestone 24) can reuse Colour steps' dither and snap.
