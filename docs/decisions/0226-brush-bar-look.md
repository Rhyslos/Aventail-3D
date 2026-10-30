# 0226. The brush bar's look

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

The mockup's Paint page ([0223](0223-paint-page-like-the-mockup.md)) draws the brush bar differently from the app's: two colour squares (the brush's colour in front, a second one behind it), a drop-down with a small picture of the brush, the numbers as bars that fill with the value, and small picture switches for mirror, pen and smoothing. The app had one colour square, the tool's name as plain text, number boxes with arrows, and word buttons.

## Options

- Keep the app's bar and only restyle it: quick, but the numbers stay small boxes that are hard to read at a glance.
- Follow the mockup (chosen).

## Decision

- **Two colour squares.** The front one is the brush's colour and opens the colour picker as before. The one behind is a second colour, kept with the brush settings (dark grey to start); clicking the part that shows swaps the two. No key is added ([0028](0028-ask-about-controls.md)).
- **The brush drop-down.** A soft-circle picture and the tool's name; its list holds the ready-made brushes and the ones you kept, the same as the shelf. Choosing one sets the brush, as clicking it in the shelf does. With the bucket ("Fill the layer") and polygon fill, the name shows without a list.
- **Fill bars** for Size, Flow, Hardness and Spacing: the bar fills with the value. Dragging sideways changes it (Shift for small steps); a click lets you type. Size and Spacing fill on an eased scale, so small values, used most, take most of the bar.
- **Picture switches** for mirror (with its axis letter and the axis list beside it), pen pressure and smoothing; lit when on.

## Consequences

- Nothing moves: the bar keeps its place, groups and rows (0215), and every setting is still in the Paint tab.
- The second colour is a setting of the brush, not of the project: it isn't saved in project files.

## Amended 2026-10-30, brush handling

The bars for Size and Spacing change by a share of themselves as they're dragged (`EASED_PER_POINT`, about twice as big for 90 points), from the first point moved; a press opens typing only if it moved 2 points or less (`CLICK_SLOP`; egui's click allowed 6). While Size is dragged, the 3D view shows the brush's circle in its middle (a brush measured on the surface at the scale under the mouse when it was last over the model).
