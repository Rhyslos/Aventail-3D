# 0139. Spline editing: adding, joining, splitting and shaping curve points

- **Status:** Accepted
- **Date:** 2026-10-24
- **Builds on:** [0138](0138-curves-sweep-lathe.md) (curves), [0028](0028-ask-about-controls.md) (ask about controls), [0037](0037-tool-menu-and-command-list.md) (one command list), [0048](0048-ctrl-wheel-changes-a-tools-count.md) (Ctrl + wheel), [0051](0051-connect-bridge-weld.md) and [0128](0128-weld-polygons.md) (Connect and Weld; M welds at the middle), [0055](0055-basics.md) (Join objects), [0126](0126-rework-and-first-use.md) (grouped menus, the development file format), [0133](0133-key-audit.md) (Alt + drag moves the view), [0040](0040-bevel.md) (Bevel)
- **Replaces part of:** [0138](0138-curves-sweep-lathe.md) ("not yet: adding a point in the middle, joining two curves"; a point's smooth switch becomes a handle kind)

## Problem

Decision 0138 gave curves drawing, moving, deleting and a sharp/smooth switch. The project lead asked for splines "done well from the beginning" and chose (2026-10-24):

1. **Adding a point:** in the Points mode, a **double-click on the curve's line** adds a point there without changing the shape. A double-click on a point still switches it between sharp and smooth.
2. **Removing points:** Delete, as built, keeping the shape as near as reasonable.
3. **Joining, like meshes:** **Join objects** on curves joins them at their nearest ends; in the Points mode, with two end points selected, **M** welds them into one point at the middle and **A** (Connect) joins them with a straight stretch (on one curve's two ends, both close it).
4. **Carrying on:** Draw curve, clicking an **end of the selected curve**, carries on drawing from it (in front, from the start end), as one undo step.
5. **Extras:** Split, Reverse, Open/Close; **handle kinds** per point (Sharp, Smooth, Broken, Auto), set from the Attributes panel and the E menu, not Alt + drag (0133), the double-click still switching sharp and smooth; **rounding a corner** (a fillet) with a radius; **even spacing** (resampling) with a point count. All commands in the Mesh menu, the E menu in the Points mode and command search, greyed out with a reason when they don't apply.

Left to decide: the maths of each, what keeps the shape, ids, where the commands live in the menus and the E menu, how the fillet and even spacing take their numbers, the file format, and the awkward cases.

## Options

For the fillet's radius and even spacing's count:
1. **Fields in the Attributes panel (chosen):** a **Curve points** section in the Points mode, with the radius and the count beside buttons that run the commands; the menus and the E menu use the same values. No new mouse action.
2. A drag tool like Bevel (decision 0040), with Ctrl + wheel for a count: Bevel's drag measures an offset from edges of a mesh; a curve has no edges, so the drag would need a pull of its own (a new mouse action, which needs the project lead's say, 0028). Even spacing has no drag at all. Left for later if the fields feel slow.

For the Mesh menu:
1. **A Curve submenu in the Mesh menu (chosen),** beside Build, Cut and join, Remove, Booleans and Shading (0126's groups): one place for every curve command, greyed out with the reason when no curve is selected, so it's found where people look.
2. A Curve menu shown only for curves: menus that come and go are harder to learn, and 0126 keeps the menu bar fixed.

For the file format:
1. **Extend version 41 in place (chosen):** pull request #131, which brought version 41, isn't merged, so no released app reads it. Each point gains an optional kind; a file without one (the first version 41 files) takes it from the smooth switch, which is still written.
2. Version 42: needed only once 41 is in `main`.

## Decision

**Handle kinds.** A curve point has a kind (`PointKind`) in place of 0138's smooth switch:
- **Sharp:** no handles, a corner.
- **Smooth:** handles in one line; dragging one turns the other to match, keeping its length (as 0138's smooth points).
- **Broken:** each handle moves on its own, for a kink with curved sides.
- **Auto:** handles worked out from the neighbours, along the line from the point before to the point after, a third of the way to each (a Catmull-Rom curve's pull); an open curve's end points a third of the way to its one neighbour. They follow whenever a neighbour moves. Pulling an auto point's handle by hand makes it smooth, so the pull stays.

Set from the Attributes panel's **Handle type** buttons (the kind the selected points share is lit), the E menu's **Handle type** place (it opens the four, like the UV ring's Turn & flip, decision 0109), the Mesh menu and command search. A double-click on a point switches sharp to smooth and any other kind to sharp. Making a point without handles smooth or broken gives it auto-style handles to pull.

**Adding a point** splits the stretch at the place double-clicked (de Casteljau): the new point (smooth, its handles in line) and the shortened handles either side are set so the shape is exactly the same. On a straight stretch the new point is a sharp corner on the line. An auto neighbour becomes smooth so its handles stay. The double-click looks at the selected curves' lines within 6 screen points, after points and handle ends (which keep their double-click).

**Deleting points** keeps each gap's neighbours' handles pointing the same way and stretches them across the gap as the deleted stretches spanned it; deleting one smooth point gives back the curve it was split from exactly. Deleted points at an open curve's ends just go. **A curve keeps at least two points; a loop needs three** (closing a two-point curve, or welding a three-point curve's ends, is refused with the reason).

**Joining.** `mesh::curve::join` puts two open curves end to end in the same space: the first keeps its ids and runs as it did (turned round if joined at its start), the second gets new ids. A weld melts the two ends into one point at their middle (smooth if their handles meet in line, else broken); otherwise a straight stretch joins them, unless the ends are on one spot (within 0.001 cm), when they melt anyway. If the result's own two ends then meet on one spot, they melt and it closes; otherwise nothing closes.
- **M and A** in the Points mode with curve points selected: exactly two points, each an end of an open curve. On one curve, its two ends: M welds them and A joins them with a stretch, closing it. On two curves: the curve selected first stays, with its settings, and the other joins on and is deleted (`Command::JoinCurveEnds`). Anything else selected says why in the status bar.
- **Join objects** with only curves selected joins them into the one selected first: each time, the curve whose end is nearest one of the growing curve's ends joins on there (`Command::Join`). A closed curve has no ends: greyed out with the reason. With meshes selected too, Join works as before (curves with a solid join as their solid).

**Carrying on.** With Draw curve, the selected open curves' ends show as dots before anything is drawn; the first click on one starts from it (on the plane through it facing the view, as a new curve's first point would choose). Points drawn go after the last point, or in front of the first (handles turned round to match). Clicking the curve's other end closes it. Enter, a right click or another tool finishes; Esc cancels; Backspace never takes back the end itself. One undo step for the whole carrying on (`CurveEdit::Extend`). Clicking anywhere else starts a new curve as before.

**Split, Reverse, Open or close.**
- **Split curve** cuts at the selected points: each ends one piece and starts the next (an open curve's ends cut nothing). The curve keeps the first piece; each other becomes a new curve object beside it with the same settings (sweep or lathe), transform, materials and mirror; all the pieces are selected (`Command::SplitCurve`). A closed curve cut at one point opens there (its copy of the point gets a new id); at more, it falls into that many pieces. Each piece keeps its shape.
- **Reverse curve** turns curves round: an open curve's last point comes first; a closed curve keeps its first point. Reversing twice gives the same curve.
- **Open or close curve** closes open curves with a straight stretch, or opens closed ones.

**Round corners** turns sharp points with a neighbour on each side, not in a straight line, into a circular arc of the radius set under **Round corners** in the Attributes panel (10 cm to start with): two smooth points where the arc meets the legs (the first keeps the corner's id), with handles of a Bézier arc. The radius is cut back so the arc fits: on all of a leg, or half of it where the corner at its other end is rounded too. Other points selected are left alone; with none that can be rounded the command is greyed out with the reason.

**Even spacing** spreads `count` points evenly along the curve by its length (measured finely), the count set under **Even spacing** in the Attributes panel: as many as the curve has until it's changed (at least two, three for a loop, at most 4096). Each point's handles lie along the curve, a third of the way to its neighbours, so the shape is kept closely; on a straight stretch a point is a sharp corner with no handles, so straight lines stay straight. All new ids. It is a command, not a tool in use, so its count is a field, not Ctrl + wheel (0048).

**Ids** stay unique and are never reused (0138): new points take the next id. A curve that has made 65 536 points over its life refuses more, with the reason.

**Commands and menus.** Ten commands in the command list (0037): Split curve, Reverse curve, Open or close curve, the Handle type group with Sharp points, Smooth points, Broken handles and Auto handles, Round corners, Even spacing; none has a key. Reverse, Open or close and Even spacing work on whole selected curves in the Model and Points modes; the others on the selected points in the Points mode. **The E menu** in the Points mode with only curves selected shows a **curve ring**: Weld, Connect and Delete in their modelling-ring places, the rest round them. **The Mesh menu** has a **Curve** submenu with them all, each greyed out with its reason when it doesn't apply; **command search** lists them under Mesh. Every change is a command, so undo works; each command is one undo step. The Keys window lists the new mouse actions.

**Saving.** Version 41, extended in place: each point stores its kind (and its smooth switch, true for smooth and auto). Files saved before keep opening: a point marked smooth is smooth, a sharp one with handles is broken (its handles already moved on their own), a sharp one without is sharp. The first version 41 sample stays as `sample-v41-smooth-only.avtl`; the new sample holds a curve with every kind.

## Consequences

- Curves can be reshaped point by point without redrawing; a sweep or lathe follows every change.
- With a curve selected after drawing it, Draw curve's first click on one of its ends carries it on; to start a separate curve there, deselect it first (the dots on its ends show the difference).
- Rounding and even spacing have no live preview; if they feel slow to tune, a drag tool can come later (asking the project lead for its controls, 0028).
- Not yet: curves in the UV Editor, joining a curve to a mesh's edges, and a numeric handle length field.
