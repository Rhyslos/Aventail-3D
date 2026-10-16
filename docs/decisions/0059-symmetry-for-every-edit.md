# 0059. Symmetry for every edit, and the Symmetry menu

- **Status:** Accepted; replaced for new work by [0088](0088-live-mirror.md) (the live mirror)
- **Date:** 2026-10-14

## Problem

Symmetry ([decision 0054](0054-symmetry.md)) mirrored only moving, rotating and scaling. Testing it, the project lead put a loop cut on one side and nothing happened on the other. He asked for every modelling edit to be mirrored. He also asked for a way to choose where the mirror sits: an arrow beside the palette button opening symmetry settings, with the X, Y and Z switches moved there from the Attributes panel, a choice of the object's pivot, its middle or the world centre, and offsets along X, Y and Z.

## Options

How a mirrored edit is recorded:

1. **Two separate edits**, the edit and its mirror, in one undo step. No new kind of edit, but the Last edit panel would show and change only the mirrored copy.
2. **The selection grown by its mirror** before the edit is made. Right for edits on a selection (extrude, delete), but wrong for edits with a direction or a line (a loop cut's position, a slit's line, a bend's direction, a weld's target).
3. **One mirrored edit**: the edit as made, with the mirror planes. When the shape is worked out, the edit's mirror is found on the shape just before it. Edits on a selection are made once on both sides together; the others are made, then made again as a mirrored copy. The Last edit changes the one edit, so both sides follow.

Where the mirror goes through is a setting of the view (like symmetry itself), but each edit needs its own copy, as later changes of the setting mustn't move earlier edits.

## Decision

- **Option 3:** `Operation::Mirrored { op, mirror }` (`scene::mirroring`).
  - **Together** (the selection and its mirror, made once): extrude, inset, edge extrude, bevel, delete, dissolve, fill hole (with a selection), flip, hard and soft edges.
  - **As copies**: loop cut, slit, connect, bridge, weld and bent extrudes. A copy that would be the edit itself is left out: a loop cut whose ring holds the mirrored edge, or a selection, line or target that mirrors onto itself. A bend's direction is turned to bend the mirrored way.
  - Mirrors are found by position (`mesh::symmetry::Partners`, within 0.01 cm), and also carry mirrored anchors, so an edit holds when the shape's structure changes underneath, like any edit.
  - Not mirrored: moves (mirrored while dragging, as before), whole-shape edits (Fix facing, Hard edges by angle, Fill every hole), and whole-object commands.
- **Each edit keeps its mirror's middle** (`Mirror { axes, centre }`, in the shape's own space), worked out when the edit is made: the pivot (the origin), the box's middle, or the world centre seen from the object, moved by the offset. The planes always lie along the object's own axes. Moving, rotating and scaling use the same middle.
- **Weld the middle now and Don't cross the middle**, asked for by the project lead on testing the menu.
  - **Weld** was first automatic, after every mirrored edit. Testing showed it can't be undone by moving back: points once joined are one point. The project lead chose to make it a **button that welds once, when clicked** (`Operation::WeldMiddle`: the mirror planes and the distance, an edit like any other that Ctrl + Z takes back; `weld_middle` puts points within the distance onto the planes and welds points meeting there into one). Nothing welds on its own, and points near the middle aren't held there. A **live weld** (joined only while close, parting when moved away) comes with the mirror generator in Phase 7.
  - **Don't cross the middle** (clip) is kept with each mirrored edit: `keep_sides` puts any point pushed across a plane back on it, and a move's offset is limited so no selected point crosses (`move_limits`). Turning and scaling aren't clipped. Off as standard.
- **The Symmetry menu**, placed as the project lead asked: a small arrow right of the palette's Symmetry button, opening On, Mirror across X/Y/Z, Mirror through (Object's pivot, standard; Object's middle; World centre), Weld the middle now with its distance, Don't cross the middle, and Move the mirror by X/Y/Z. The Attributes panel's Symmetry section goes. All of it is remembered with the view options.
- **File format version 14**: version 13's edits, plus mirrored edits (an edit of version 13, the axes, the middle and the clip switch), and the middle welded once (`WeldMiddle`: the axes, the middle and the distance). Mirrored edits don't nest.

## Consequences

- Modelling one half now works for the whole toolset, and the Last edit panel changes both halves at once.
- An edit's name ends in "mirrored", so the undo list and the Last edit say so.
- Edits crossing the middle in odd ways (part of a selection mirroring onto itself, part not) may be made in ways that don't look as expected; the manual says to switch symmetry off for those.
- Mirroring separate objects onto each other (a left and a right object) isn't covered: mirrors are found within one shape.
- Projects saved now need this version or newer to open.
