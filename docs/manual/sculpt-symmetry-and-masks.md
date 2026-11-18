# Symmetry and masks while sculpting

> Sculpt one side and see both, round an axis too; mask the parts you want to keep still; and move, turn or scale everything else with the gizmo.

## When to use it

- **Symmetry** halves the work on anything with two matching sides: faces, creatures, most props. Turn it on (**L**) and every stroke is repeated on the other side, exactly mirrored. **Radial** symmetry repeats a stroke round an axis, for flowers, wheels, crowns, starfish and tentacles.
- A **[mask](concepts.md#sculpt-mask)** holds parts of the shape still. Paint it over an eye before sculpting the brow, over a finished arm before smoothing the shoulder, or over a whole body before pulling out a tail with the gizmo. Masked parts are drawn darker.
- With part of a shape masked, the **gizmo** (**W**, **R**, **S**, **T**) moves, turns or scales the rest: bend a tail, turn a head, lengthen a horn, without touching the body.

Symmetry while sculpting is not the same as the [live mirror](symmetry.md) you model with: the live mirror makes a real second half, while sculpting's symmetry repeats your strokes on a shape that is already whole. If the shape has a live mirror, Sculpt mode asks whether to bake it in first ([Sculpting](sculpting.md)).

## How to use it

### Symmetry

1. In Sculpt mode, press **L** (or tick **Symmetry** in the Sculpt tab). The brush's circle now lies on the surface under the mouse, and a dashed circle shows where the mirrored stroke goes.
2. Sculpt as usual: both sides change together, one undo step per stroke.
3. To mirror across another plane, pick **X** (left and right, the standard), **Y** (front and back) or **Z** (up and down) in the Sculpt tab; several at once give four or eight copies.
4. For **radial** symmetry, tick **Radial**, set how many copies (2 to 64) and the axis they turn round (Z, up, is usual).

The mirror goes through the shape's own pivot and turns with the object, as the live mirror does. Strokes mirror in space even on a shape that isn't quite symmetrical.

**Make symmetrical** (in the Sculpt menu and the Sculpt tab) fixes a sculpt that drifted, for example after sculpting with symmetry off: **+ side to − side** copies everything on the plus side of the first chosen axis onto the minus side, and **− side to + side** the other way. It needs the shape under the sculpt to be symmetrical itself (every point having a partner at its mirrored place); if it isn't, it says so and changes nothing.

### Masks

1. Press **9** for the **Mask brush** and paint over what should stay still. Masked parts turn darker. Hold **Ctrl** while painting to take the mask away; the brush's **Strength** and **Falloff** make a softer or harder edge.
2. Press **9** again for the **box mask**: drag a box on the view, and everything inside it is masked, right through the shape (the back too). Press **9** once more for the **lasso mask**: draw round what to mask. Hold **Ctrl** while dragging to unmask instead. A fourth **9** goes back to the brush.
3. Sculpt: masked parts don't move under any brush; partly masked parts (a soft edge) move partly.
4. Change the whole mask at once from the **Sculpt** menu (shown in Sculpt mode) or the Sculpt tab's **Mask** section:
   - **Invert mask** (**Ctrl + I**): masked becomes free and free masked. Mask what you want to work on, then invert.
   - **Clear mask** (**Alt + M**).
   - **Grow mask** and **Shrink mask** by one ring of points: **Ctrl + wheel** over the view, up to grow, down to shrink.
   - **Sharpen mask** and **Soften mask**: a harder or gentler edge.
   - **Mask from cavity**: the creases and hollows, so you can sculpt the raised parts without filling them.
   - **Mask from the edges**: the border of an open shape, so it stays put.
   - **Mask what faces up**, **down** or **the view**.

Every mask change is one undo step, and the mask is saved with the project.

### Moving the unmasked part

1. Mask what should stay still (often: mask the part to move, then **Ctrl + I**).
2. Press **W** (move), **R** (turn), **S** (scale) or **T** (all three). The gizmo appears at the middle of the unmasked part.
3. Drag it. Partly masked points follow partly, so a soft mask edge bends smoothly. **Esc** while dragging cancels.
4. Press **Q** or a brush key to sculpt again.

With nothing masked, the gizmo moves the whole object, as everywhere else.

### Make a shape from the mask

Mask a part of the sculpt (an armour plate, a belt, a collar), set **Thickness** in the Sculpt tab's Mask section, and choose **Make a shape from the mask**. The masked polygons become a new object, given that thickness (as the [Shell](modifiers.md) modifier would), where the sculpt is. The new object is selected; pick the sculpt in the Objects panel to carry on sculpting it.

## Settings

| Setting | What it does |
|---|---|
| **Symmetry** | Repeats each stroke mirrored (**L**) |
| **X**, **Y**, **Z** | Which planes it mirrors across: X left and right (standard), Y front and back, Z up and down |
| **Radial**, copies, axis | Repeats each stroke this many times round the chosen axis (2 to 64) |
| **Mask brush**, **Box mask**, **Lasso mask** | What the Mask brush (**9**) draws with |
| **Thickness** | How thick Make a shape from the mask makes the new object (1 cm as standard) |

## Shortcuts

| Action | Shortcut |
|---|---|
| Symmetry on or off | `L` |
| Mask brush; again: box mask, lasso mask | `9` |
| Unmask while painting or drawing a box or lasso | hold `Ctrl` |
| Invert the mask | `Ctrl + I` |
| Clear the mask | `Alt + M` |
| Grow / shrink the mask | `Ctrl + wheel` |
| Move, turn, scale the unmasked part | `W`, `R`, `S`, `T`, then drag the gizmo |

## Tips and common problems

- **Problem:** Make symmetrical says the shape isn't symmetrical. **Why:** the shape under the sculpt (the shape before sculpting) has points that have no partner on the other side, for example a shape modelled or remeshed off-centre. **Fix:** strokes with symmetry still mirror in space; to use Make symmetrical, start from a shape built symmetrically about its pivot.
- **Problem:** the mirrored stroke lands in the wrong place. **Why:** the mirror goes through the object's pivot, not its middle. **Fix:** put the pivot in the middle (Object mode's pivot tools) before sculpting.
- **Problem:** the back of the shape was masked too. **Why:** the box and lasso masks go right through the shape. **Fix:** unmask the back with **Ctrl** and a box from the other side, or paint the mask with the brush instead.
- **Problem:** a brush does nothing. **Why:** the part is masked (shown darker). **Fix:** **Alt + M** clears the mask.

## Related

- [Sculpting](sculpting.md) and [Sculpt brushes](sculpt-brushes.md).
- [Symmetry](symmetry.md) for the live mirror while modelling, and [mirror painting](painting-textures.md) in the painter.
- Concepts: [mask (sculpting)](concepts.md#sculpt-mask), [radial symmetry](concepts.md#radial-symmetry), [matcap](concepts.md#matcap).
