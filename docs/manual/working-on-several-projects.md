# Working on several projects: tabs

> Each open project has a tab, so several pieces can be open in one window and you can switch between them with a click.

## When to use it

Game art often comes in sets: a crate, a barrel and a sack that should look alike, or a modular kit spread over a few projects. With tabs you can keep them all open and go back and forth, each exactly as you left it: the same shapes, the same selection, the same view, the same mode and layout, and its own undo history. Undo in one tab never touches another.

If you'd rather keep one project at a time, nothing makes you open a second: with one tab the app works as it always has. A setting (below) makes **New** and **Open** replace the project in the tab in use instead of starting a new tab.

## How to use it

The tabs sit on the menu bar, between the menus and the [layout buttons](interface-tour.md#layouts). Each shows the project's name: its file name, or **Untitled** before it is first saved (**Untitled 2**, **Untitled 3** and so on when there are several).

- **Switch:** click a tab. The window shows that project as you left it. The window's title names the project in use.
- **Start a new project in a new tab:** click **+** after the tabs, or **File → New** (**Ctrl + N**).
- **Open a saved project in a new tab:** **File → Open…** (**Ctrl + O**) or **File → Open recent**. A project that's already open in a tab isn't opened twice: its tab comes to the front.
- **See what's unsaved:** a tab with changes you haven't saved has a **dot** in the [accent colour](interface-tour.md#accent-colour) before its name.
- **Close a tab:** click its **×**, or click the tab with the **middle mouse button** (the wheel). If it has unsaved changes, the tab comes to the front and asks first, as closing the app does: **Save**, **Don't save** or **Cancel**. Closing the last tab leaves a new, empty project.
- **Reorder:** drag a tab along the row and let go where you want it.
- **Lots of tabs:** they shrink to share the room; when even that isn't enough, the row scrolls (turn the mouse wheel over it). The tab in use is always brought into sight. Point at a tab to read its whole name and where it is saved.

When **New** or **Open** would start a new tab but the tab in use holds an empty project you haven't touched yet (as when the app has just started), they use that tab instead, so you don't collect empty tabs.

**Quitting** with unsaved changes in one tab asks about that tab, in that tab. With changes in several, a window lists them first:

- **Go through them** asks about each in turn, in its own tab: Save, Don't save or Cancel. Cancel at any point stops quitting, and the app stays open.
- **Close without saving** closes the app and loses the changes in every listed project. It has no key, on purpose.
- **Cancel** (or **Esc**) keeps the app open.

## Settings

In **Edit → Settings… → Interface → Projects** (see [Settings](settings.md)):

| Setting | What it does |
|---|---|
| **Tabs: In the menu bar** / **On their own row** | Where the tabs sit. **In the menu bar** (standard) puts them between the menus and the layout buttons, so the views keep all their height. **On their own row** gives them a row above the menu bar, as in a web browser: more room for long names, a little less height for the views. |
| **New and Open start a new tab** | On (standard): **New** and **Open** start a new tab (or use the tab in use while its project is empty and untouched). Off: they replace the project in the tab in use, asking to save its changes first, as the app did before tabs. **+** always starts a new tab. |

Settings, your brushes, colours, recipes and own boxes, and the recent projects are the same in every tab: change one in any tab and the others have it too.

## Shortcuts

Tabs are used with the mouse only; they add no keys.

| Action | How |
|---|---|
| Switch to a tab | Click it |
| New project in a new tab | **+**, or **File → New** (`Ctrl + N`) |
| Open a project in a new tab | **File → Open…** (`Ctrl + O`) |
| Close a tab | **×**, or middle-click the tab |
| Move a tab | Drag it along the row |

## Tips and common problems

- **Problem:** File → New didn't start a new tab. **Why:** the project in use was empty and untouched, so it was used instead; or **New and Open start a new tab** is off in Settings. **Fix:** click **+** for a new tab whatever the setting.
- **Problem:** a tab won't close. **Why:** it is still being saved or exported, and closing waits for that (the status bar says so). **Fix:** close it again once the status bar says it's done.
- **Problem:** the names are cut short. **Why:** many tabs share the room. **Fix:** point at a tab for its whole name, or put the tabs on their own row in Settings.
- Each open tab keeps its shapes ready to draw, so many heavy projects open at once use more of your computer's memory. Close the ones you've finished with.
- Backup copies are made for each tab with unsaved changes, and when you leave a tab with changes not yet in a backup, one is made straight away. After a crash, the next start offers every tab's work back. See [Backup copies and crash recovery](autosave.md).

## Related

- [Saving and opening projects](saving-and-opening.md)
- [Backup copies and crash recovery](autosave.md)
- [Interface tour](interface-tour.md)
- [Settings](settings.md)
- [Concepts: tab](concepts.md#viewing-the-scene)
