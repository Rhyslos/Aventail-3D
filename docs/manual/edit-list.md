# The edit list

> Every edit you've made on an object is listed in the Edit layers panel. Click one to change its settings, however long ago you made it; switch edits off, bake old ones into the shape, or delete them.

Each extrude, bevel, move and so on is recorded as an [edit](concepts.md#objects-and-scenes) on top of the shape, in order. Until now only the last one could be changed. The edit list opens up all of them: change the width of a bevel you made twenty edits ago, and every edit after it is made again on the new shape, finding its points by where they are. It works like going back down 3ds Max's modifier stack, for every single edit.

![The Attributes panel showing "Edit: Extrude 1 polygon", opened from the edit list below it: On, Distance and the extrude's other settings. In the Edit layers panel, the extrude is picked under its layer, Edits, with the layer Edits 2 above it holding a later move.](../screenshots/2026-10-16-edit-list.png)

## When to use it

- **To fix an early decision:** the first extrude was too deep, a bevel too wide, a loop cut in the wrong place.
- **To compare:** switch one edit off to see the shape without it, then on again.
- **To keep a big model quick:** bake old edits you won't change again into the shape, so they aren't made again every time something below them changes.
- **To take one edit out** from the middle, without undoing everything after it.

## How to use it

1. Select one object and open the **Edit layers** tab (beside Objects). The tab appears by itself once you select an object with more than one layer; otherwise choose **Window → Edit layers** to show it. Each layer lists its edits, newest first.
2. **Click an edit.** The Attributes panel shows it as **Edit: …**, with:
   - **On**: untick to see the shape without it;
   - its **settings**, the same ones the Last edit section shows. Change them and the shape follows;
   - **Bake into shape up to here** and **Delete edit**.
3. Click **Close** (or make a new edit) to go back to the last edit.

Or **right-click** an edit for the same things: Open in Attributes, Switch off, Bake into shape up to here, Delete edit. The small eye in front of each edit switches it off and on.

Every change is one undo step (**Ctrl + Z**).

**Drags in a row make one edit.** Drag the same points, edges or polygons several times one after another, with nothing else in between, and the list shows a single move (or turn, or scale) edit, not one per drag: the moves add up into it. Each drag is still its own undo step, so Ctrl + Z takes back just the last drag. Select something else, or use another tool, and the next drag starts a new edit.

## Settings

| Setting | What it does |
|---|---|
| **On** | Off: the shape is made as if this edit wasn't there. Nothing is lost. A switched-off edit shows "(off)" in the list. |
| **Bake into shape up to here** | Bakes the shape's settings and every edit up to and including this one (in its layer and the layers below) into the shape. The shape looks the same, the edits after it still work, and the model gets quicker to change, because those edits no longer have to be made again. A primitive becomes a plain mesh: its Width, Segments and other settings go, as with **Bake into shape** on the whole object. Layers whose edits were all baked go too. |
| **Delete edit** | Takes the edit out. The edits after it are made again without it. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Open an edit | Click it in the Edit layers panel |
| Every edit action | Right-click it |
| Switch an edit off or on | The small eye in front of it |

There are no keys for the edit list yet.

## Tips and common problems

- **Problem:** an edit says **parts missing**. **Why:** a change below it (fewer segments, a deleted edit) removed some of the points, edges or polygons it was made on, and they can't be found by place. It still applies to the rest. **Fix:** change things back, or delete the edit and make it again. Hover over the mark for details.
- **Problem:** an edit says **skipped**. **Why:** it would have made the shape larger than the app can safely handle. **Fix:** fewer rows or a smaller selection.
- **Problem:** Bake into shape up to here is greyed out. **Why:** one of the edits it would bake is switched off, or a layer it's in is off. It won't guess whether a switched-off edit should be in the shape. **Fix:** hover over the button to see which; switch them on, or delete them.
- **Problem:** I dragged some points three times and there's only one edit in the list. **Why:** drags of the same points in a row make one edit. **Fix:** nothing to fix: undo still takes back one drag at a time.
- **Tip:** changing an early edit works best when the edits after it were made on parts it doesn't remove. A bevel on an extrude's side keeps working when the extrude gets deeper; a loop cut through polygons that a later edit deletes won't.
- **Tip:** while an edit is open, the tool you're using shows its settings for the next drag only after you click **Close**.

## Related

- [Edit layers](edit-layers.md)
- [The Attributes panel: Last edit](attributes.md)
- [Concepts: base and edits, the edit list, baking into the shape](concepts.md#objects-and-scenes)
