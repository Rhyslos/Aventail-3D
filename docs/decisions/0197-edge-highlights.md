# 0197. Edge highlights

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's nineteenth milestone ([0177](0177-texture-painter-plan.md)), part of the hand-painted kit. Hand-painted textures pick out the outer edges of a shape with a thin light line, where the light catches them: it makes the form read from far away. The project lead chose an edge highlight from the curvature map (DESIGN.md, texturing choices).

## Decision

- **+ Edge highlight** in the Paint layers panel adds a ready-made layer above the chosen one, built from parts that already exist:
  - a **fill layer** of warm near-white (`[255, 246, 225]`), named *Edge highlight*;
  - blend **Screen** at **70%**, so it lightens whatever colour is below instead of painting over it (a red edge turns light red, not white);
  - a **smart mask** of kind **Edges** ([0195](0195-smart-masks.md)), from the shape's curvature: a narrow band (width 12%), softly bordered (softness 35%) and lightly broken up (noise 15%, small).
- Everything about it is changed where it already lives: the colour in the layer's settings, the width and breakup in its smart mask's settings. Switching the mask to **Cavities** and the blend to Multiply makes the opposite, dark creases.
- One click, one undo step; the mask is made on the shape and follows modelling like any smart mask.
- No new file data: it saves as a fill layer with a smart mask (format 73).

Substance Painter's starting layers work the same way (a fill with a generator mask), as asked when in doubt about controls.

## Consequences

- Edge highlights take one click, and stay right through modelling.
- A highlight that follows the colour below exactly (a lighter version of each painted colour) comes from Screen; a separate "lighter than below" colour mode is left for later.
