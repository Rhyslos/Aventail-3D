# 0200. See it as the player will

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's twenty-first milestone ([0177](0177-texture-painter-plan.md)). Painters work close up, but players see a prop from metres away, a few hundred pixels tall at most. Fine detail vanishes, and the game reads the texture from its smaller, blurrier copies (mipmaps). The plan asked for "a switch that shows the model at game distance, with the blur and mipmaps the engine uses, while you paint".

## Decision

- **See it as the player will**, a viewport switch, in the shading button's list (bottom left) and the **Display** menu. While on:
  - each view is drawn into a **smaller image**, as many pixels tall as the model at the camera's focus would be on the player's screen, then **enlarged with its pixels kept** (nearest filtering), so you see exactly the pixels a player gets;
  - drawing that small makes the GPU read the texture's smaller copies, as a game does: the blur comes for free and matches;
  - a note at the top of each view says *As the player sees it: 5 m away on a 1080p screen* (or that the view is already as small);
  - painting, picking and navigation keep working: the point under the mouse is found in the smaller image.
- **Its settings,** shown under the switch while on: **Distance** (standard 5 m, a prop across a room), **Screen** (720p, 1080p standard, 1440p, 2160p) and **View angle** (standard 60°, most third-person games). They are remembered with the viewport settings.
- **How small:** the model's size in pixels per centimetre in the game, `screen height ÷ (2 × distance × tan(view angle ÷ 2))`, against the view's, from its camera's distance to the focus and its own angle (or a flat view's height). When the view already shows it as small or smaller, it's drawn as is. The image is never less than 1/128 of the view.
- **No keys:** a menu switch, like the paint previews ([0198](0198-paint-previews.md)).

## Consequences

- A painter sees in one click whether details survive at game distance, and works on what matters.
- The size is measured at the camera's focus: parts much nearer or farther than it are judged less exactly, as on a player's screen.
- The view's anti-aliasing and the engine's may differ slightly; the live engine preview (milestone 33) is the exact check.
