# 0308. Pens, strokes and alphas for sculpting (Phase 17a, milestone 3)

- **Status:** Accepted, built while the project lead was away; the plan's choices and the new ones below wait for review in [docs/plans/phase-17.md](../plans/phase-17.md#choices-to-review) (36 to 41 new)
- **Date:** 2026-11-18
- **Follows:** [0300](0300-sculpting-plan.md) (the sculpting plan), [0304](0304-sculpt-mode-and-layers.md) (Sculpt mode), [0306](0306-sculpt-brushes-for-forms.md) (the brushes for forms), [0184](0184-brush-feel.md) (the painter's pen and smoothing)

## Problem

Sculpting's third milestone: the brushes should feel like the paint brushes. **Pen pressure** changing size and strength through curves, **smoothing** (the lazy mouse, **D**), three kinds of stroke (**Freehand**, **Line** and **Drag a stamp**), and **alphas**, greyscale pictures a brush stamps or drags: a set made in code (stones, bark, scales, stitches, rivets, cracks, scallops) and your own pictures, on the shelf's **Alphas** tab. The plan sets these; building them needed a few choices it doesn't make.

## Options

1. **Where your own alphas are kept.** In each project (a file format change; an alpha added in one project is missing from the next); in a library of your own in the app's data folder, the same in every project, as your scripts are (decision 0303); or only while the app is open.
2. **Sharing with the painter's tips and the material graph's stamps.** The plan says alphas are shared with them. But the painter's tips are drawn in code (Round, Chalk, Bristle, Dry brush, Spatter), not pictures, and a graph's stamps are colour pictures with see-through parts, kept in one box. Make one shared picture store now (a change to the painter and the graph, their files and undo, in a sculpting milestone); or keep alphas apart for now, read through the same image reader with the same limits, and share them when a painter tip from a picture is built.
3. **How an alpha changes a dab.** Multiply the move each brush gives a point by the alpha under it (one rule for every brush); or give each brush its own way of using the alpha (more work, more to learn).
4. **Which way an alpha's top points.** Up the view (as seen when the stroke begins), turned by an angle; or along the stroke, turned by an angle (a switch); for a stamp, along the drag.
5. **Pressure while drawing a Line or dragging a stamp.** The pressure now, changing as the line is redrawn; or the pressure at the press, kept for the whole line or stamp.

## Decision

- **Pen pressure** changes the size and the strength, each with its own switch, least share and curve (Soft, Even, Firm), as Paint mode's pressure does (decision 0184). Both are on as standard, a light touch giving a tenth. A mouse sculpts at full size and strength. A pen through the system's touch is read in Sculpt mode too.
- **Smoothing** is Paint mode's lazy mouse with its own setting: **D** turns it on and off; the number is the string's length. The grabbing brushes don't use it (they hold the surface under the mouse).
- **Strokes:** **Freehand** (standard), **Line** (press, drag, let go: dabs along the straight screen line from the press to the mouse) and **Drag a stamp** (press where it goes; the drag's length on screen is its radius and its way turns its alpha; one dab). While a Line or stamp is dragged, every move takes back what it laid and lays it again, so it shows as you drag; on a heavy shape a line is laid as far as a frame's time allows and finished when you let go. Grab, Elastic grab and Snake hook always stroke freehand.
- **Alphas** (`sculpt::alpha`): one byte a pixel, at most 1024 pixels across or down; sampled between pixels, black outside the picture; centred in a square as wide as its longest side. **An alpha multiplies each point's move** by the alpha under it (option 3, the first way), laid flat on the plane the brush's polygons face, so every brush but the grabbing ones takes alphas with one rule.
- **The alpha's top** points up the view, turned by **Angle**; **Follow the stroke** points it along the stroke's way instead (option 4). A stamp's top points along its drag.
- **The set made in code** (`sculpt::alpha::Made`): Stones, Bark, Scales, Stitches, Rivets, Cracks and Scallops, 256 pixels each, made once when first wanted. No pictures are kept in the repository.
- **Your own alphas live in an alpha library** (option 1, the second way): an `alphas` folder in the app's data folder, beside the script library, one greyscale PNG an alpha (`fileio::alphas`). A picture brought in (the Alphas tab's **Add…** card; PNG, JPEG, TGA or BMP) is read as any image is, turned grey (how light a pixel is, times how solid: see-through is black), made smaller to fit 1024 pixels, and kept. Damaged and huge pictures, and pictures black everywhere, are refused in plain words. Right-click one of yours to take it out. No file format change.
- **Sharing with the painter and the graph waits** (option 2, the second way): alphas go through the same image reader and limits as stamps and stencils; a shared picture store comes with the first painter tip made from a picture.
- **Line and Drag a stamp keep the pressure of the press** (option 5, the second way): a line redrawn as you drag would otherwise change its strength under you.

## Consequences

- No new file format: alphas change only how strokes are made, and a stroke's result is kept as before.
- An alpha costs a little per point (a blended look-up); on the heavy test shape a stamp with a 1024-pixel alpha and about 200 000 points in reach is timed in the heavy sculpting check.
- Long Line strokes on a heavy shape lay again from the start on every mouse move; they stay within a frame's time and finish when you let go, so a very long, heavy line can lag behind the mouse while dragged.
- The painter's tips and the graph's stamps don't list sculpting alphas yet, and alphas don't list them; the plan records this as a choice to review.
- Scripts still see Sculpt mode as the Model mode.
