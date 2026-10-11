# app

**Layer 4 · App**

The application itself: wires every crate together, opens the window and owns the frame loop.

## Purpose

Starts the program: loads settings, opens the window, hands each frame to `ui`, connects the renderer, shows file dialogs and calls `fileio`. Also home to `branding.rs`, the **only** place in code that names the app (see RENAMING.md).

## Must never

- Contain feature logic: every feature belongs in a lower crate
- Name the app anywhere except through `branding`

## Depends on

`ui`, `render`, `fileio` (every other app crate planned). Outside libraries: `eframe` (window and frame loop), `windows-sys` and `raw-window-handle` (Windows only), `rfd` (native Open and Save dialogs; the desktop portal on Linux).

## Status

- **`branding`**: the app's name, short name, settings ID, family and project file extension.
- **`cli`**: `--version` and `--help`; unknown options are explained (exit code 2).
- **`gpu`**: `GpuViewport` connects the `render` crate to the window's GPU device and registers its images with egui: one per view (`ViewportView::slot`), so the four-view layout shows four images. Passes flat views' height and grid plane on to the renderer, and, whenever the scene drawing's version changes, turns its meshes into one GPU shape (`render::Shape::from_meshes`) and uploads it.
- **`project`**: carries out the workbench's Save and Open actions: native dialogs (`rfd`, filtered to the app's project files, `.avtl` added if left out), `fileio::project::save`/`open`, and reporting back (`saved`, `save_failed`, `save_cancelled`, `opened`, `open_failed`).
- **`cli`**: `--version`, `--help`, and a project path to open at start (as a file association will pass it).
- **`platform`**: small fixes for one system. `keep_keyboard_focus` runs each frame; on Windows it hands the window the keyboard when it is active but nothing holds it (the cause of the error sound on key presses), using `windows-sys` — the one module allowed `unsafe` ([decision 0026](../../docs/decisions/0026-windows-keyboard-focus.md)). Elsewhere it does nothing.
- **`window`**: opens the window with eframe and wgpu, hands each frame to the `ui` workbench, carries out Quit, asks the workbench before the window's close button closes it with unsaved changes, keeps the title (project name; `●` and "(unsaved)" for unsaved changes) up to date, and saves the panel layout, view options, accent colour, units (metres or centimetres), undo memory limit and recent projects between sessions under the stable `APP_ID`. A panic from a missing system library is caught and explained with an install hint.
