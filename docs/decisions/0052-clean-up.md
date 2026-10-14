# 0052. Clean-up: Delete deletes, Backspace dissolves; Fill hole, Flip normals, Fix facing; file format version 10

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0028](0028-ask-about-controls.md) (ask about controls), [0037](0037-tool-menu-and-command-list.md) (the command list), [0043](0043-attribute-layers-and-the-builder.md) (the builder), [0051](0051-connect-bridge-weld.md)

## Problem

Modelling leaves things to tidy: elements to remove, holes to close, polygons facing the wrong way. Each needed controls, and "remove" needed two meanings: with a hole left behind, or without.

## Decision

1. **Delete and Dissolve** (the project lead: "Delete deletes, backspace dissolves").
   - In the Points, Edges and Polygons modes, **Delete** deletes the selected elements and every polygon using them, leaving a hole. Points no polygon uses any more go too.
   - **Backspace** dissolves them: they go but the surface stays closed.
     - An edge: the two polygons beside it merge into one.
     - A point: the polygons around it merge into one. A point between just two edges (on a straight line, say) simply goes from its polygons.
     - Polygons: each group of touching ones merges into one.
     - Afterwards, points left in the middle of a straight line by the dissolve go too, so no needless corners are left. Corners of a real turn stay.
     - What can't be dissolved without opening a hole (an edge or point on an open border, or polygons that wouldn't make one simple polygon) stays. If nothing could go, nothing is recorded and the status bar says why.
   - In the Model mode both keys still delete the selected objects, as before.
   - A merged polygon keeps the lowest id of those merged, and values come along through the builder.
2. **Fill hole** (tool menu, Mesh menu; no key, as the project lead chose for the clean-up commands). In the Edges or Polygons mode it closes each hole (a loop of open border edges) touching the selection with one polygon. With nothing selected, every hole of the selected objects. The new polygon takes its values from the polygon beside the hole, and faces the same way as its neighbours.
3. **Flip normals** (tool menu, Mesh menu; no key). In the Polygons mode it turns the selected polygons inside out, keeping each corner's values.
4. **Fix facing** (Mesh menu). It turns every polygon that faces the wrong way round, in each selected object:
   - in each piece, the polygons that disagree with the way most of their piece faces (walking across shared edges);
   - in a closed piece, the piece as a whole is made to face outward (its enclosed volume positive).
5. **Select flipped and Select similar facing** (Select menu). Both switch to the Polygons mode.
   - Select flipped selects the polygons Fix facing would turn, so they can be checked first, or flipped one by one.
   - Select similar facing adds every polygon facing within **30°** of a selected polygon. Planned as "select by facing"; working from the selection needs no direction to be typed in.
6. **The edits:** `Operation::DeleteElements`, `Dissolve`, `FillHoles` and `FlipPolygons` record ids and anchors, found again by position when a shape's points are renumbered; `FixFacing` works out what to turn from the shape as it is.
7. **File format version 10.** Version 9's operations, plus these five.

## Consequences

- Holes opened by Delete can be closed again with Fill hole, or with Bridge on two halves of a border.
- Dissolving edges or points can make n-gons. That is fine on flat areas; use Connect or Slit to cut them up again where needed.
- Fix facing can't decide which way an open sheet (a plane, say) should face: it makes the polygons agree with the majority, and Flip normals turns the whole sheet if needed.
- Shapes made by dissolving can't be undone by changing the shape's settings: like every edit, a dissolve is found again by anchors, and is skipped if what it removed is no longer there.
