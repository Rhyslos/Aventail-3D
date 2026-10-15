# Check for baking

> List what in your model would spoil a bake in Substance Painter, in plain words, and fix most of it in one click.

![The Check for baking window listing problems on the Cube, each with Select and, for some, Fix](../screenshots/2026-10-15-bake-check.png)

## When to use it

[Baking](concepts.md#the-game-asset-pipeline) turns the shape of your model into a normal map. Some things that look fine in the viewport come out badly in a bake: black specks, streaks, inside-out patches. They are much quicker to fix in the model than to track down from a broken normal map. Run the check before exporting for Substance Painter.

## How to use it

1. Choose **File → Check for baking…**. (With **Made for: Substance Painter** in the [Export window](export-and-import.md), there is a **Check for baking…** button there too.)
2. The window lists each problem as *Object: what is wrong*. Point at one to read why it spoils a bake and what to do.
3. Click **Select** to see it: the object is selected and, in the right mode, the polygons, edges or points concerned.
4. Click **Fix** where there is one. The fix is one undo step: **Ctrl + Z** takes it back.
5. The list updates as you work. When it says *Nothing found that would spoil a bake*, export.

## What it checks

| Problem | Why it spoils a bake | Fix |
|---|---|---|
| **Polygons facing the wrong way** | The baker sees them from behind: the normal map comes out inverted or black there. | **Fix** turns them round (as Mesh → Fix normals). |
| **Edges shared by three or more polygons** ([non-manifold](concepts.md#mesh-basics)) | They can't be unfolded or baked cleanly. | Select them, then delete or [split off](split-and-join.md) the extra polygons. |
| **Polygons with no area** | No direction to bake along: a black speck or a streak. | Select them, then [weld](connect-bridge-weld.md) their points (M) or delete them. |
| **Edges with no length** (points on the same spot) | They make polygons with no area and break the shading. | **Fix** welds each group of points into one. |
| **Many-sided polygons bent out of flat** | An [n-gon](concepts.md#mesh-basics) with a corner lifted shades unevenly across its triangles. | Select them and cut them up with [Connect](connect-bridge-weld.md) (J) or [Slit](slit.md) (X). |
| **Many-sided polygons cut into sliver triangles** | Very thin triangles shade badly and leave streaks. | Select them and cut them up with Connect (J) so the triangles are fuller. |
| **Points no polygon uses** | They go into the file for nothing and can confuse the baker. | **Fix** deletes them. |
| **A `_low` object without its `_high`** (or the other way round) | Substance Painter pairs a low-poly object with its [high-poly](concepts.md#mesh-basics) one by name: `crate_low` with `crate_high`. | Rename one, or make the missing partner. |

Only many-sided polygons (five or more corners) are checked for bends and slivers. A thin four-sided strip, like a bevel, is normal, and a slightly bent one shades predictably.

## Settings

None. Every shown shape is checked; an [instance](copies-and-instances.md) is checked through its original.

## Shortcuts

None: use the File menu.

## Tips and common problems

- **Problem:** after **Fix**, the status bar says something else got worse (*but now: 3 edges shared by three or more polygons*). **Why:** welding points can squash polygons together. **Fix:** Ctrl + Z, then select the points and fix that spot by hand.
- **Problem:** a problem keeps coming back after Fix. **Why:** an edit further down in the object's list of edits makes it again. **Fix:** Select it, then look at what you did there last.
- **Tip:** the check doesn't look at UVs yet. Overlapping UVs and seams join it with the UV tools.

## Related

- [Export and import](export-and-import.md): the Substance Painter preset.
- [Clean-up](clean-up.md): delete, dissolve, fill holes, flip and fix normals.
- [Connect, Bridge and Weld](connect-bridge-weld.md).
- [Concepts: baking, normal maps, high-poly and low-poly](concepts.md#the-game-asset-pipeline).
