# 0109. Pins, unfolding one way, and unwrap methods

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0099](0099-unwrap-relax-straighten.md) (unwrap, relax and straighten), [0108](0108-uv-powerhouse-and-baking.md) (the Phase 8 plan), [0037](0037-tool-menu-and-command-list.md) (the tool menu), [0028](0028-ask-about-controls.md) (ask about controls)

## Problem

Phase 8's first milestone: give the user control over how a piece is laid flat.

- **Pins:** hold chosen UVs still while the rest is laid out around them (every serious UV tool has them).
- **Unfolding one way:** move the UVs only across, or only up, keeping the other direction as it was. Useful for evening out a strip along its length without touching its height.
- **More ways to lay a piece flat** beside least squares conformal maps (LSCM), with the best picked for each piece.

## Options

- **Controls** (asked first, decision 0028). The project lead chose:
  - Pin and Unpin in the E menu and the UV menu, no key.
  - The E ring's four turn and flip tools grouped into one place, **Turn & flip**, to make room. The other new places are Pin, Unpin and a **Stitch & cut** group.
  - Stitch & cut holds two tools that arrive with milestone 8. Their keys in the UV Editor were chosen now: **X** cuts and **C** stitches. In the viewport, C bridges; the project lead knows they differ, and C reads as "join" to him.
- **How a group works in the ring:**
  - **choosing it opens its tools on the same ring, side by side around the group's place**, the middle one where the group was (chosen). The ring keeps its size and the mouse is already near them. Esc closes it; the next E opens the whole ring again. The quad menu lists a group's tools in its place.
  - a second, smaller ring: a new look to learn, and a new size, against the rule that buttons never change size.
- **What a pin is:** a flag on polygon corners (the `uv_pin` corner layer).
  - **Pinning a polygon or point pins the corners sharing its spot in the texture** (as moving UVs does). A pinned corner is held by anything touching its spot.
  - The layer is removed when nothing is pinned.
  - Pins are recorded edits (`SetUvPins`), so they undo, replay by anchors and save.
- **What pins do:**
  - **Unwrap, two or more pins apart:** the piece is solved around them and keeps its place, size and turn. It isn't turned, scaled or moved afterwards, since the pins say where it goes. A piece that lay mirrored is solved mirrored back, then turned over again, so it stays mirrored (laying flat never turns a piece over by itself).
  - **Unwrap, one pin:** the piece is laid out as usual, then moved so the pin is back.
  - **Relax:** pinned corners don't move.
  - **Straighten:** a straight grid can't bend to meet several pins, so the piece is moved so its first pin stays put.
  - Strips aren't laid out as straight grids when pinned: the pins win.
- **Unfolding one way:**
  - **the unwrap as usual, fitted onto the old layout (moved, turned and scaled to lie over it as well as it can), then each point given back its old V (across only) or old U (up only)** (chosen). It needs old UVs; without them it is a plain unwrap.
  - solving with one coordinate held: the same result for a strip, but harder to explain.
- **The methods:**
  - **Keep shapes:** LSCM, as before, and still the standard. It is fast, and live unwrapping as seams change uses the chosen method.
  - **Keep shapes, smoother:** linear angle-based flattening (Zayer, Rössl and Seidel 2007). The angles are evened out over the whole piece: each triangle's add up to a half turn, those round each inner point to a full turn, and the sides round each point agree. They stay as near to the real angles as can be, weighted by 1/angle². This is solved as one sparse system with the existing conjugate gradient solver, then laid out by LSCM from those angles. Curved pieces drift less in size.
  - **Keep sizes:** as rigid as possible (Liu et al. 2008). It starts from LSCM; then, twelve times, each triangle gets its best turn and the whole piece is solved again with cotangent weights. Pixels come out the same size all over.
  - **Automatic:** all three are tried for each piece, and the one with the lowest score is kept. The score is area-weighted, and adds up three things:
    - how much each triangle stretches one way more than the other: |ln(σ₁/σ₂)|;
    - how far its size is from the piece's average: |ln(σ₁σ₂/average)|;
    - a heavy price for a triangle turned over.

    A later method must be clearly better (by 1%) to win, so near-ties always go the same way (keep shapes, then smoother, then keep sizes). Nothing is random.
  - ABF++ (the non-linear method): better on very curved pieces, but much more code to keep right. The linear version gets most of the gain.
- **Where the method is chosen:**
  - **UV → Unwrap method, one choice for the app, remembered** (chosen).
  - per piece: that's milestone 4 (per-piece settings).

## Decision

1. `mesh::attributes::UV_PIN`, `mesh::uvs::pins` and `set_pins`.
2. `uv::methods`:
   - `UnwrapMethod`;
   - `lay_flat`, `real_shapes`, `conformal_shapes`, `angle_based_shapes`, `as_rigid_as_possible` and `distortion`.
3. `uv::unwrap`:
   - `UnwrapOptions { strips, method, only }`, `UnfoldAxis` and `unwrap_with`. `unwrap` is the plain case.
   - Pins in unwrap, relax and straighten.
4. `scene::Operation`:
   - `Unwrap` gains `method` and `only`;
   - new `SetUvPins { faces, points, pin, anchors }`, which is never mirrored and doesn't mend.
5. File format 34: `OperationV34` stores pins, and unwraps that aren't plain (plain ones are stored as version 30 stored them).
6. `ui`:
   - `CommandId::UvPin`, `UvUnpin`, `UvTurnFlip`, `UvStitchCut`, `UvStitch` and `UvCut`, with `members()` and `group_menu`. `ToolMenu::open_group` handles groups, and six new icons go with them.
   - `UvSettings::unwrap_method` (`UnwrapWay`).
   - In the UV menu: Unwrap method, Unfold across only, Unfold up only, Pin and Unpin.
   - Pinned UVs are drawn as magenta dots in the UV Editor in every mode.

## Consequences

- No new keys now; X and C for Cut and Stitch are taken in the UV Editor for milestone 8.
- Keep sizes takes about six times as long as keep shapes, and Automatic the three together; a piece of 3,000 points takes 0.7 s for all three. Big pieces should be split with seams, as before.
- Milestone 4's per-piece settings will store a method per piece, over this app-wide choice.
