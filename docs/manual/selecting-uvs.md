# Selecting UVs, isolating pieces and copying UVs

> Select UVs by what's wrong with them or what they're like, grow and shrink a selection without crossing seams, show only some pieces, let the UV Editor keep its own selection, and copy one piece's layout onto others made the same way.

## When to use it

- **Select overlapping** / **Select mirrored**: jump to the problems the UV Editor marks in red and purple, to fix them in one go.
- **Select stretched**: find the polygons whose texture is stretched (the ones the [Stretch view](checker-and-stretch.md) shows red), to relax or re-cut them.
- **Select similar pieces**: pick every bolt, plank or repeated part like the one selected, to line them up, stack them or pack them together.
- **Grow / Shrink in the texture**: widen or narrow a selection by one row of polygons, never across a [seam](seams.md): handy to take exactly one piece's part.
- **Shortest path in the texture**: select the edges between two points along a piece, to mark a seam or stitch along it.
- **Isolate the selected pieces**: show only what you're working on in the UV Editor.
- **The UV Editor's own selection**: pick UVs in the editor without losing what's selected in the viewport.
- **Copy UVs to similar pieces**: a model has several copies of a part (wheels, bolts) modelled separately; lay one out nicely, then give the others the same layout.

## How to use it

Everything is in **UV → Select in UVs** (the UV Editor's own **UV** menu has it too).

1. Select the objects (and, for Similar, Grow, Shrink and Copy, some polygons in the Polygons mode **3** or the UV mode).
2. Choose the selection. The result is selected in the Polygons mode (or stays in the UV mode); **Shortest path** needs two points selected in the Points mode (**1**) and selects edges. Each is one undo step.
3. **Isolate the selected pieces** (a tick): the UV Editor shows only the pieces holding the selection. Untick it to see everything again. It only changes what the editor shows, and what you can reach there: a [UV brush](piece-tools.md) leaves the hidden pieces alone, so you can brush one piece without nudging the pieces lying under it.
4. **UV Editor follows the viewport's selection** (ticked as standard): untick it and the editor keeps a selection of its own, starting from the viewport's. Clicks and boxes in the editor change only the editor's, with no undo steps, and the editor's gizmo, brushes, Turn & flip, [Stitch and Cut](piece-tools.md) (**C** and **X** over the editor) and Frame selected use it. The UV menu's other commands (Unwrap, Pack, …) still use the viewport's selection. Tick it again to share one selection.
5. **Copy UVs to similar pieces**: select polygons in two or more pieces made the same way. The piece holding the lowest-numbered selected polygon is copied onto the others, each staying where it was. Pieces made differently are skipped and the status bar says how many.

## Settings

| Setting | What it does |
|---|---|
| Stretched | One and a half times or more counts as stretched. |
| Similar | The same number of polygons and corners, and areas within 2%. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Every command here | **UV → Select in UVs** (no keys, as chosen for isolate and sync) |
| Select the whole pieces | **Alt + A** over the UV Editor (see [Selecting](selecting.md)) |

## Tips and common problems

- **Problem:** Similar finds nothing more. **Why:** the other pieces differ by more than 2% in size, or are made of different polygons. **Fix:** check them with View → Numbers on pieces in the UV Editor.
- **Problem:** after unticking "follows the viewport", Unwrap ignored what I picked in the editor. **Why:** menu commands use the viewport's selection. **Fix:** tick it again, or select in the viewport.
- **Problem:** Copy UVs skipped a piece. **Why:** it's made differently (another polygon count or order round a point), or it's locked. **Fix:** copy only between real copies of a part.
- **Tip:** Select similar, then [Align and spread](piece-tools.md) or [Pack](packing.md) with stacking on, to stack repeated parts on one patch of texture.

## Related

- [The UV Editor](uv-editor.md)
- [Stitch, cut, align, orient, warp and brush pieces](piece-tools.md)
- [Seeing and checking UVs](seeing-and-checking.md)
- [Selecting](selecting.md)
