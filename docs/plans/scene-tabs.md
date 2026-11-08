# Scene tabs plan

The working plan for having several projects open in one window, one **tab** each ([decision 0266](../decisions/0266-scene-tabs.md)), and the layout buttons that came first ([decision 0265](../decisions/0265-layout-buttons.md)). Agreed with the project lead on 2026-11-08 after the mockups (the Design artifact with options A to F for the menu bar).

The standing OK carries on: each pull request is merged once its checks pass, one step per pull request.

## The lead's choices

- **Layout buttons (option A):** the four layouts (Standard, UV Edit, Paint, Material) are one bar of bigger buttons with a picture and a name on the right of the menu bar, the one in use filled with the accent colour; the menu bar a little taller. The old small text tabs stay as a setting, with a temporary Help menu entry to compare them.
- **Tab row, placement F (standard):** the tabs sit in the menu bar, between the menus and the layout buttons, so the views lose no height. **Placement E** (a setting): the tabs on a row of their own above the menu bar, as in a web browser.
- **A tab** shows the project's name, a dot in the accent colour while it has unsaved changes, and × to close it; a **+** after the tabs starts a new one. A sleeping tab (step 3) shows a moon, and on hover why it sleeps and that nothing is lost.
- **Mouse only, no new keys:** click a tab to switch to it, × or a **middle click** closes it, **drag** moves it along the row, **+** opens a new tab.
- **New (Ctrl + N) and Open (Ctrl + O)** start a **new tab**, but use the tab in use while its project is empty and untouched. A setting makes them replace the project in the tab in use instead (asking to save first, as before tabs).
- **Many tabs:** they shrink alike, down to a width that still shows a few letters of the name, then the row scrolls.
- **Closing** a tab or quitting asks about unsaved changes per tab, with the usual Save / Don't save / Cancel question; quitting lists every tab with changes first.
- **Window title, recent files, backup copies and recovery, settings** work per tab: the title is the tab in use's; settings are the same in every tab.
- **Sleep (step 3):** a tab you leave sleeps straight away, provided switching back doesn't stutter: what is costly is put away (GPU meshes and textures, caches, preview and bake workers) and everything needed to wake exactly as left is kept (scene, undo, selection, view, mode, layout, unsaved state). Measure the wake time with the heavy-mesh check's big scenes; if waking a heavy scene would stutter visibly, sleep after a short delay or in the background instead, and say what was chosen and why.
- **Copy and paste between tabs (step 4):** copy objects in one tab and paste them in another, bringing their materials, paint, graphs and own boxes (ids remapped, no clashes), undoable in the receiving tab.

## Steps

| # | Step | Decision | State |
|---|---|---|---|
| 1 | Layout buttons (A), with the small tabs as a setting (Settings and, for now, Help) | [0265](../decisions/0265-layout-buttons.md) | built |
| 2 | Scene tabs: several projects in one window (tab row F, E in Settings; mouse controls; New and Open with their setting; unsaved changes asked about per tab) | [0266](../decisions/0266-scene-tabs.md) | built |
| 3 | Sleeping tabs | to come | planned |
| 4 | Copy and paste between tabs | to come | planned |

## How step 2 is built

**Each tab is a whole workbench.** `ui::SceneTabs` holds one `ui::Workbench` per tab, in the row's order, and which is in use. A workbench already holds everything about one project: the document (scene, undo history, selection), the view and camera, the mode, the four panel layouts, the drawing caches, and its own background work (bakes, graph fills, the Material preview's pictures). Keeping a whole workbench per tab means nothing has to be pulled apart or put back on switching: the tab left is simply not shown. A workbench on its own (most tests) works as before tabs.

**Shared things are handed on.** Settings (view options, accent, units for new projects, undo memory, backups, tool menu, UV and export settings), the libraries (brushes, own boxes, recipes, used colours) and the recent projects are app-wide. Only the tab in use can change them, so when the tab in use changes the new one takes them from the old (`Workbench::take_shared_from`). Each tab's own Edges switch (ticked by its mode) stays its own. Settings are saved from the tab in use; the panel layouts saved are the tab in use's.

**The tab row is drawn by the workbench in use** (`scene_tabs::strip`), in its menu bar (F) or on a panel of its own above it (E), from a `TabStrip` the tabs hand it before the frame. What is clicked comes back as `TabEvent`s (switch, close, new, move) that the tabs carry out after the frame. The workbench's own actions come back paired with the tab they are for (`Vec<(TabId, WorkbenchAction)>`), so a tab switch in the middle of a frame never sends a save to the wrong project.

**New, Open and Quit in a tab.** `Workbench::tab_action` turns New into `WorkbenchAction::NewTab` and Open into `WorkbenchAction::OpenInNewTab` (when the setting is on and the project isn't untouched), and Quit into a plain `Quit` that the tabs take over. The app asks which file (`project::which_to_open`; typing the path when no file window comes, `FilePurpose::OpenProjectInNewTab`) and opens it with `project::open_in_new_tab`: a project already open in a tab isn't opened twice, its tab comes to the front.

**Closing and quitting.** Closing a tab with changes brings it to the front and asks the usual question ("… before closing its tab"); its answer ends in `WorkbenchAction::CloseTab`, which the tabs carry out. A tab whose save or export is under way waits for it. Quitting with changes in one tab asks in that tab; with several, a window lists them, then **Go through them** asks about each in turn (Cancel at any point keeps the app open), **Close without saving** loses them all. Closing the last tab leaves a new, empty project.

**The app's part for each tab** (`app::tabs`): its GPU renderer (`GpuViewport`, on the window's one GPU device; a closed tab's renderer is dropped, freeing its meshes, textures and view images), its backup session (each tab has a session folder of its own; only the first offers what a crash left), and its emergency copy (a child of the one the panic hook holds; a crash writes every tab's). A tab being left with changes no backup holds gets a backup copy at once. One save or export runs at a time, reported to the tab that asked for it.

**Ready for step 3.** A tab's costly parts are already separate and droppable: the renderer can be dropped and made again from the document (the GPU side is rebuilt from the scene, as at start), and the workbench's caches (`DrawingCache::forget`, the bakes, graph fills and preview) can be let go and worked out again. Sleeping a tab means dropping its renderer and those caches while keeping the workbench's document, undo, selection, view, mode and layouts. The renderers could also share the GPU pipelines (made once per tab now) if waking needs to be quicker.

**Ready for step 4.** Each tab's document is separate, with its own ids; pasting into another tab goes through that tab's commands (one undo step), remapping ids the way importing does.

## Step 2 tests

Workflow tests in `crates/ui/src/scene_tabs/tests.rs` (a new tab; switching keeps each tab's scene, undo, selection and mode; settings shared; closing with and without changes, Cancel, Don't save and Save; the last tab; a tab being saved; quitting with the list, going through, cancelling and closing without saving, and with one tab with changes; dragging to reorder; middle-click; New and Open both ways with the setting; a project opened twice; the title and the dot; a backup copy on leaving; placement F and E; many tabs shrinking and scrolling; the Settings page), the saved options' round trip and a fuzz test, the strip's widths and drop places, the autosave timer on leaving, a second tab's backup session (`app::backups`), and every tab's emergency copy (`app::emergency`).
