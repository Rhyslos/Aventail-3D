# 0218. Fixes from the texture painter's end-to-end reviews

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

At the project lead's request, two reviewers used the app from modelling to UVs to texture painting (a hand-painted crate, a realistic power cell) on 2026-10-29. Besides plain bugs, they found behaviours that needed a choice.

## Decision

Bugs fixed (no choice involved): a press over a panel started a stroke where the mouse moved to; Subdivide (any modifier that makes the shape afresh) put paint on the wrong polygons, as new polygons took the paint of old ones with the same number (a polygon now keeps its paint point for point only when its corner points are the same ones too); a flat ring round a small hole was straightened into a stretched strip (a quad that tapers more than 1.35 times isn't part of a strip); Straight out on a closed band slid it sideways (it goes out along the normals when the facings cancel out); the UV Editor's notes ran over each other.

Choices made:
- **Esc** closing a menu or pop-up in Paint mode only closes it; a second Esc leaves Paint mode.
- **Polygon fill** (4) always starts filling, not erasing, even straight after the eraser: nothing on screen showed it would erase. Erase is ticked in the Paint tab to fill with nothing.
- **The eraser on a fill or made layer** (gradient, light and shade, material) makes no empty layer: it says why, and how (a paint layer, the opacity, the mask).
- **Painting puts a modelling tool down:** entering Paint mode with Extrude, Inset, Bevel or another modelling tool chosen goes back to selecting (the gizmos stay).
- **Made layers** wait while one of their settings is dragged or typed (they were made again at every step of a slider, freezing the window for minutes), and say "Making the paint layers…" a frame before they are made. Packing shared paint says "Packing…" first too.
- **Bakes** show how far the map under way is, can be cancelled, and stop (and are forgotten) when another project is opened or a new one started.
- **The UV Editor** shows the channel chosen in Show in the views (roughness, metal, height as greys, glow on black), as the 3D view does.
- **The Paint layers list** comes before the chosen layer's settings, so choosing a layer no longer moves the list under the mouse; layers made from their settings show a cog, not the brush.
- **Layer colours** (fill, gradient ends, light and shade, material colours, glow) open the app's own colour picker, as the brush's colour does.
- **The shelf's panel tab** is called **Shelf**, not Brushes, so it isn't confused with its Brushes tab.
- **The mirror axis list** on the brush bar names the axes as the manual does: X (left and right), Y (front and back), Z (up and down).
- **File windows** count as never shown when they give nothing back within two seconds (was half a second, missed on a busy computer, so Export did nothing); a closed Export window says nothing was exported; Open starts in the folder of the last project when none is open.
- **Recover unsaved work?** isn't asked when the project was saved after its last backup copy (the app was closed without quitting after saving); those copies are thrown away.
- **The eraser** shows no realism channel rows (it takes every channel away).

## Consequences

- One review finding wasn't reproduced: undo and redo bringing back an extrude at a different distance after a Last edit change was undone. It stays on the list to look at with the project lead.
- A finding left for later: UV pieces left overlapping after modelling a painted shape aren't pointed out while painting.
