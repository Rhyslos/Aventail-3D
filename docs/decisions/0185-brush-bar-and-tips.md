# 0185. The brush bar and brush tips

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's eighth milestone ([0177](0177-texture-painter-plan.md)), first part: a stylised brush set and a place to choose brushes, as in Substance Painter's shelf, in the brush bar the plan put in Coordinates' place (mockup A). Effect brushes, projection and clone are the second part ([0186](0186-effect-brushes-projection-clone.md)).

## Decision

- **Tips** (`paint::Tip`): the shape each dab paints. **Round** (the falloff by hardness), **Chalk** (round, broken by grain fixed to the texture, so it reads like chalk on rough paper and doesn't swim as you paint), **Bristle** (nine soft lines along the stroke, each its own strength), **Dry brush** (bristles that skip where the grain is high) and **Spatter** (ten specks scattered in the dab, different each dab). Bristles turn to follow the stroke. A dab knows its angle and its number in the stroke; the same tips paint on the model and flat in the UV Editor.
- **Strokes keep their tip** (project format 70). A stroke's dab angles come back from its dabs' order when painted again.
- **Brushes are presets** (`ui::brush_bar::BrushPreset`): a tip with hardness, spacing, flow and the pen curves. Choosing one keeps the brush's size and colour, so switching brushes doesn't lose them. The app comes with **Soft round, Hard round, Chalk, Bristle, Dry brush** and **Spatter**; **+** keeps the brush as set now under a name. The person's own brushes are the app's (kept between sessions, like the layouts), not the project's: up to 200, any out of range left out when read.
- **The brush bar** (`Panel::Brushes`): along the bottom of the Paint layout, with Coordinates as a tab behind it (Coordinates must stay in every layout); cards with a sample stroke drawn with the brush's tip and hardness, and its name; **All / Built in / Saved**; a search box; **+** to keep a brush; right click on one's own brush deletes it. The bar keeps its height (it isn't Coordinates' slim band).
- **Tip** is also a choice under the brush in Attributes.
- **The Keys window** lists Paint mode's keys and mouse actions, in a Paint mode colour of their own (`[` and `]` among the mouse actions, as the drawn keyboard has no bracket keys on every layout).

## Consequences

- Hand-painted textures can be blocked in with a soft round, broken up with chalk and dry brush, and given texture with bristle and spatter without leaving the app.
- Brush rotation by Ctrl + left drag up and down (in the plan) and tilt are still to come: bristles follow the stroke for now.
