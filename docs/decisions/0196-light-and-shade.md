# 0196. Light and shade

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's eighteenth milestone ([0177](0177-texture-painter-plan.md)), part of the hand-painted kit. Hand-painted textures carry their own lighting: light on the parts that face the sun, shade on the rest, warm light against cool shade. Working that out by eye on every side of a model is the slow part of a first pass. The project lead asked for a helper that pre-paints soft light and shade from a sun direction as a starting layer, with a switch to turn it off for those who don't want lighting while texturing.

## Decision

- **A light-and-shade layer** is a paint layer that keeps its settings (`paint::Lighting`, `Layer::lighting`), made from them on the shape like a gradient ([0194](0194-gradient-fills.md)):
  - the sun's direction as two angles: **Sun around** (0° from the right, 90° from the back, 180° from the left, 270° from the front) and **Sun height** (0° on the horizon to 90° overhead);
  - the **Light** colour (warm by default) and the **Shade** colour (cool blue by default);
  - **Softness**, from a hard cartoon line to a long fade; the turn sits a little past side-on, so light wraps round as painters draw it;
  - **Cast shadows**: parts of the object shade what lies behind them from the sun (rays toward the sun, a few spread by the softness so shadow edges soften too).
- **+ Light and shade** in the Paint layers panel adds one above the chosen layer, set to **Overlay** at **60%** so it shades the colours below instead of covering them. The default sun is high, from the front left.
- **The switch to turn it off** is the layer's eye: hidden, it costs nothing and paints nothing. Its blend and opacity change like any layer's.
- **Not painted into:** a stroke with it chosen goes on a new paint layer above it (`Layer::is_made`, shared with gradients).
- **It follows the model:** made again when its settings, its shapes or the texture size change, in the same undo step.
- **Saving:** project format 74 saves the settings with the layer.

## Consequences

- A first pass of light and shade is one click, and stays right through modelling.
- Shadows are cast only within each object, not from one object onto another sharing the material. Setting the sun from the current view, and more than one light, are left for later.
- The light is baked into the texture, as hand-painted games expect; it does not move with the game's lights. That is the point for a stylised look, and why it can be hidden.
