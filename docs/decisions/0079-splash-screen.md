# 0079. The splash screen

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** the project lead's request (DESIGN.md §13, 2026-10-13)

## Problem

Until its first frame, the app's window was empty, or not there at all:

- the graphics were being set up;
- settings read;
- a project opened;
- backups checked.

On a slow computer, or with a big project, that looks like a hang. The project lead asked for a splash screen that shows the app's name and version, and what it is loading.

## Options

- **A separate window before the main one:** an extra window to create and close; flickers on some systems.
- **The main window's first frames show the splash, and the app starts up one step between frames** (chosen). One window, and each step's name is on screen while that step works.
- **Look:**
  - original artwork;
  - **the app's own cube icon in the accent colour, the name, the version, a thin progress bar and the step's words, on the dark background** (chosen). Calm, quick to draw, and nothing copied.
- **How long:** only as long as starting takes, or **at least 0.6 s** (chosen), so a quick start doesn't just flash it. The steps themselves aren't slowed.

## Decision

1. The steps, in order, each named on the splash while it works:
   - *Starting the graphics…*: the viewport's renderer on the window's GPU;
   - *Getting the workbench ready…*: the remembered settings;
   - *Opening crate.avtl…*: only with a project given at start;
   - *Checking for unsaved work from last time…*: backups and crash recovery.
2. `ui::splash::show` draws it; the app (`window::Shell`) runs one step a frame after the frame naming it has been shown, then switches to the running app.
3. Reading settings is fast and happens before the first frame, so the splash already uses the accent colour picked in Settings.
4. Closing the window while starting is allowed. Settings aren't saved then, since none have changed.

## Consequences

- A slow start always says what it is doing.
- Later start-up work (scripts, plugins) adds a step with its own words.
