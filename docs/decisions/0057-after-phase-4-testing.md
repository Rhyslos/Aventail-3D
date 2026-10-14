# 0057. Fixes and keys after Phase 4 testing

- **Status:** Accepted
- **Date:** 2026-10-14

## Problem

The project lead tested every Phase 4 milestone and collected what felt wrong. This record covers the first of the follow-up pull requests: the bugs, the key changes and the small behaviour changes. Closed Slit lines, symmetry for every edit, the drawn keyboard, spirals and the renames come in their own pull requests.

What he ran into:

1. After dissolving an edge, its red selected line stayed, and so did its cyan hard-edge mark. A dissolved corner of a cube was drawn with a wrong triangle and a gap.
2. Picking Inset after an extrude still showed the extrude's bend rows, as if they were Inset's.
3. A Bridge between two boxes looked smeared: its new edges along the corners were soft.
4. With Weld or Bridge in use, clicks didn't select and nothing lit up under the mouse.
5. He kept forgetting he was still in the Edges mode, and wanted one key to get back to selecting objects.
6. Connect on J was a reach; C bridged two rims of an open tube when he expected caps.
7. Weld's drag only circled the target; he wanted it to snap.
8. A bend set for one extrude carried on to the next, which then turned by surprise; the bend's sliders didn't match the rest of the panel.
9. Dissolve across a sharp corner is almost never wanted, but he didn't want it forbidden.
10. Double-clicking to select a loop was unreliable. With Move, Rotate or Scale on, the first click moves the gizmo to the clicked edge or point, right under the mouse, so the second click landed on the gizmo and was taken by it.

## Options

For the selection problem (4):

1. **A click always selects, in every tool; a drag is the tool's.** Simple to explain, but Loop cut (click cuts) and Slit (click adds a point) need clicks.
2. **Only Weld and Bridge change** (the tools he hit it in): a click selects, a drag that starts on a highlighted point or open edge is the tool's, a drag from empty space selects. Loop cut and Slit stay.

For the back key (5): one step at a time (tool, then mode), or **straight back to the Model mode**. For the bent polygons (1): forbid them, or **draw them properly**. For Dissolve (9): refuse, warn, or **warn with a way through**.

## Decision

- **Weld and Bridge: option 2.** They keep the hover highlight; the viewport gives them a drag only when it starts on something highlighted.
- **Tab steps straight back**, chosen by the project lead: it finishes a Slit line, puts a modelling tool away (Move, Rotate and Scale stay), closes the tool menu and lands in the Model mode. Not while typing (Tab moves between fields there), and not during a drag. egui's own Tab (moving the keyboard focus to the next button) is cancelled, so the keys keep working.
- **Connect moves to A**, **Fill hole gets J**, and **C always bridges**, all chosen by the project lead. Ctrl + A still selects all.
- **Weld's and Bridge's line snaps** to the point or edge it would join, ringed; what it snapped to is what is joined.
- **The bend Angle goes back to 0** after an extrude and when the tool changes; the finished extrude keeps its bend. Editing the last extrude's Angle no longer sets the next extrude's. The bend rows become number fields with arrows under a small Bend heading.
- **While a tool with settings is in use, the Attributes panel shows only that tool's settings**; the last edit shows when the same tool made it.
- **Gone elements aren't drawn**: the overlay draws only edges the shape still has, and Dissolve forgets the values (hard marks) of edges it took away. Two hard edges joined into one by dissolving the point between them stay hard.
- **Bridge and Fill hole mark their new sharp edges hard** (sharper than 30°, the same start as Hard edges by angle). Nothing is made soft. Old projects with bridges or filled holes show the crisper corners when opened, as edits are worked out again from the file.
- **Polygons are cut into triangles properly** (`mesh::triangulate`), brought forward from Phase 5's consistent triangulation: triangles as they are; quads 0–2 unless dented; flat polygons without dents as a fan; anything else laid flat on its best plane and cut along lines inside it, the shortest cut for up to 32 corners (which follows a bent polygon's fold), ear by ear above that. Drawing, picking and the selection tint use the same cut.
- **A double-click on a gizmo handle goes through to selecting**: a press on a handle that ends without a drag and is the second click of a double-click is left to the selection, so the loop is selected. A single click on a handle still does nothing, as before.
- **Dissolve warns** when it would take away an edge whose polygons meet more sharply than 30°. The question explains why and suggests Delete; **Dissolve** stays locked until "I know what I'm doing, proceed" is ticked; "Don't warn me again" switches it off, and Settings → Interface → Warnings switches it back on (remembered with the view options).

## Consequences

- Bridge and Weld feel like the drag tools: selecting never needs a tool change. Loop cut and Slit still use every click; if that bothers the project lead, revisit.
- A key for going back means modes are never a trap. Tab is no longer a way to move the keyboard focus between buttons outside of text fields.
- Triangulation now depends on where the points are. It is worked out again when a shape's structure changes, not while points are only being dragged, so a polygon bent by a drag keeps its earlier cut until the next edit. Export (Phase 5) will use the same function.
- Bridge and Fill hole results now carry hard marks; tests that count hard edges on their results need to expect them.
- The format of the project file doesn't change.
