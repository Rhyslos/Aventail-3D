# app

**Layer 4 · App**

The application itself: wires every crate together, opens the window and owns the frame loop.

## Purpose

Starts the program: loads settings, opens the window, hands each frame to `ui`, connects the renderer, shows file dialogs and calls `fileio`. Also home to `branding.rs`, the **only** place in code that names the app (see RENAMING.md).

## Must never

- Contain feature logic: every feature belongs in a lower crate
- Name the app anywhere except through `branding`

## Depends on

Every other app crate (planned).

## Status

A program skeleton that reports its name and version. The window arrives later in Phase 0.
