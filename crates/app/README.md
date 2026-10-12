# app

**Layer 4 · App**

The application itself: wires every crate together, opens the window and owns the frame loop.

## Purpose

Starts the program: loads settings, opens the window, hands each frame to `ui`, connects the renderer, shows file dialogs and calls `fileio`. Also home to `branding.rs`, the **only** place in code that names the app (see RENAMING.md).

## Must never

- Contain feature logic: every feature belongs in a lower crate
- Name the app anywhere except through `branding`

## Depends on

`ui`, `render`, `fileio`, `scene` (for the backup copy's document) (every other app crate planned). Outside libraries: `eframe` (window and frame loop), `windows-sys` and `raw-window-handle` (Windows only), `rfd` (native Open and Save dialogs; the desktop portal on Linux).

## Status

- **`backups`** ([decision 0036](../../docs/decisions/0036-autosave.md)): `folder()` is where backup copies go (`%LOCALAPPDATA%\<APP_ID>\autosave` on Windows, `eframe::storage_dir(APP_ID)/autosave` elsewhere). `Backups::start` offers what crashed runs left behind to the workbench (`offer_recovery`) and starts this run's `fileio::autosave::Session`; `write` writes the workbench's `Autosave` copy on another thread; `update` (each frame) reports a finished copy (`autosaved`, `autosave_failed`) and notes the open project; `restore` opens a copy, adopts it into this session and hands it to the workbench (`recovered`); `discard` deletes a leftover; `end` (on a normal close, `eframe::App::on_exit`) waits for a copy being written and deletes this run's folder. Tests: a crash and restore across two runs, discarding, a damaged copy, and no folder at all.
- **`branding`**: the app's name, short name, settings ID, family and project file extension.
- **`cli`**: `--version` and `--help`; unknown options are explained (exit code 2).
- **`gpu`**: `GpuViewport` connects the `render` crate to the window's GPU device and registers its images with egui: one per view (`ViewportView::slot`), so the four-view layout shows four images. Passes flat views' height and grid plane on to the renderer. Keeps each object's mesh on the GPU ([decision 0035](../../docs/decisions/0035-heavy-meshes.md)): a mesh that is the same isn't touched, one whose points only moved is updated around them (`render::MeshGeometry::moved`), anything else is uploaded whole, and a heavy new one (50 000 polygons or more) is worked out on another thread while the view says so (`ViewportRenderer::preparing`). Each frame it places the objects (`mesh_draws`: edges in the accent when selected, plain while edited, points in the Points mode) and leaves out points and edges too dense to see from that view (`leave_out_dense`).
  It also turns the workbench's `ComponentOverlay` into a `render::Overlay` (`overlay_geometry`: plain points and lines in soft white, hovered ones larger and thicker in the accent colour, hovered polygons tinted) and uploads it when its version changes.
  Selected elements are drawn in the accent colour (larger points, thicker edges, tinted polygons); the hovered one in a lighter accent (`HOVER_LIFT`).
- **`project`**: carries out the workbench's Save and Open actions: native dialogs (`rfd`, filtered to the app's project files, `.avtl` added if left out), `fileio::project::save`/`open`, and reporting back (`saved`, `save_failed`, `save_cancelled`, `opened`, `open_failed`).
- **`cli`**: `--version`, `--help`, and a project path to open at start (as a file association will pass it).
- **`platform`**: small fixes for one system. `keep_keyboard_focus` runs each frame; on Windows it hands the window the keyboard when it is active but nothing holds it (the cause of the error sound on key presses), using `windows-sys` — the one module allowed `unsafe` ([decision 0026](../../docs/decisions/0026-windows-keyboard-focus.md)). Elsewhere it does nothing.
- **`window`**: opens the window with eframe and wgpu, hands each frame to the `ui` workbench, carries out Quit, asks the workbench before the window's close button closes it with unsaved changes, keeps the title (project name; `●` and "(unsaved)" for unsaved changes) up to date, and saves the panel layout, view options, accent colour, units (metres or centimetres), undo memory limit, recent projects and autosave settings between sessions under the stable `APP_ID`. Carries out the workbench's `Autosave`, `Restore` and `DiscardBackups` actions through `backups`, and asks for a repaint after any action so what it did (an opened or restored project) shows at once. A panic from a missing system library is caught and explained with an install hint. View options saved include the snapping settings, axis locks and world or object axes.
