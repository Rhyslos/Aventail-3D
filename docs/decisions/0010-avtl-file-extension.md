# 0010. Project files use `.avtl`

- **Status:** Accepted (replaces the extension chosen in [0007](0007-rhy-project-format.md); the rest of 0007 stands)
- **Date:** 2026-10-10

## Problem

Decision 0007 named project files `.rhy`, after Rhylib, so the extension would survive a rename of the app. Once the name Aventail 3D was settled, the project lead wanted the extension to match the app instead. Nothing has been saved in any format yet, so changing it now costs nothing.

## Options

1. **Keep `.rhy`:** tied to the Rhylib family rather than this app; could clash if Halberd ever saves its own projects.
2. **`.avt`:** short, but already used by CLO 3D and Marvelous Designer for avatar files. Marvelous Designer is popular with game artists, so double-clicking could open the wrong program on their machines.
3. **`.avtl`:** short for Aventail, with no known use by other software.
4. **`.aventail`:** unmistakable, but long to type and to read in file lists.

## Decision

`.avtl`. The file signature (magic bytes) changes to `AVTL` to match. The extension lives in `PROJECT_EXTENSION` in `crates/app/src/branding.rs`, with a test that stops it from being changed by accident.

## Consequences

- Project files are clearly Aventail's, and `.rhy` is free for Halberd or other Rhylib tools.
- If the app is ever renamed, `.avtl` stays: changing an extension breaks double-click opening for everyone's existing projects, and many programs keep an old extension after a rename (see RENAMING.md).
