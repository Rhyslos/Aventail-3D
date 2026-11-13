# 0272. More boxes: cracks, chips, scratches, fill the gaps, brush strokes, edge highlight, slope blur, directional light

- **Status:** Accepted
- **Date:** 2026-11-13

## Problem

Milestone 10 of Phase 12 ([the plan](../plans/phase-12.md)): "More boxes, for both the hand-painted and the realistic side: Cracks, Chips, Scratches, Fill the gaps (moss, dirt, grout); Brush strokes (a hand-painted look), Edge highlight, Slope blur; Directional light (painted-in light and shade, matching the painter's helper)." Wear, hand-painted looks and painted-in light had to be built from several boxes each (Edge finder, Levels, Blend, noise), or couldn't be made at all. Later milestones cover nearby ground: the weathering set (25: dirt in crevices, rust, edge wear, dust and water streaks following the model) and stylised light boxes (23: toon bands, painted highlights, colour steps). These boxes had to stay general so they don't overlap.

## Options

**Fill the gaps, given Mix materials:** milestone 3's Mix materials by height already lays a second material where it stands higher ("moss fills cracks"). Extending it was considered: it needs two whole materials and gives no shape of where the filling is. **A separate Fill the gaps box** (chosen) works on a height, fills its low places up to a level with a colour (patchy, by its own noise), and gives the gaps as a shape, the filled height, and the colour or material with the filling laid in. Its "What does this do?" points to Mix materials for filling with a whole second material.

**Directional light and the painter's helper:** copying the painter's maths into `matgraph` would let the two drift apart. `paint` already depends on `matgraph` (graph fills), so **the shared maths moved into `matgraph::light`** (`sun`, `lit`, `warmed`) and `paint::Lighting` calls it (chosen): the painter's results are unchanged (its tests pass as before), and the box turns from light to shade exactly as the helper does. The box has the helper's settings (Around, Sun height, light and shade colours, Softness, Warmth, Shadows) plus **Depth** (how deep the height is, in centimetres, so slopes and shadows are the right size) and **Amount**. Without a colour plugged in it gives the light and shade colours alone, as the helper's layer does; with one, they are laid over it by soft light (mid grey leaves it, lighter lightens, darker darkens), so the colour's own hues stay.

**How each box works** (all seamless and seed-made; sizes in centimetres through the real-world size):

