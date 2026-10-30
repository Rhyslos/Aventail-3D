# 0227. The Paint layers on top, with pictures

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

The mockup's Paint page ([0223](0223-paint-page-like-the-mockup.md)) puts the layers at the top of the right column, beside Objects, and the brush's settings under them, beside Attributes. Each layer row has a small picture of the layer and of its mask, its name with its kind under it, and its blend mode over its opacity. Under the list are only **+ Paint**, **+ Fill**, **+ Group** and **⋯**. The app had the Paint and Attributes tabs on top and the layers under them. Its rows showed a small mark and the name. Above the list, two rows of buttons held every kind of layer and every action, and the realism switch and the presets row sat over the list.

## Options

- Keep every button visible: nothing to look for, but the panel stays long and pushes the list down.
- Follow the mockup (chosen): the three adds used most stay out, the rest go in one menu.

## Decision

- **The Paint layout:** Objects | **Paint layers** (in front) on top, Attributes | **Paint** (in front) under them, the top taking 45% of the column. A saved Paint layout that still has the layers under Attributes gets the new arrangement once; one already in the new arrangement, or changed by hand after, is kept.
- **Layer rows** 36 points high:
  - the eye;
  - a 26-point picture: the layer's pixels over a dark checker, a fill's colour, or stacked squares for a group with its arrow before it;
  - the mask's picture, if it has one: white where it shows, black where it hides; a click chooses it, as before;
  - the name, with its kind under it ("Paint", "Fill · smart mask · clipped"…);
  - the blend mode over the opacity on the right.
- **Pictures are made again only when the image changes.** Painted images share their tiles until painted ([0188](0188-speed-with-big-textures.md)), so the tiles' addresses are compared. Pictures of layers no longer listed are let go.
- **Under the list:** **+ Paint**, **+ Fill**, **+ Group** and **···**. The mockup's ⋯ is drawn as three middle dots, as the interface font has no ⋯. The menu holds:
  - **Add:** + Gradient, + Light and shade, + Edge highlight, + Material, and + Dirt, + Wear and + Rust with the realism channels;
  - **the chosen layer's actions:** Up, Down, Duplicate, Merge down, Group or Ungroup, the masks, Delete;
  - **Realism channels;**
  - **Presets:** Save style…, Save material…, Load preset….
- The chosen layer's settings, Bakes and "Why does this look wrong?" stay under the list.
- No keys or mouse actions are added ([0028](0028-ask-about-controls.md)).

## Consequences

- The list and the brush's settings both fit on a laptop screen.
- The rarer actions take one more click. A right-click menu on a layer row could give them back without the trip to ···; that would be a new mouse action, so it waits for the project lead.

## Amended 2026-10-30, room and smaller fixes

The layer list and the chosen layer's settings scroll separately: the list up to 45% of the panel (at least 120 points), the settings, bakes and "Why does this look wrong?" the rest; the tab itself no longer scrolls up and down. A new painted mask is chosen from the layers (the layer gained a mask that isn't smart), not by comparing the undo label.
