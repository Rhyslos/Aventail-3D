# 0228. The Paint tab in folding sections

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

The mockup's Paint tab ([0223](0223-paint-page-like-the-mockup.md)) has channel buttons with coloured dots. Under them, the brush's settings sit in folding sections, each with a few words on the right saying what's set inside: Tip (with a picture of the tip), Lays down, Pen and smoothing, Symmetry and trim, Paint through and effects. The app's tab was one long list, with only Pen and smoothing folding. It ended in a long sentence of keys that the panel's edge cut off.

## Decision

- **Folding sections**, each with a line above it, a drawn arrow, its name, and on the right what's set inside:
  - **Tip**, open to start with: a picture of a dab with the brush's hardness, then **Shape** (the tip), **Size space**, **Alignment** and **Paint on**. Size, Flow, Hardness and Spacing are added here only when the brush bar isn't shown.
  - **Lays down**, with the realism channels and a brush that lays them down: the channel rows.
  - **Pen and smoothing**.
  - **Symmetry and trim**: mirror, and the trim row with a trim sheet (a note says what it's for without one).
  - **Paint through and effects**: Paint through and the effect brushes' settings.
  - **Keys for painting**, closed to start with: one key per line. It isn't called "Keys", as the menu bar has a Keys menu.
- **The polygon fill tool** has Fills, Lays down, and Symmetry and trim.
- **Open or closed** is kept per section, as egui keeps folding state.
- **Arrows are drawn**, not typed, as the interface font has no ▾ or ▸. The group arrows in the Paint layers list are drawn the same way.
- **Channel buttons** get a dot: the brush's colour for Colour, greys for Rough, Metal and Height, amber for Glow; none for All.
- No keys or mouse actions are added ([0028](0028-ask-about-controls.md)).

## Consequences

- The tab is short with the sections closed, so the brush's main settings are in view without scrolling.
- A setting in a closed section is a click further away; the words on the right say whether it's on.
