# app

**Layer 4 · App**

The application itself: wires every crate together, opens the window and owns the frame loop.

## Purpose

Starts the program: loads settings, opens the window, hands each frame to `ui`, connects the renderer, shows file dialogs and calls `fileio`. Also home to `branding.rs`, the **only** place in code that names the app (see RENAMING.md).

## Must never

- Contain feature logic: every feature belongs in a lower crate
- Name the app anywhere except through `branding`

## Depends on

`ui`, `render` (every other app crate planned). Outside library: `eframe` (window and frame loop).

## Status

- **`branding`**: the app's name, short name, settings ID, family and project file extension.
- **`cli`**: `--version` and `--help`; unknown options are explained (exit code 2).
- **`gpu`**: `GpuViewport` connects the `render` crate to the window's GPU device and registers its images with egui: one per view (`ViewportView::slot`), so the four-view layout shows four images. Passes flat views' height and grid plane on to the renderer, and, whenever the scene drawing's version changes, turns its meshes into one GPU shape (`render::Shape::from_meshes`) and uploads it.
- **`window`**: opens the window with eframe and wgpu, hands each frame to the `ui` workbench, carries out Quit, and saves the panel layout between sessions under the stable `APP_ID`. A panic from a missing system library is caught and explained with an install hint.
