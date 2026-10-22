# 0126. The rework: what changes for first use

- **Status:** Accepted
- **Date:** 2026-10-22
- **Builds on:** [0121](0121-rework-before-the-rest-of-phase-8.md) (the rework comes first), [0028](0028-ask-about-controls.md) (ask about controls)
- **Replaces part of:** [0064](0064-automatic-hard-edges.md) (Automatic hard edges as one switch for the whole app), [0068](0068-pivot-editing-and-scene-units.md) (the Object mode for pivots), [0094](0094-quad-menu.md) (the quad menu off as standard), [0054](0054-symmetry.md) (the mirror in world space)

## Problem

A review of the whole app (decision 0121) found that it works but is hard to start with: the selection is hard to see, some labels mean two things, commands are hard to find, there are six modes where people expect five, the edit history asks questions ("Keep editable?") nobody needs to answer, hard edges are set for the whole app instead of per object, symmetry stays in the world when the object moves, and new settings each needed a new file version. The project lead asked for the fixes "in one go" (2026-10-22), in one pull request.

## Decision

1. **The selection is drawn last, thickest and brightest**, with a **High contrast** setting, and **Select only visible** is on as standard.
2. **One meaning per label** ("Delete objects" and "Delete points, edges or polygons", "Mark hard edges"), and **"pieces"** instead of "islands" in the interface.
3. **Keys chosen by the project lead:** **Space** opens command search; **Edit pivot** has no key; **Alt + N** is Don't save (Ctrl + D is what Maya users press to duplicate); **Esc** cancels a slit and closes the Keys window; E doesn't open the tool ring over a window. Every other key change waits for the key audit document.
4. **Finding things:** command search, the right-click (quad) menu on as standard, Help → User manual and Words used in the app, View → Show and hide, test tools in Help → Developer, and no "arrives later" placeholders.
5. **Five modes:** Model, Points, Edges, Polygons, UV. The Object mode becomes an **Edit pivot** switch in the Model mode.
6. **One verb, "Bake into shape"**, replaces Make editable and Freeze; **Keep editable** goes. Drags of the same points in a row are one edit. The Edit layers tab shows only when an object has more than one layer, or from the Window menu.
7. **Smoothing belongs to each object** (issue #114): by angle (30° as standard) or by marked edges only. Marking edges on an object smoothed by angle first keeps its look as marks, then switches the object to its marks, in one undo step.
8. **Symmetry belongs to the object:** the mirror is at its pivot and moves and turns with it. **Apply mirror** makes the mirrored half part of the shape and turns symmetry off, as Blender's Apply does.
9. **Menus:** the Mesh menu in groups; no command both in Edit and in Select; File keeps to files; the UV menu also in the UV Editor, and a **UV settings** panel beside Attributes (issue #115). Rotation reads X, Y, Z.
10. **One development file format until the alpha:** version 39 keeps version 38's model in CBOR (a self-describing encoding, `ciborium`, Apache-2.0), so new settings get a standard value and need no new version. Proper versioning starts again at the alpha. Older files still open.

## Consequences

- The manual's pages on pivots, hard edges, symmetry, the edit list, the quad menu and the menus are rewritten; command search gets a page.
- Projects saved now open only in this version of the app or later.
- Phase 9's remaining milestones (labels on the palette, the key audit's changes, the safety net, the tidy-up) continue on later branches; the plan is in `docs/plans/phase-9.md`.
