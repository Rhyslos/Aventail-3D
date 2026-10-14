# 0051. Connect, Bridge and Weld: J, C (or drag) and M (or Ctrl + M drag); file format version 9

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0028](0028-ask-about-controls.md) (ask about controls), [0043](0043-attribute-layers-and-the-builder.md) (the builder), [0048](0048-ctrl-wheel-changes-a-tools-count.md) (Ctrl + wheel)

## Problem

Three tools for joining parts of a shape: Connect, Bridge and Weld. Each needed controls and a precise meaning.

## Decision

1. **Connect, `J`** (the project lead's choice; Blender joins points with J too).
   - **In the Points mode**, it joins the selected points across each polygon holding two or more of them. If there are three or more, they are joined all the way round in the polygon's order.
   - **In the Edges mode**, it cuts each selected edge in the middle (or **Cuts** times, evenly) and joins the new points across each polygon holding two or more of them. A ring of edges gets a new loop.
   - Polygons beside a cut edge take its new points as corners, so no gap opens.
   - It applies at once. Cuts shows as the Last edit.
2. **Bridge, `C`**, both ways, as the project lead asked ("I prefer drag, but it's inconvenient when you want to bridge many edges"):
   - **With something to bridge selected, C bridges at once:**
     - in the Edges mode, two open borders (chains of edges with a polygon on one side only);
     - in the Polygons mode, two polygons, which are removed so a tube joins their borders.
   - **Otherwise C makes Bridge the tool:** drag from one open edge to another, and a polygon joins them.
   - **Settings:** Segments (rows; Ctrl + wheel while the tool is in use) and Twist (turns a band between closed loops by whole steps).
   - **The band:** borders of the same length get quads, lined up so matching points are closest. Different lengths get a zip of triangles.
   - **Direction:** the band always runs against the polygons already beside each border, so nothing faces the wrong way.
3. **Weld, `M`** (D, the project lead's first wish, is Extrude):
   - M melts the selected points into one at their middle.
   - **Ctrl + M** makes Weld the tool: drag a point onto another and it melts there. If the dragged point is selected, the whole selection melts there.
   - The point that stays is the target, or the lowest id.
   - Polygons left with fewer than three corners go, and points left with no polygon go too. Edges that went to the kept point keep their values.
   - Ctrl + D stays free for Duplicate.
4. **The edits.** `Operation::Connect`, `Bridge` and `Weld` record ids and anchors, found again by position when a shape's points are renumbered. A weld's target is the point found nearest where it was.
5. **File format version 9.** Version 8's operations, plus these three. Edges must join two different points; counts are brought into range.

## Consequences

- Holes can be closed with Bridge on two halves of a border, or with Fill hole (next milestone). Separate pieces can be joined by bridging their open borders.
- Welding can make shapes that aren't closed solids (a cube's top welded to one point is fine; two far corners welded fold a polygon). The mesh check in the Attributes panel says so.
- Bridging borders of different lengths makes triangles; bridging matching ones keeps quads, which suit loop cuts later.
