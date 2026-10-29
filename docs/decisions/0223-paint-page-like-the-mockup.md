# 0223. The Paint page made like the mockup

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

After the paint interface overhaul ([0213](0213-paint-interface-overhaul.md)), the Paint page still carried the whole modelling interface: the modelling toolbar, the mode column, the view's menu line and labels, the grid, four views on a middle click, and panels laid out differently from the agreed mockup (the Design artifact "Paint workspace", board "Final · shelf at the bottom"). The project lead: the interface update isn't finished until the Paint page looks like the mockup.

## Decision

Seven pieces, one pull request each (plan: [docs/plans/paint-interface.md](../plans/paint-interface.md), milestones 7 to 13), with the project lead's choices:

- **The layout tabs stay where they are** (Standard, UV Edit, Paint, top right), the same on every page, for uniformity; the mockup's Model · UV · Paint · Bake row isn't used.
- **The status bar stays**, on every page.
- **The grid is off while painting**, with a **Grid** switch in the shading button's list (and Display), remembered.
- **One view while painting:** a middle click and F5 don't split it into four there (they still do elsewhere).
- **Move, rotate and scale stay** in the Paint page's tool row, for adjustments while texturing.
- The Paint page has no modelling toolbar and no mode column (the layout tabs and Esc leave it; the left bar returns only for the paint tools when Settings puts them there), and no view labels.

## Consequences

- Painting shows the model and what paints it, little else.
- Until the tool row is built (milestone 8), the view's menu line stays, so its menus can still be reached.
