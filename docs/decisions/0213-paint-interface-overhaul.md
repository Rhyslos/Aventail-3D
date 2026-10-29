# 0213. The paint interface overhaul, and the Paint tab

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

After Phase 11 the project lead's first look at Paint mode was that the Attributes panel was crowded: the brush's settings, its pen, its realism rows, the whole colour picker and the palettes sat on top of the object's own settings, so one long column held two different things (the object, and how you paint). They asked for the painting settings to be tidied away from the object's.

## Options

Seven layouts were mocked up and discussed with the project lead on 2026-10-29 (the Design artifact "Paint workspace"): a Paint tab beside Attributes; a Colour tab only; Brush and Colour tabs; a Cinema 4D-style grouped manager; a close copy of Substance Painter; a brush strip docked under the views; one slim bar with brushes on the right. Research into Substance 3D Painter, ArmorPaint, Blender, Krita, Photoshop and Procreate found five things they share: the few numbers changed all the time sit in a bar over the view; colour is one small square that opens a picker; brushes and colours live in a library; layers are on the right with the rest of the settings under them; tools sit in a strip. The project lead then proposed a layout in the style of Solis's tile map screen (tools inside the viewport), and the two were merged.

## Decision

The paint interface is reworked in six pull requests, one piece each (plan: [docs/plans/paint-interface.md](../plans/paint-interface.md)):

1. **The Paint tab** (this pull request): the brush's settings move out of Attributes into a **Paint** panel, a tab beside Attributes in the Paint layout, in front. Attributes shows only the selected object while painting. A saved Paint layout from before gets the tab added beside Attributes.
2. **A floating brush bar** over the top of the viewport: the colour square (its picker a pop-up), the brush, Size, Flow, Hardness and Spacing, and Mirror, Pen and Smoothing switches, in the floating style of the viewport's own buttons.
3. **The paint tools** in a floating bar along the viewport's bottom, beside the existing shading and navigation buttons.
4. **A Colours tab in the brush shelf**: the palette, colours painted with saved there by themselves (Used lately), double-click a colour to edit it in the pop-up. The colour picker leaves the Paint tab.
5. **Showing one channel** (All, Colour, Rough, Metal, Height, Glow) from the top of the Paint tab, with new Height and Glow views.
6. **A setting** that puts the paint tools in the left bar (the mode palette, at its usual width) instead of floating at the bottom; floating stays the standard.

No slots or hotbar: the shelf's Brushes and Colours tabs do that job without covering the viewport (the project lead, 2026-10-29). Keys come later; the only new mouse action is double-clicking a colour to edit it.

## Consequences

- Attributes means one thing again: the object. The brush's settings have their own place, and in later steps most of them move nearer the painting.
- The brush's state still lives in the Attributes panel's code (`AttributesPanel::brush`), drawn by the Paint tab: a smaller change than moving it, and the brush bar, keys and tests already reach it there.