- **Cracks** (Detail): the borders of a jittered grid of pieces (a whole number across), pushed about by noise (Wiggle), each border cracked or not by its own random number (Amount), narrowing and widening along its length. The exact distance to a border (the two-pass cell search) keeps widths in centimetres. Inputs Height (carved into; flat white without) and Where; outputs Height and Cracks.
- **Chips** (Detail): angular cells (a whole number across), each with a random bite, cut where the place is nearer the outline than its bite (the exact distance Bevel and Edge finder use). Needs a shape. Outputs the chipped Shape, a Height cut down (from Height, or the shape), and Chips.
- **Scratches** (Detail): placed as Scatter's copies are (`scatter::placing`: middles, lengths, turns, strengths from the seed; the lookup grid), each a tapering, slightly bowed line. Inputs Height and Where; outputs Height and Scratches.
- **Fill the gaps** (Detail): as above.
- **Brush strokes** (Colour): strokes placed by `scatter::placing` (about three over each place), each a long rounded dab taking the colour under its middle (Detail kept lets the colour beneath show), lighter or darker at random, streaked by bristles that show more toward its tail; later strokes on top. Colour and Material in (a material's colour only); Height out is the paint's ridges.
- **Edge highlight** (Colour): the height less a soft blur of it, Width wide: above is an edge (lightened toward a highlight colour), below a hollow (darkened). From the texture's own height, unlike the weathering set's edge wear, which will follow the model.
- **Slope blur** (Shape): samples along the slope of By (or the box's own noise, as Warp), Steps of them over Distance, averaged (Smear), the lowest (Wear down) or the highest (Build up). Takes a height, a colour and a material as Transform and Repeat do (`moving::plugs`, `run_outputs`, `needs_an_input`); colours and normals are averaged.
- **Directional light** (Colour): as above; shadows are found by looking along the texture toward the sun (wrapping), at most 160 pixels.

**Groups:** wear (Cracks, Chips, Scratches, Fill the gaps) is **Detail**, beside Scatter; Brush strokes, Edge highlight and Directional light change colour, so **Colour**; Slope blur reshapes, so **Shape**.

**Offering boxes between plugs:** with these boxes, a height dropped on a material plug would have offered Slope blur ahead of Material from height (only three are offered). A box that also takes the kind wanted only passes that input on (Transform's material out needs a material in), so **`boxes_between` now leaves out boxes with an input of the wanted kind**: Material from height is offered again, and Transform and Repeat no longer appear there (they could never turn a height into a material). The evaluator's "nothing plugged in" note names each kind once (Slope blur has two height plugs).

## Decision

- **matgraph:** `boxes::{cracks, chips, scratches, fill_gaps, brush_strokes, edge_highlight, slope_blur, directional_light}`, 37 standard boxes; `light` (shared with the painter); `pattern::whole_repeats` and `smooth`; `scatter::placing` shared with Scratches and Brush strokes (the order random numbers are drawn from a seed in Scatter is unchanged); Warp's `slope` and `steepest` shared with Slope blur.
- **Ids** (released from here on): `cracks` (in `in`, `where`; out `out`, `cracks`; settings `size`, `width`, `amount`, `wiggle`, `depth`, `seed`), `chips` (`shape`, `height`; `shape_out`, `out`, `chips`; `size`, `reach`, `amount`, `depth`, `seed`), `scratches` (`in`, `where`; `out`, `scratches`; `spacing`, `length`, `length_vary`, `width`, `direction`, `direction_vary`, `strength_vary`, `depth`, `seed`), `fill_gaps` (`height`, `colour`, `material`; `out`, `gaps`, `colour_out`, `material_out`; `level`, `softness`, `patchy`, `patch_size`, `fill_colour`, `colour_vary`, `roughness`, `seed`), `brush_strokes` (`colour`, `material`; `colour_out`, `material_out`, `out`; `length`, `width`, `size_vary`, `direction`, `direction_vary`, `keep_detail`, `colour_vary`, `bristles`, `seed`), `edge_highlight` (`height`, `colour`, `material`; `colour_out`, `material_out`, `edges`; `width`, `amount`, `highlight`, `hollows`), `slope_blur` (`in`, `colour`, `material`, `by`; `out`, `colour_out`, `material_out`; `distance`, `mode`, `direction`, `steps`, `scale`, `seed`), `directional_light` (`height`, `colour`, `material`; `colour_out`, `material_out`, `light`; `around`, `up`, `depth`, `light_colour`, `shade_colour`, `softness`, `warmth`, `shadows`, `amount`).
- **Starters:** the **Metal plate** starter gains Scratches (every way, faint, scored into the height and added bright to the colour): its file and card were written again and looked at; the other cards are unchanged.
- No new keys or mouse actions, and no file format change (new box kinds are ordinary graph data).

## Consequences

- Wear, hand-painted looks and painted-in light are a box each, for any graph.
- The weathering set (25) can build on Fill the gaps, Edge highlight and Cracks with the model's maps; the stylised light boxes (23) on `matgraph::light` and Directional light (toon bands are Softness 0 already; colour steps and painted highlights are theirs).
- The look of Cracks, Chips, Scratches and Brush strokes depends on the order of random numbers from their seeds: it stays as it is.
- Speed at 2048 pixels on two cores (matgraph optimised, `more_box_speed`, ignored), every input plugged in: Scratches 0.12 s, Edge highlight 0.25 s, Brush strokes 0.6 s, Chips 0.8 s, Directional light 1.4 s, Fill the gaps 1.9 s, Cracks 3.1 s, Slope blur 6.1 s (a height, a colour and a material, twelve samples each). Slope blur and Cracks are where a GPU move would help.
