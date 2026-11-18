# 0306. The brush set for forms (Phase 17a, milestone 2)

- **Status:** Accepted, built while the project lead was away; the plan's choices and the new ones below wait for review in [docs/plans/phase-17.md](../plans/phase-17.md#choices-to-review) (31 to 35 new)
- **Date:** 2026-11-18
- **Follows:** [0300](0300-sculpting-plan.md) (the sculpting plan), [0304](0304-sculpt-mode-and-layers.md) (Sculpt mode, Draw and Smooth), [0037](0037-tool-menu-and-command-list.md) (the tool menu), [0039](0039-drags-follow-the-pull.md) (drags follow the pull)

## Problem

Sculpting's second milestone: the brushes a stylised artist builds forms with, beyond Draw and Smooth: **Clay strips, Clay, Inflate, Crease, Pinch, Flatten, Scrape, Fill, Grab, Elastic grab** and **Snake hook**; the shelf's Brushes tab with pictures and what each does; the tool menu (**E**) holding the brushes; keys **2** to **7**. The plan sets the brushes and keys; building them needed a few choices it doesn't make.

## Options

1. **What Ctrl does to the grabbing brushes** (Grab, Elastic grab, Snake hook). Ctrl inverts every other brush; a grab has no opposite. Nothing; or something of its own (Grab moving only along the surface's facing, say).
2. **Picking a brush in the floating brush bar.** Thirteen buttons (they'd cover a third of the view); a drop-down; or buttons for the seven with keys.
3. **The tool menu's places.** The brushes one to a place in the shelf's order (thirteen of fifteen places, two kept for Mask and Trim); or groups (Clay strips and Clay, Flatten with Scrape and Fill, the grabs), leaving room for every brush to come but taking two clicks.
4. **How far Elastic grab reaches and what its rubber is like.** The whole shape (the published method's own reach: slow on millions of points); or four times the brush's radius, fading to nothing over its outer half. Rubber that keeps its volume (Poisson's ratio one half) or a springier one.
5. **The brushes' pictures.** Small renders of a sculpted ball (pictures to keep and remake); or drawings in code of the surface seen from the side, before (dashed) and after, as every other icon in the app is drawn.

## Decision

- **The brushes** (`sculpt::forms`, plain functions of a point; `LiveSculpt::dab` gathers and applies them on every core):
  - **Clay strips** lays clay up to a level above the plane under the brush (the points in reach, each counted by the brush's strength there), raising only points below it, so hollows fill first; its square turns with the stroke. **Clay** is the same, round. Points more than the brush's radius behind the plane (the far side of a thin part) stay.
  - **Inflate** pushes each point along its own facing. **Crease** cuts in along the brush's facing and pinches its sides towards the brush's middle line; Ctrl makes a ridge. **Pinch** pulls along the surface towards that line; Ctrl pushes apart.
  - **Flatten** moves points towards the plane under the brush; **Scrape** only those above it, **Fill** only those below. Ctrl: Flatten pushes away from the plane, and Scrape and Fill swap.
  - **Grab** takes the points in reach where the stroke starts and puts each at where it was plus the brush's whole move since, by the falloff (not added up dab by dab, so it is exact). **Snake hook** gathers afresh around the place it holds at every dab and carries it on, so a horn can be pulled far past the brush. **Elastic grab** moves the shape by a regularised Kelvinlet (de Goes and James, 2017; our own code) dab by dab from where the points are, so each small bend keeps the volume.
  - A grabbing brush follows the mouse on a plane through the place it holds, facing the view, so it follows the pull in every view, flat ones too (decision 0039).
- **Ctrl does nothing to the grabbing brushes** (option 1, the first way).
- **The brush bar shows the brush as a drop-down**; the Sculpt tab keeps a button per brush (option 2, the second way).
- **The tool menu (E) in Sculpt mode holds the brushes one to a place** (`commands::SCULPT_RING`, `CommandId::SculptBrush`), building ones at the top right, levelling at the bottom, grabbing at the left, Smooth beside Draw; two places kept for Mask and Trim (option 3, the first way). Command search finds them under Sculpt; outside Sculpt mode they say where they work.
- **Elastic grab reaches four times the brush's radius**, fading over the outer half, and its rubber keeps its volume (option 4, the second way, with Poisson's ratio one half).
- **The pictures are drawn in code**: the surface from the side, dashed before and solid after, with an arrow where the brush pulls (`icons::Icon::SculptBrush`), on the shelf's cards (the paint shelf's card and scrolling row) and in the tool menu (option 5, the second way).
- **Keys** (the plan's): **2** Clay strips, **3** Inflate, **4** Crease, **5** Flatten, **6** Grab, **7** Snake hook; **8** and **9** stay kept for Trim and Mask. The rest are on the shelf and in **E**.
- **Standard settings:** each brush keeps its own size and strength; the grabbing ones start at full strength, Crease with the Sharp falloff.

## Consequences

- No new file format: brushes change only how strokes are made, and a stroke's result is kept as before.
- Elastic grab works on every point within four brush radii at every dab: on the 3-million-polygon test shape with a big brush that is most of the shape, so it is the slowest brush (a frame of about a fifth of a second with a million points in reach on two cores). Grab, Snake hook and Clay strips are timed in the heavy sculpting check. Speed work for every brush is milestone 17a.6.
- Snake hook stretches polygons as it pulls (there is no dynamic topology, plan choice 6); Remesh for sculpting (17a.5) evens them out again.
- Scripts still see Sculpt mode as the Model mode.
