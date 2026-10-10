# 0008. App window: eframe, egui_dock, drawn icons

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

The app needs a native window, a frame loop, and the Cinema 4D-style layout from DESIGN.md §5.1 with panels the user can drag, split and resize, all running on wgpu as chosen in [0002](0002-rust-wgpu-egui.md). Halberd solved the same problem a few days earlier.

## Options

1. **winit and wgpu wired by hand**, with egui on top: full control, but a few hundred lines of setup to own.
2. **eframe**, egui's official framework: the same winit and wgpu underneath, already wired together, plus saving window size and app state between sessions. Halberd uses it successfully.

For panels: **egui_dock** (drag, split, tab groups, saved layouts), as in Halberd.

For palette icons: **image files** (need drawing, licensing and loading) or **icons drawn in code** with egui's shapes.

## Decision

- **eframe with the wgpu renderer** owns the window and frame loop. The `app` crate implements `eframe::App`; everything drawn comes from the `ui` crate.
- **egui_dock** provides four panels: Viewport, Attributes, Objects and Coordinates. They can be rearranged but not closed or torn off into floating windows, so none can be lost. **View → Reset panel layout** restores the standard arrangement: Attributes top right, Objects below it (the project lead's preference, the reverse of Cinema 4D), Coordinates under the viewport.
- **The menu bar, command palette (top), mode palette (left) and status bar** are fixed egui panels around the dock, as in Cinema 4D. Commands that don't exist yet are shown greyed out with the phase that brings them, so the layout is complete from day one.
- **Icons are drawn in code** (`ui/src/icons.rs`): original drawings, nothing to license, sharp at any scale, and coloured by the theme.
- **The layout is saved by eframe** in its storage file (`app.ron` in the system's per-user data folder, in a folder named after the stable `APP_ID`, so a rename keeps everyone's layout). A saved layout missing a panel or containing one twice is replaced by the standard layout, with a note in the status bar.
- **Window smoke test in CI:** on every change, Linux CI opens the real window on a virtual screen with a software Vulkan driver, checks the viewport colour, clicks a mode, quits with Ctrl+Q and checks the layout was saved. Screenshots are kept as a build artifact.

Accepted in `deny.toml`, as in Halberd:

- Licenses: **BSL-1.0** (Windows clipboard library), and **OFL-1.1** and **Ubuntu Font Licence 1.0** for egui's built-in fonts. The font license texts are in `licenses/fonts/` and ship with every build.
- "Unmaintained" notices (not vulnerabilities), with no replacement upstream: `paste` (RUSTSEC-2024-0436, compile time only, via egui_dock) and `ttf-parser` (RUSTSEC-2026-0192, via winit's Linux window decorations).

## Consequences

- Little setup code to maintain. If the viewport later needs control eframe does not give, a new decision record will weigh hand-wired winit.
- The interface uses egui's default look until the theme milestone gives it the calm Cinema 4D-style colours and the Inter font.
- On Windows a terminal window opens behind the app; it will be hidden before the first public release.
- Some windowing libraries panic instead of returning an error when a system library is missing; the app catches that and explains it in plain words, with an install hint.
