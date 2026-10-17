# 0093. Snap to and Align

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0046](0046-our-own-names.md) (Quick Align is called Snap to here), [0068](0068-pivot-editing-and-scene-units.md) (Pivot to object, picked with a click); DESIGN.md W7 and Phase 6

## Problem

Putting a part exactly against another (a lid on a box, a pipe on an outlet, crates on one floor) means dragging and typing numbers. 3ds Max has Quick Align (pivot onto pivot) and Align (sides or middles along chosen axes); the wishlist's W7 asks for them, under our own names.

## Options

- **How the target is chosen:** **a click on it in the viewport, after choosing the tool** (chosen, as Max and our Pivot to object); a list of objects, which is slower with many parts.
- **Align's settings:**
  - **a window with the axes, the point of the moving objects and the point of the target (Lowest, Middle, Pivot, Highest), and Also turn them to match; shown live, kept with Align, taken back with Cancel** (chosen, as Max's dialog: the live result is how you check it);
  - fixed commands (Align tops, Align bottoms…), one menu entry each, which multiplies entries and can't combine axes.
- **Where the window sits:** **the bottom left of the viewport** (chosen), so what moves stays in view; the middle, as other windows, covered it.
- **Which axes:** **the world's** (chosen): Z is up, which is what "stand it on top" means; object axes can come later.
- **Keys:** the project lead was asleep; **no keys** for now, the recommended choice ([0028](0028-ask-about-controls.md)). They are in the Tools menu beside the pivot commands. The Model mode's tool menu has no free place.

## Decision

1. **Picking a target** is one mechanism for all three uses: `TargetUse { Pivot, SnapTo, Align }` replaces Pivot to object's own flag (`Workbench::target_pick`, `take_target_click`). Esc cancels, and the pointer is a cross while waiting.
2. **Snap to** (`Workbench::snap_to`): each selected object (not the target, and not one holding the target) moves so its pivot sits on the target's pivot. It is one `SetTransforms` step, "Snap to Sphere".
3. **Align** (`workbench::aligning`):
   - `AlignSettings` (axes, `from`, `to`: `AlignPoint`, `turn`) is remembered for the session.
   - While the window is open, every change replaces the shown step (`Document::abandon` of its merge key). Turning first, then moving, both merged into one step, "Align to Sphere".
   - Points come from each object's world box (`Scene::bounds_of`: children and live mirror included) or its pivot.
   - Doing something else meanwhile keeps what was shown and closes Align.
4. `dialog::Dialog::anchor` places a window elsewhere than the middle.

## Consequences

- Parts can be put together exactly without typing numbers.
- Nothing new in the scene or the file: both are object moves.
- Align for points, edges and polygons (flattening a selection to a plane, lining up points) is left for later.
