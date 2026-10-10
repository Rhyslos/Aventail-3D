# Objects and the Object Manager

> List, select, rename, nest, show, hide and delete the objects in your scene, and undo any change.

![The Object Manager with two groups and the test cube](../screenshots/2026-10-10-object-manager.png)

## When to use it

Every model is built from [objects](concepts.md#objects-and-scenes), and the **Objects** panel (the Object Manager, bottom right) lists them all as a tree. Use it to keep a scene tidy as it grows: give parts clear names ("Barrel", "Handle"), gather the parts of one asset into a [group](concepts.md#objects-and-scenes) so they move and hide together, and hide what's in the way while you work on something else.

For now the scene holds the 2 m test cube, and you can add **groups**: empty objects whose job is to hold others. Shapes you can model (cubes, cylinders and the rest) arrive in the next updates and will appear here the same way.

## How to use it

**Select.** Click a name to select that object; its name turns orange. **Ctrl + click** adds an object to the selection, or takes it out again. **Shift + click** selects every row from the last one you clicked to this one. Click the empty space below the list, or press **Esc**, to select nothing. The status bar says how many objects are selected.

**Create a group.** Use **Create → Group**, the group button on the command palette (the cross with a circle, after the shapes), or **Object → New group** in the panel's own menu. New objects appear at the top of the list, already selected, and are named *Group*, *Group 2* and so on.

**Put objects in a group.** Select them and press **Alt + G** (or **Object → Group objects**): they go into a new group, which takes the place of the first of them. To take them out again, select the group and press **Shift + G** (**Object → Ungroup**): its contents go back where the group was, and the empty group is removed.

**Drag to rearrange.** Drag a row (press on its name and move the mouse):
- onto the **middle** of another row to put it **inside** that object (an outline shows where it will go);
- onto the **top or bottom edge** of a row to put it **above or below** that row, beside it (a line shows where);
- onto the **empty space** below the list to move it back to the top level.

If several objects are selected and you drag one of them, they all move, keeping their order. An object can't be dropped inside itself or inside something it contains; the mouse pointer shows a "no" sign there.

**Fold.** Objects with something inside have a small arrow on the left. Click it to fold the contents away, and again to show them. **View → Collapse all** and **Expand all** fold or open everything.

**Rename.** Double-click a name (or select one object and use **Edit → Rename**), type the new name and press **Enter**. Clicking elsewhere keeps the new name too; **Esc** keeps the old one. Names can't be empty.

**Show and hide.** Click the eye at the right of a row to hide that object from the viewport; click the closed eye to show it again. If the object is part of the selection, the whole selection is shown or hidden. Hiding a group hides everything inside it too: their own eyes stay open but turn faint, so you can see they're only hidden because of the group. **Object → Show** and **Hide** work on the selection. Hidden objects are left out when you frame with **H** or **F**.

**Delete.** Press **Delete** or **Backspace** (or use **Edit → Delete**) to delete the selected objects, together with everything inside them.

**Right-click** a row for the same commands in one menu. (Right-clicking a row that isn't selected selects it first.)

**Undo and redo.** Every change to the scene can be taken back: creating, renaming, moving, grouping, showing, hiding and deleting. Press **Ctrl + Z** to undo and **Ctrl + Y** (or **Ctrl + Shift + Z**) to redo, or use the arrows at the left of the command palette or the **Edit** menu, which names what would be undone ("Undo: Delete object"). Undo brings back exactly the same objects, in the same places. The status bar confirms what was undone. Changing only the selection is not an undo step (yet).

**Framing.** With objects selected, **F** (*frame selected*) fits them in the view; with nothing selected it fits everything shown, like **H**. See [Navigating the view](navigating-the-view.md).

## Settings

The Object Manager has no settings of its own yet. Its menus:

| Menu | Entries |
|---|---|
| **Edit** | Undo, Redo, Rename, Delete, Select all, Deselect |
| **View** | Expand all, Collapse all |
| **Object** | New group, Group objects, Ungroup, Show, Hide |

The main menu bar's **Edit** menu has the same entries as the panel's Edit menu, and **Create → Group** makes a group.

## Shortcuts

| Action | Shortcut |
|---|---|
| Undo | `Ctrl + Z` |
| Redo | `Ctrl + Y` or `Ctrl + Shift + Z` |
| Delete the selection | `Delete` or `Backspace` |
| Select nothing | `Esc` |
| Put the selection in a new group | `Alt + G` |
| Take the contents out of the selected groups | `Shift + G` |
| Add to or take out of the selection | `Ctrl` + click |
| Select a run of rows | `Shift` + click |
| Rename | Double-click the name |

On macOS, use `Cmd` instead of `Ctrl` and `Option` instead of `Alt`. They don't act while you're typing a name.

## Tips and common problems

- **Tip:** name and group as you go. A tidy tree makes exporting easier later, because objects become the separate parts Godot and Substance Painter see.
- **Problem:** something vanished from the viewport but is still in the list. **Why:** it, or a group it's inside, is hidden. **Fix:** look for a closed eye on the object or on the groups above it, and click it.
- **Problem:** a row won't drop where you want. **Why:** an object can't go inside itself or one of its own children. **Fix:** drag the child out first, or drop the parent somewhere else.
- **Problem:** Delete or Backspace does nothing. **Why:** nothing is selected, or you're typing a name. **Fix:** click the object first, or press Enter to finish the name.
- **Problem:** you deleted something by mistake. **Fix:** press **Ctrl + Z**. Undo keeps every step for as long as the app is open.

## Related

- [Concepts](concepts.md): [objects, groups and hierarchy](concepts.md#objects-and-scenes), and [undo](concepts.md#objects-and-scenes).
- [Interface tour](interface-tour.md): where the Objects panel sits, and how to move it.
- [Navigating the view](navigating-the-view.md): framing the selection with **F**.
