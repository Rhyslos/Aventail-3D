# 0064. Automatic hard edges: shade by angle after every change

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0053](0053-hard-and-soft-edges.md) (hard and soft edges); replaces the per-object choice in [0063](0063-wireframe.md)

## Problem

Edits leave edges marked hard where a shape is meant to be round. Every ring of a bent extrude comes out hard, so a pipe looks faceted. A plain extrude keeps the old cap rim hard, leaving a seam. Fixing this by hand after every edit is tedious.

Decision 0063 first recorded a per-object setting. The project lead then explained what they meant: "in the display menu, to have a checkbox… on by default. What it will do is check the model every time there is a change, and apply hard edges based on the angle… right below the 'Hard edges' checkbox".

## Options

1. **Record a Hard edges by angle edit after every change:** this clutters the edit list and undo, fights the user's own edits, and is slow on heavy shapes.
2. **Shade by angle while drawing** (chosen): the shape and its edits stay as they are, and only the shading follows the angle.

## Decision

1. **The switch:** **Automatic hard edges** sits in the viewport's Display menu, right below Hard edges. It is on as standard and remembered (`ViewOptions::hard_edges_by_hand`, false as standard).
2. **What it does:** while it is on, every shape is shaded by angle, whatever its edges are marked. Where two polygons meet more sharply than **30°** (`AUTO_HARD_ANGLE`, the same as Hard edges by angle's start), the shading breaks; everywhere else it flows. Edges shared by more than two polygons break too; polygons with no area don't.
3. **How it works:**
   - `mesh::smooth_groups_with` / `corner_shading_with` take a `Hardness`: `Marked` or `ByAngle(degrees)`.
   - The renderer builds geometry with it (`MeshGeometry::shaded`).
   - Moving points updates only what moved, unless an edge around them turns sharp or soft (`can_follow`, using `mesh::angle_hardness_changed`); then that shape is built afresh.
   - Switching the setting rebuilds every shape.
4. **The cyan Hard edges guides** follow the same rule while it is on (`hardness::sharp_edges`).
5. **Edges set by hand are kept** and shade as before once the switch is off. While it is on, Hard edges, Soft edges and Hard edges by angle say so in the status bar.

## Consequences

- Extrudes, bends and cuts look clean straight away, with no extra edits or undo steps.
- The shading no longer always matches the marked edges. Export (Phase 5) must decide which to write: most likely the shading as seen. This is to be settled with the project lead when export is built.
- Low-poly curves whose sides meet at more than 30° look faceted while it is on. To shade them, switch it off and soften those edges by hand.
- Heavy shapes keep their quick updates while points move. Only a move that changes an edge's sharpness rebuilds that shape.
