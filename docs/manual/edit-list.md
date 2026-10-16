# The edit list

> Every edit you've made on an object is listed in the Edit layers panel. Click one to change its settings, however long ago you made it; switch edits off, keep them editable, freeze old ones, or delete them.

Each extrude, bevel, move and so on is recorded as an [edit](concepts.md#objects-and-scenes) on top of the shape, in order. Until now only the last one could be changed. The edit list opens up all of them: change the width of a bevel you made twenty edits ago, and every edit after it is made again on the new shape, finding its points by where they are. It works like going back down 3ds Max's modifier stack, for every single edit.

![The Attributes panel showing "Edit: Extrude 1 polygon", opened from the edit list below it: On, Keep editable, Distance and the extrude's other settings. In the Edit layers panel, the extrude is picked under its layer, Edits, with the layer Edits 2 above it holding a later move.](../screenshots/2026-10-16-edit-list.png)

## When to use it

- **To fix an early decision:** the first extrude was too deep, a bevel too wide, a loop cut in the wrong place.
- **To compare:** switch one edit off to see the shape without it, then on again.
- **To keep a big model quick:** freeze old edits you won't change again, so they aren't made again every time something below them changes.
- **To take one edit out** from the middle, without undoing everything after it.

## How to use it

1. Select one object and open the **Edit layers** tab (beside Objects). Each layer lists its edits, newest first.
2. **Click an edit.** The Attributes panel shows it as **Edit: …**, with:
   - **On**: untick to see the shape without it;
   - **Keep editable**: tick to keep it changeable for good (Freeze never bakes it);
   - its **settings**, the same ones the Last edit section shows. Change them and the shape follows;
   - **Freeze up to here** and **Delete edit**.
3. Click **Close** (or make a new edit) to go back to the last edit.

Or **right-click** an edit for the same things: Open in Attributes, Switch off, Keep editable, Freeze up to here, Delete edit. The small eye in front of each edit switches it off and on.

Every change is one undo step (**Ctrl + Z**).

## Settings

| Setting | What it does |
|---|---|
| **On** | Off: the shape is made as if this edit wasn't there. Nothing is lost. A switched-off edit shows "(off)" in the list. |
| **Keep editable** | Marks the edit **kept**. Freeze stops at it, so it stays changeable. Use it for the edits you're likely to tweak (the main bevel, the panel line depth). |
| **Freeze up to here** | Bakes the shape's settings and every edit up to and including this one (in its layer and the layers below) into the shape. The shape looks the same, the edits after it still work, and the model gets quicker to change, because those edits no longer have to be made again. A primitive becomes a plain mesh: its Width, Segments and other settings go, as with **Make editable**. Layers whose edits were all frozen go too. |
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
- **Problem:** Freeze up to here is greyed out. **Why:** one of the edits it would bake is kept editable or switched off, or a layer it's in is off. Freeze never bakes something you asked to keep, and won't guess about switched-off edits. **Fix:** hover over the button to see which; switch them on, delete them, or untick Keep editable.
- **Tip:** changing an early edit works best when the edits after it were made on parts it doesn't remove. A bevel on an extrude's side keeps working when the extrude gets deeper; a loop cut through polygons that a later edit deletes won't.
- **Tip:** while an edit is open, the tool you're using shows its settings for the next drag only after you click **Close**.

## Related

- [Edit layers](edit-layers.md)
- [The Attributes panel: Last edit](attributes.md)
- [Concepts: base and edits, freezing, keeping an edit editable](concepts.md#objects-and-scenes)
