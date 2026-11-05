# 0239. UV selection, isolate, the editor's own selection, copying UVs

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 11. UVs could be selected by clicking, boxes, whole pieces and connected; not by what's wrong with them or what they're like, grown without crossing seams, or kept apart from the viewport's selection. Copies of a part modelled separately had to be laid out one by one.

## Decision

- **`uv::select`**: `overlapping` and `mirrored` (from `uv::warnings`), `stretched` (by `Stretch::off`, 1.5 times), `similar` (pieces with the same polygon and corner counts and UV and real areas within 2%), `grow` and `shrink` (across sides with the same UVs on both sides only), `shortest_path` (Dijkstra along edges joined in the texture, by their texture length), and `copy_layout` (pieces matched by walking out from a polygon of the same corner count, each turn tried, corner by corner; the copy placed with its box middle on the target's).
- **UV → Select in UVs** (no keys; the project lead chose menus for isolate and sync): the seven selections (one undo step each, shown in the Polygons mode, or the UV mode if in it; the path in the Edges mode), **Isolate the selected pieces**, **UV Editor follows the viewport's selection**, and **Copy UVs to similar pieces** (the piece holding the lowest-numbered selected polygon is the source; one `SetUvs` per object).
- **Isolate** is a UV Editor view setting: each shown shape is cut down to the pieces holding its selection (a copy of the mesh with the other polygons deleted, ids kept), worked out again when the shape or selection changes.
- **The editor's own selection** (`UvEditor::own`, an `OwnSelection` per object), as the plan suggested (simpler than a second selection in the document): while kept, the editor's clicks and boxes change it without undo steps; the editor's drawing, gizmo (`gizmo::uv_targets_with`), brushes, Turn & flip from the ring and Frame selected use it. Other UV commands use the document's selection; the manual says so.

## Consequences

- The editor's own selection isn't saved or undone; it lasts while the setting is off.
- Copy UVs stacks nothing: each copy stays where its piece was, so Pack or Align afterwards.
