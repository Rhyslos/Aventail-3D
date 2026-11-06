# Stitch, cut, align, orient and warp pieces

> Hand tools for UV pieces: join pieces along an edge, cut them apart, line them up, turn a piece square to one of its edges, and bend UVs with a lattice of handles.

## When to use it

The automatic tools ([Suggest seams](seams.md), [Unwrap](unwrapping.md), [Pack](packing.md)) get most of a layout right. These are for the last touches:

- **Stitch**: two [pieces](concepts.md#uvs-and-texturing) that belong together were cut apart (a seam you don't want after all). Stitching moves the smaller one to meet the other and joins them, so the texture runs on across the edge with no visible join.
- **Cut**: a piece is too big or too stretched, or you want part of it somewhere else in the texture. Cutting along edges makes them [seams](seams.md) and nudges a part that comes loose away, so you can move it.
- **Align and spread**: tidy a row of pieces (trim-sheet strips, planks, bolts) so their edges line up, or so they're evenly spaced.
- **Orient by edge**: a piece lies at a slant; turn it so one of its edges runs straight across or straight up, which wastes less texture and keeps wood grain or brick rows level.
- **Lattice warp**: gently bend UVs (fit a piece to a curved part of a trim sheet, straighten a slight bow) without moving each point.

## How to use it

**Stitch and Cut** work on edges: in the Edges mode (**2**) select the edges in the UV Editor or the viewport. With polygons selected instead, Stitch joins any of their cut edges, and Cut cuts round the selection.

1. With the mouse over the UV Editor, press **C** to stitch or **X** to cut. Or choose **UV → Pieces → Stitch** or **Cut**, or **E → Stitch & cut** over the UV Editor (or in the UV mode).
2. Stitch: the smaller piece moves, turns and resizes to meet the other along the edge, the corners there become one, and the seam mark goes. A [locked](locked-pieces.md) piece never moves; the other comes to it.
3. Cut: the edges become seams. If that cuts a part right off, the smaller part moves two texture pixels away from the cut so you can see and grab it. A cut that only slits a piece (it's still joined round the end) opens up when you [Unwrap](unwrapping.md).

**Align and spread:** select polygons in two or more pieces of one object and choose **UV → Pieces → Align and spread**, then:
- **Align left / right / top / bottom**: each piece's side on the outermost one's;
- **Align centres across / middles up and down**: each piece's middle on the middle of them all;
- **Spread evenly across / up** (three or more pieces): the first and last stay, the others are spaced so the gaps between their middles are even.

**Orient by edge:** in the Edges mode, select one edge of each piece to turn, then **UV → Pieces → Orient by edge**. Each piece turns about its middle by the smallest angle that makes its edge straight across or straight up.

**Lattice warp:**
1. Select the pieces (or the object), and choose **UV → Pieces → Lattice warp**.
2. A grid of 3 × 3 handles appears over them in the UV Editor. Drag a handle: the UVs follow it, bending smoothly towards the handles that stay. Each drag is one undo step.
3. Press **Esc** or click **Done warping** to put the lattice away.

Every tool here is one undo step, saved with the project and recorded on the object's [edit list](edit-list.md).

## Settings

None: the tools use the selection. The cut's nudge is two pixels of the [texture size](texture-and-density.md).

## Shortcuts

| Action | Shortcut |
|---|---|
| Stitch | **C** over the UV Editor; **UV → Pieces**; **E → Stitch & cut** |
| Cut | **X** over the UV Editor; **UV → Pieces**; **E → Stitch & cut** |
| Align and spread, Orient by edge, Lattice warp | **UV → Pieces** |
| Put the lattice away | **Esc**, or **Done warping** |

**C** and **X** only stitch and cut while the mouse is over the UV Editor: over the viewport, C still [bridges](connect-bridge-weld.md) and X still [slits](slit.md).

## Tips and common problems

- **Problem:** Stitch moved the wrong piece. **Why:** it moves the smaller (fewer polygons). **Fix:** [lock](locked-pieces.md) the piece that should stay, then stitch.
- **Problem:** stitched pieces overlap others. **Why:** the moved piece came to its partner wherever that is. **Fix:** [Pack](packing.md) afterwards.
- **Problem:** Cut made a seam but nothing moved. **Why:** the cut didn't go all the way round, so the piece is still one. **Fix:** cut the rest of the way, or Unwrap to open the slit.
- **Problem:** Align says it needs two pieces. **Why:** the selected polygons are all in one piece, or in different objects (each object's pieces are lined up on their own).
- **Tip:** Orient by edge, then Pack with turning off (UV → Settings → Packing), keeps pieces level for wood and brick.

## Related

- [Editing UVs by hand](editing-uvs.md) (move, turn, scale and flip)
- [Seams](seams.md)
- [Locking pieces](locked-pieces.md)
- [Packing](packing.md)
- [Trim sheets](trim-sheets.md)
- [Concepts: UV map, seam, piece](concepts.md#uvs-and-texturing)
