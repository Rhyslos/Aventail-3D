# 0207. Realism channels and the realistic preview

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's twenty-eighth milestone ([0177](0177-texture-painter-plan.md)), the first of the realism group: "roughness, metal, normal, height and glow per layer, and a lit realistic preview with a turnable light." Realistic game materials are described by more than colour: how rough or shiny the surface is, whether it is metal, and what light it gives off. Substance Painter paints all of these at once: each layer changes some channels, and the brush lays down a value in each.

## Options

- **A texture set per channel** (a roughness set with its own layers): simple to build, but a rusty patch would be painted three times, and layers couldn't hold a whole look.
- **Channels per layer (chosen):** as Substance Painter. A layer holds its colour and, for each channel it changes, painted pixels (a paint layer) or one value (a fill layer).

## Decision

- **Channels:** Roughness, Metal and Glow (`paint::channel`). Height and normal come with milestone 32 (height and normal painting), which builds on this.
  - Roughness and metal are kept as grey (0 smooth or not metal, 255 rough or metal); glow as a colour.
  - Where no layer changes a channel, its standard value shows: roughness 80%, not metal, no glow.
- **Per layer** (`Layer::channels`): a part for each channel it changes. A channel mixes its layers like the colour, in order, with their opacity, masks, clipping and visibility, always by the Normal blend (`channel_view` feeds `Composite`). Merging down mixes each channel too; paint following the model and texture size changes move and resize them with the colour.
- **Per texture set, a switch** (`TextureSet::realism`; Paint layers → **Realism channels**). Hand-painted sets keep to colour, so the panels stay simple. The Realistic starting layout ([0206](0206-presets-and-starting-layouts.md)) turns it on and gives its layers roughness and metal.
- **Painting:** while the set uses realism, Attributes shows **Roughness**, **Metal** and **Glow** under the brush and the polygon fill tool: each ticked channel is painted with its value as the colour is painted.
  - Roughness (50%) and metal (0%) are ticked as standard, as Substance Painter's brushes paint every channel.
  - The eraser erases every channel the layer has.
  - Effect brushes and masks paint colour only.
  - A fill layer's settings get the same three rows, each with a tick and a value.
- **Seeing it:**
  - **Display → Paint preview → Realistic** lights the surface the way engines light realistic materials: GGX microfacet light from a sun, the sky's light and its reflection (blurred on rough surfaces), glow added, Filmic tone mapping. **Turn the light** (0–360°) under it turns the sun round the model. A slider, not a mouse action, so no controls change.
  - **Roughness only** and **Metal only** show a channel as greys.
  - Godot's preview adds the glow.
- **On the GPU:** besides its colour, each painted set uploads a surface texture (roughness in red, metal in green, linear, over the standard values) and a glow texture, bound beside the colour (`render::PaintMap`); only changed tiles go up.
- **Saving:** project format 78 stores the switch (`SetV64::realism`) and each layer's channels (`LayerV64::channels`: painted tiles or a value), checked on opening. Presets keep channel values and leave out painted channel pixels, like the colour's.
- **Exporting** the channels (glTF metallic-roughness and emissive, and each engine's packing) is milestone 29.

## Consequences

- Realistic materials can be painted and checked in the app; exporting them follows next.
- Each painted channel adds an image per layer, kept only where painted (tiles), so hand-painted work costs nothing more.
- A brush stroke with realism on paints up to three more images; strokes stay smooth on 2048 textures.

## Amended 2026-10-30

The first painter review found height or roughness couldn't be painted without colour. The brush, the polygon fill and fill layers now have a **Colour** tick (with the realism channels on). Unticked, a stroke paints its colour with no alpha, so nothing changes in the colour and the kept stroke paints the same when made again; a fill layer's colour gets alpha 0.

## Amended 2026-10-30, room and smaller fixes

The realism switch is noticed whatever tool is in use (it was read after the select and move tools returned early).
