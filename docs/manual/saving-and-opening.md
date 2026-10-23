# Saving and opening projects

> Save your scene as a project file (`.avtl`) and open it again later, exactly as you left it.

![The File menu](../screenshots/2026-10-11-file-menu.png)

## When to use it

Whenever you want to keep your work. A project file holds everything in your scene:
- every object, its name, where it sits in the Objects panel, and whether it's hidden;
- each shape's settings (so a cube is still a cube with editable size and segments when you open it again);
- plain meshes made with Bake into shape;
- each object's smoothing (smoothed by angle, and its angle) and its symmetry;
- edits recorded on top of shapes;
- what was selected.

Project files are only for this app. To take a model into Substance Painter or Godot, you *export* it (see [Export and import](export-and-import.md)); the project file stays your working copy.

## How to use it

1. **Save** with **File → Save** or **Ctrl + S**. The first time, a window asks for a name and a folder; the app adds `.avtl` to the name if you leave it out. After that, Ctrl + S saves to the same file without asking.
2. **Save under a new name** (to keep the old version, say) with **File → Save As…** or **Ctrl + Shift + S**.
3. **Open** a project with **File → Open…** or **Ctrl + O**, or pick one from **File → Open recent**, which lists the last eight projects you saved or opened (hover over one to see where it is). The opened scene replaces the current one, and the view frames it.
4. **Start again** with **File → New** or **Ctrl + N**: an empty scene.

The project's name is shown on the right of the menu bar. When there are changes you haven't saved, a red **Unsaved** badge appears next to it, and the window's title gets a dot in front and "(unsaved)" after the name (for example "● crate.avtl (unsaved) — Aventail 3D"), so you can see it even in the taskbar. Before the first save the project is called *Untitled*.

**Closing the app:** use **File → Quit** or the window's **×**. There's no key for quitting, on purpose: the keys you use all the time shouldn't be able to close the app by accident.

**You won't lose changes by accident.** If you start a new project, open another, or close the app with unsaved changes, the app asks first:

![The question asked before unsaved changes would be lost](../screenshots/2026-10-11-save-changes.png)

- **Save** (or **Enter**) saves, then carries on. If the project has never been saved, it asks where first; closing that window without choosing cancels everything.
- **Don't save** (or **Alt + N**) throws the changes away and carries on.
- **Cancel** (or **Esc**) goes back to your work.

**Saving is safe.** The app writes the new file next to the old one, checks it arrived whole, and only then puts it in place. So a crash or a full disk while saving never leaves you with a damaged project. The previous save is kept as a backup beside it, named like `crate.avtl.bak`: if you ever need it, rename it to end in `.avtl` and open it.

**Backup copies cover the time between saves.** While you work, the app also saves backup copies of your scene in the background (every 5 minutes as standard, only when something changed). They never replace your project file. If the app ever closes without saving, it offers them back the next time it starts. See [Backup copies and crash recovery](autosave.md).

**Saving can export too.** When the project has a game project folder with **Export on every save** ticked, each save also exports the model into that folder, so Godot, Unity or Unreal Engine picks up the change. See [Export into a game project](export-and-import.md#export-into-a-game-project).

**Undo starts fresh** when you open a project: the undo history isn't saved in the file (yet). Saving doesn't clear it, so you can still undo after saving.

You can also open a project by giving it to the app when it starts (for example, from a terminal: `aventail crate.avtl`). Once the app has an installer, double-clicking a project file will do the same.

## Settings

None of its own. Your preferences (accent colour, navigation and so on) are not part of a project: they belong to you, not the scene, and are kept in [Settings](settings.md). A few things that are part of the model are saved with it: the project's **units** (metres or centimetres), where every object's **pivot** sits ([Pivots and Edit pivot](pivots.md)), its [materials](materials.md), and its **game project folder** ([Export and import](export-and-import.md#export-into-a-game-project)). A new project starts in the units chosen in Settings.

## Shortcuts

| Action | Shortcut |
|---|---|
| New project | `Ctrl + N` |
| Open | `Ctrl + O` |
| Save | `Ctrl + S` |
| Save As | `Ctrl + Shift + S` |
| In "Save changes?": save / don't save / cancel | `Enter` / `Alt + N` / `Esc` |

On macOS, use `Cmd` instead of `Ctrl`.

## Tips and common problems

- **Problem:** "Couldn't save … it wasn't saved, because the saved copy wouldn't open again". **Why:** a bug made the project into something the app couldn't read back, and the save was stopped before it could replace your previous save. **Fix:** your previous save is untouched, and your work is still open. Try undoing the last step and saving again, or use Save As under a new name. A copy of the unreadable save is kept beside the project (`name.avtl.unreadable`); please report the problem with it.
- **Tip:** save often, and use Save As with a number (crate-01.avtl, crate-02.avtl) before big changes, so you can always go back.
- **Problem:** "Couldn't open … it was saved by a newer version of the app". **Why:** the project was saved by a newer version, in a format this version doesn't know yet. **Fix:** update the app. Projects open in every newer version, but once saved by a newer version they may not open in an older one: this version saves in format 40, which holds each object's own smoothing, mirrors that belong to the object, and [booleans](booleans.md). (Newer versions always open projects from older ones. Projects with extrudes, insets, bevels, loop cuts, slits, pipe bends, Connect, Bridge, Weld, clean-up edits, hard and soft edges, instances, slides, symmetry or spirals need a version from October 2026 on.)
- **Problem:** "Couldn't open … the file is damaged". **Why:** the file was cut short or changed by something else (an interrupted copy, a sync tool). **Fix:** open the backup: rename `name.avtl.bak` so it ends in `.avtl`, and open that.
- **Problem:** "Couldn't open … it holds more than the app can open" (or too many objects, edits or polygons). **Why:** the app sets limits on what a project may ask of your computer, so a damaged file, or one made to cause trouble, can't freeze it or use up its memory. The limits are far above anything you can make in the app: up to 100,000 objects, 100,000 recorded edits, and about 30 million polygons in all. **Fix:** if it's your own project, open the backup (`name.avtl.bak`). If someone sent it, ask them to send it again; the copy may have been damaged on the way.
- **Problem:** the app crashed, or the power went, before you saved. **Fix:** start the app again: it offers the backup copies it saved in the background ([Backup copies and crash recovery](autosave.md)).
- **Problem:** "Couldn't open … it isn't a project file". **Why:** the file isn't one of this app's projects. **Fix:** to bring in a model from elsewhere, use import (from Phase 5).
- **Problem:** a project in Open recent is gone from the list. **Why:** it couldn't be found when you tried to open it (moved, renamed or deleted). **Fix:** use **File → Open…** to find it where it is now.

## Related

- [The Objects panel](objects.md): undo and redo.
- [Backup copies and crash recovery](autosave.md): autosave, and what happens after a crash.
- [Export and import](export-and-import.md): exporting into a game project on every save.
- [Settings](settings.md): preferences that belong to you, not the project.
- [Concepts](concepts.md): [project file](concepts.md#objects-and-scenes).
