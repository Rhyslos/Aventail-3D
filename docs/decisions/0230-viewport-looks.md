# 0230. Viewport looks

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

The project lead found the viewport dark and flat. It had dark greys all round and mid-grey shapes under a soft light. A mockup ("Viewport look") showed the same scene in five looks. The project lead chose:

- the smooth grey gradient look as the new standard, called **Default**;
- **Studio** and **Dusk** as options;
- the current look kept as **Classic**;
- a **Custom** look with their own colours.

The layout stays as it is; only how the scene is drawn changes.

## Decision

- **Settings → Viewport → Look**, first in the section, chooses one of five; it is saved with the view options:
  - **Default** (standard): one smooth gradient, #65676b at the top, #46474b at the horizon, #2d2e31 at the bottom, with no horizon line. The grid is a light grey at 2.2 times the old strength. Light from the camera 0.42, from above 0.25, a faint shine 0.06.
  - **Studio**: a lighter grey studio with a horizon, a key light 0.8, a shine 0.3, soft shadows.
  - **Dusk**: a blue-grey sky and blue grid, a key light 0.75, a shine 0.25, soft shadows, warmth 1.
  - **Classic**: the old colours and light exactly.
  - **Custom**: the person's own top, horizon and bottom colours, horizon line, grid colour and strength, light from above, light from the camera, shine, warmth and soft shadows. **Start from** copies another look.
- **The renderer takes a look** (`render::FrameLook`) every frame:
  - a horizon line or one smooth gradient;
  - the grid's colour and strength;
  - the key light and the light from the camera (classic 0.5 and 0.12);
  - a shine;
  - warmth: a warm key light and cool shade;
  - up to 16 soft shadows.
- **Soft shadows** are round, soft, dark patches on the ground under each object's box:
  - wider than the box, and nudged away from the key light;
  - fading as the object rises;
  - laid by the grid's pass, so they sit on the ground and hide behind shapes in front;
  - in perspective views only.

  This is not a real shadow map: cheap, steady, and enough to show where things stand.
- **The no-GPU backdrop** (tests, a missing driver) uses the look's colours too.
- **Restore standard** goes back to Default and keeps the custom colours.
- **Names:** the standard look is called "Default". No other program's name appears in the app or the repository.
- **The selection outline** keeps the accent colour in every look.
- No keys or mouse actions are added ([0028](0028-ask-about-controls.md)).

## Consequences

- Everyone gets the brighter Default look on their next start; Classic brings the old one back.
- The uniform buffer grows from 352 to 656 bytes a frame: nothing measurable.
- Materials' own colours still decide what colour a shape is; a look changes only the backdrop, grid and light.
