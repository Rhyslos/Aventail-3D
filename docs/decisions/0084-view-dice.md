# 0084. The view dice and snapping while orbiting

- **Status:** Accepted
- **Date:** 2026-10-16
- **Replaces:** the clickable corner axes of [0066](0066-clickable-axes.md)

## Problem

The project lead asked for the corner axes to become a **dice**: click its side to look from there, with Ctrl + click for the other side, and an easy switch to make those views orthographic. While testing he also kept trying to land on straight views by orbiting, so he asked for snapping while orbiting.

## Options

- **What can be clicked (asked):**
  - faces only, six bigger targets;
  - **faces, edges and corners**, 26 views as on 3ds Max's view cube (chosen: "I used it a lot in 3ds Max"). Cinema 4D has no clickable cube; its corner axes only show the directions.
- **Where it sits (asked):**
  - top right, beside the arrows;
  - **bottom left, replacing the X/Y/Z arrows** (chosen), its faces in the axis colours.
- **The orthographic switch (asked):**
  - right-click the dice;
  - **the viewport's Display menu** (chosen);
  - Settings only.
- **Orbiting away from a flat dice view (asked):**
  - stay flat;
  - **back to perspective** (chosen), like Blender's auto perspective.
- **Snapping (asked):**
  - **hold Ctrl while orbiting to snap to the nearest of the 26 dice views** (chosen; Ctrl is free during an orbit in every navigation scheme, while Alt and Shift start orbits or pans in some);
  - Ctrl for 15° steps;
  - a gentle pull with no key.

## Decision

1. **Sides** (`tools::dice`): the 26 directions with X, Y and Z each -1, 0 or 1. The front is -Y, as the Front view sees it.
   - `nearest(forward)` gives the side a view most nearly looks from.
   - `name` and `face_label` give the words for the dice, the tooltip and accessibility.
   - `Navigator::looking_from(side)` turns the view round the pivot at the same distance.
   - `Navigator::snapped()` turns it to the nearest side.
2. **The dice** (`ui::viewport::dice`): a cube drawn flat in the bottom-left corner, turned as the camera is, its faces tinted in the axis colours and named.
   - Each visible face is cut into a 3 × 3 grid: the middle is the face, the strips the edges, the squares the corners. The middle is the biggest target (0.55 of the half-width each side).
   - The part under the mouse lights up on every face it shows on, with a tooltip naming the view.
   - A click glides there; Ctrl + click goes to the opposite side.
   - Looking straight at a face, four small arrows beside the dice lead to the faces around it (they can't be seen edge-on), as on 3ds Max's cube.
3. **Snapping:** with Ctrl held during an orbit, the view keeps orbiting unseen (`View::free`) and shows that orbit snapped to the nearest side. Let go of Ctrl and the orbit carries on from the snapped view.
4. **Display → Switch to orthographic when clicking the dice** (`ViewOptions::flat_from_dice`, off as standard, remembered).
   - With it on, a dice click or a snap makes a perspective view orthographic and marks it (`View::auto_flat`).
   - Orbiting freely turns a marked view back to perspective.
   - A view made flat with P is not marked, so it stays flat.
5. The Keys window lists the dice and Ctrl while orbiting.

## Consequences

- The X/Y/Z arrows and their click behaviour are gone; the dice's colours and names do their job.
- 3ds Max users get the cube they know. The arrows round a face-on dice keep every view one click away, even from straight above.
- Ctrl during an orbit is now taken; it did nothing before.
