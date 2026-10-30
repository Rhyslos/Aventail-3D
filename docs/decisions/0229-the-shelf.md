# 0229. The shelf like the mockup

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

The mockup's shelf ([0223](0223-paint-page-like-the-mockup.md)) has five tabs: Brushes, Colours, Materials, Smart masks and Stencils. The brush filters come after the tabs, and the search sits on the right. The brushes are small square pictures with their names under them, and the chosen one is outlined. The app's shelf had two tabs and wide cards with a sample stroke, and the search and buttons came straight after the filters.

## Decision

- **Five tabs.**
  - **Materials:** a card for each material kind (Brushed metal, Wood, Stone, Paint coat), its picture in the material's two colours. A click adds a material layer of that kind above the chosen layer, as ··· → + Material does.
  - **Smart masks:** a card for each kind. A click gives the **chosen layer** that smart mask, replacing any mask it had; with no layer yet, it adds a grey fill shown through it. Wear, Dirt and Rust show only with the realism channels, as with + Dirt, + Wear and + Rust.
  - **Stencils:** the projection brush's picture, once chosen (a click switches to the projection brush), and **Choose…** to pick a file.
- **Cards** 76 × 70 points: a 46-point picture (a dab for a round tip, a short stroke for a textured one), the name under it, and an accent outline for the brush in use. Each card's name is also its accessible name ("Brush Soft round", "Material Wood", "Smart mask Cavities", "Stencil …").
- **The Brushes header:** tabs, then All, Built in, Effects and Saved; on the right, from the edge in: Search, Load brushes…, Save brushes…, and + (keep this brush).
- Each card's click is one undo step. No keys or mouse actions are added ([0028](0028-ask-about-controls.md)); dragging a card onto a layer, as some programs allow, would be a new mouse action and waits for the project lead.
- Coordinates stays as a tab behind the shelf, for moving things while painting.

## Consequences

- With milestone 13, the Paint page matches the mockup ("Final · shelf at the bottom"), apart from the parts the project lead chose to keep: the layout tabs in the same place on every page, the status bar, and move, rotate and scale in the tool row.
- The shelf's material and smart mask cards are another way to the same layers as ···, so the two must stay in step.

## Amended 2026-10-30, room and smaller fixes

The Smart masks cards and ··· share one rule (`set_smart_mask`): a smart mask changes kind and keeps Invert and its seed; a mask painted by hand is never replaced, and the status bar says why. + Material from ··· is named by its kind, as the shelf's are.
