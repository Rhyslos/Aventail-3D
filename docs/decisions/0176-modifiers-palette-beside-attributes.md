# 0176. The modifiers palette sits beside Attributes

- **Status:** Accepted (the project lead's choice)
- **Date:** 2026-10-28

## Problem

The modifiers palette was always drawn on the viewport's right (decision 0091). In the UV Edit layout the UV Editor sits right of the viewport, so the palette ended up between the two views. The project lead found that "really annoying": it splits the views and sits far from Attributes, where a modifier's settings appear.

## Decision

**Not in the UV mode.** Nothing on the palette works on UVs, so it is hidden while the UV mode is on (the project lead's second thought, the same night), and the views get the room.

In every other mode, the palette sits on the right of whichever view is beside the Attributes panel: the viewport in the standard layout, the UV Editor in UV Edit. If the person drags the panels around, it follows: it goes on the view whose right edge is nearest Attributes (`layout::modifiers_palette_host`). With the UV Editor hidden, or behind another tab, the viewport carries it as before.

## Consequences

- In UV Edit, the viewport and the UV Editor sit side by side with nothing between them; the palette appears again, beside Attributes, in another mode.
- The palette sits under the UV Editor's tab bar there, so it starts a little lower than on the viewport, whose tab bar is hidden.
- The manual's "on the right of the viewport" stays true in the standard layout; the interface tour and the concepts page now say "beside the Attributes panel".
