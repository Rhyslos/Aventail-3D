# 0198. Paint previews

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's twentieth milestone ([0177](0177-texture-painter-plan.md)), part of the hand-painted kit. A texture looks different under different light. Hand-painted textures are usually shown unlit in the game; toon games shade them in bands; Godot lights and tone maps them; and painters check values (light against dark) and squint to see the big shapes. The viewport's soft studio light suits modelling, but hides some of this.

## Decision

- **Paint preview**, a viewport setting for the **Painted** surface, with seven choices:
  - **Lit** (standard): the viewport's soft studio light, as before.
  - **Unlit:** the painting's own colours, no light.
  - **Toon:** a lit band and a shaded band (the colour darkened and cooled) with a hard edge, from the studio's key light, and a thin rim of light where the surface turns away from view.
  - **Godot:** close to Godot's standard look: a sun of strength 1 from above, a sky's light from above and around, and Godot's Filmic tone mapping (its `tonemap_filmic` curve, white 1). Godot's own lights and settings will still change it a little.
  - **Godot toon:** the Godot look with Godot's toon diffuse (a material's Diffuse Mode set to Toon, roughness 0.1): the sun's light in a soft step.
  - **Value check:** the painting's colours as greys (their lightness), unlit, to check light against dark.
  - **Squint:** the painting unlit and blurred (read from a copy about 16 times smaller), as if squinting.
- **Where:** in the list of the shading button at the bottom left of the viewport, and in the **Display** menu, under **Paint preview**, both shown only while the surface is Painted. The button names the preview when it isn't Lit (*Shaded · Unlit*). It's remembered with the other viewport settings.
- **Cavity is left off** for every preview but Lit, since its shading would change what they show.
- **No keys:** the previews are a menu choice. Substance Painter switches its views from menus and a shortcut of its own; adding a key can wait until it's missed.
- **Rendering:** `render::PaintPreview` on `SurfaceDraw::Painted`, sent in the object's `surface.w`; the shader picks the lighting in `preview` and blurs in `surface_color`.

## Consequences

- A painter can check a texture the way the game will show it, and its values and big shapes, in one click each.
- The previews apply to painted surfaces only. The Godot preview is an approximation; the true check is the live engine preview (milestone 33).

## Amended 2026-10-30, room and smaller fixes

The one-channel previews (Roughness, Metal, Height and Glow only) are listed only while the painted material uses the realism channels (or while one is in use). The Lit preview is shaded by the viewport's look (decision 0230); the manual says so and points to Unlit for exact colours.
