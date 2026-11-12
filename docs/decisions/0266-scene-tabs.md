# 0266. Scene tabs: several projects in one window

- **Status:** Accepted
- **Date:** 2026-11-08

## Problem

The app held one project at a time: opening another meant closing the first. Working on a set of props, or taking a part from one project into another, means opening and closing again and again. The project lead looked at mockups (the Design artifact with options A to F) on 2026-11-08 and chose tabs, one per project, as in a web browser. The whole plan, in four steps, is in [docs/plans/scene-tabs.md](../plans/scene-tabs.md); this decision covers step 2, the tabs themselves.

## Options

**Where the tabs sit:**

1. **F, in the menu bar** between the menus and the layout buttons: no height taken from the views; room for fewer tabs before they shrink.
2. **E, a row of their own** above the menu bar, browser-style: more room for names, 32 pixels less for the views.

**How each tab holds its project:**

1. **A whole workbench per tab:** everything about a project (document, undo, selection, view, mode, layouts, caches, background work) already lives in the workbench; a tab left is simply not shown. Shared settings are handed from the tab in use to the next.
2. **One workbench, swapping a tab's state in and out:** one copy of the settings, but every one of the workbench's hundred-odd parts would have to be sorted into "the project's" and "the app's", and kept sorted as features are added. Easy to get wrong without noticing.

## Decision

The project lead's choices, built as:

- **F is the standard; E is a setting** (Settings → Interface → Projects → Tabs: **In the menu bar** / **On their own row**), saved with the view options (`ViewOptions::tab_placement`).
- A tab shows the project's name ("Untitled", then "Untitled 2" and so on before it is saved), a **dot** in the accent colour while it has unsaved changes and **×** to close it; **+** after the tabs starts a new project. **Mouse only, no new keys:** click to switch, × or a **middle click** to close, **drag** to move along the row. Many tabs shrink alike to a least width (where × shows only on the tab in use and the one under the mouse), then the row scrolls (the mouse wheel scrolls it), keeping the tab in use in sight.
- **New and Open start a new tab**, but use the tab in use while its project is empty and untouched (never saved, nothing made, nothing to undo). Settings → Interface → Projects → **New and Open start a new tab** (on as standard; `ViewOptions::open_in_new_tab`) off makes them replace the tab's project after asking to save, as before. A project already open in a tab isn't opened twice: its tab comes to the front.
- **Closing a tab** with changes brings it to the front and asks the usual question ("Save changes to … before closing its tab?"); a tab whose save or export is under way waits for it. Closing the last tab leaves a new, empty project.
- **Quitting** with changes in one tab asks in that tab; with changes in several, a window lists them: **Go through them** (the usual question in each tab in turn; Cancel at any point keeps the app open), **Cancel**, **Close without saving**. Throwing every change away has no key.
- **Each tab is a whole workbench** (option 1 above): `ui::SceneTabs`. Settings, the libraries (brushes, own boxes, recipes, used colours) and the recent projects are handed from the tab in use to the next when it changes, so they are the same in every tab; each tab's Edges switch, ticked by its mode, stays its own. The panel layouts saved for next time are the tab in use's; a new tab starts with them.
- **The app keeps, for each tab,** its GPU renderer (on the window's one GPU device), its backup session (a folder of its own; only the first tab offers what a crash left), and its emergency copy (a crash writes every tab's). A tab left with changes no backup holds gets one at once. One save or export runs at a time for all tabs, reported to the tab that asked. The window's title is the tab in use's.

## Consequences

- Several projects open at once, each exactly as left when switched back to.
- Each open tab keeps its GPU meshes, textures and caches: many heavy tabs use more memory. Step 3 (sleeping tabs) puts a tab's costly parts away; the renderer and the caches are already separate and droppable for it.
- Each tab's renderer makes its own GPU pipelines (a moment when a tab opens); they could be shared later if opening or waking tabs needs to be quicker.
- The project's name and Unsaved badge on the menu bar are replaced by the tabs (the dot and the name); a workbench on its own still shows them.
- Copy and paste between tabs (step 4) goes through the receiving tab's commands, so it is one undo step there.
