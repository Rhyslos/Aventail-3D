# 0024. Saving projects: the `.avtl` file in practice

- **Status:** Accepted
- **Date:** 2026-10-11

## Problem

Decisions [0007](0007-rhy-project-format.md) and [0010](0010-avtl-file-extension.md) set the outline of the project file: a header with the `AVTL` signature, a version and flags; a serde body compressed with zstd; upgrade steps for old versions; safe saving with a backup. Building it means choosing the encoding, how the file relates to the app's own types, what a project holds, and how the interface asks before changes are lost.

## Options and decisions

**The file model is separate from the app's types.** The file's contents are plain data structures for each format version (`fileio/src/project/v1.rs`), converted to and from the scene. The alternative, adding serde to the scene and mesh types themselves, is less code today, but then renaming a field in the app would silently change the file format. With a separate model, the app's types can change freely: the format only changes with a new version and an upgrade step. A sample file saved by version 1 is kept in the repository (`fileio/tests/data/sample-v1.avtl`), and a test checks it still opens, so no future change can break old projects unnoticed.

**Encoding: postcard, compressed with zstd.** Postcard (MIT/Apache-2.0) is a compact binary encoding for serde with a stable, documented wire format. Text formats such as RON are easier to read but several times larger and slower for meshes with many points. zstd (the `zstd` crate, MIT/Apache-2.0, over the BSD-licensed library) at level 3 is fast. Meshes shrink by around a third; positions are floating-point numbers and don't compress far.

**The header** is 20 bytes: `AVTL`, the format version, the flags (bit 0 = zstd), and the body's length. Versions above the app's own are refused with advice to update. So are unknown flags, since only a newer writer would set them. Version 0, a wrong length, and data after the body are refused as damage.

**Reading never trusts the file.** Lengths are checked against a 4 GB cap before reading, and unpacking stops past 4 GB. Every value is checked before it becomes part of a scene:
- positions, rotations and recorded moves must be real numbers;
- mesh faces must name existing, distinct points, and hard edges existing points;
- parents must come before their children, and ids must be unique;
- names must not be empty, and groups must have no edits.

Primitive settings are brought within limits, as when typed. Errors name the object and say what is wrong. Fuzz tests check that cut, flipped and random bytes never crash the reader.

**What a project holds:**
- the objects, in tree order, with ids, names, visibility and transforms;
- bases (primitive settings or meshes, with deleted slots kept, so ids survive);
- stacks of recorded edits;
- the id the next object gets (ids are never reused);
- the selection.

Not saved yet:
- **Undo history.** It is optional in DESIGN.md §8, and is left for a later version of the format.
- **Camera views.** These belong to the interface, not the scene. They can join a later version.
- **Thumbnail.** Also left for a later version.

Preferences such as accent colour and units are the user's, not the project's, and stay in Settings.

**Safe saving:**
1. The file is written to `name.avtl.tmp`, synced to disk and read back to check it.
2. The previous save is copied to `name.avtl.bak`.
3. The temporary file is renamed over the project in one step.

At every moment there is a complete project on disk.

**In the interface:**
- File → New (`Ctrl + N`), Open… (`Ctrl + O`), Open recent (the last eight), Save (`Ctrl + S`) and Save As… (`Ctrl + Shift + S`). These are the keys almost every program uses.
- The title shows the project's name. *Changed 2026-10-11 at the project lead's request, as `*` was hard to see:* unsaved changes put `●` in front and "(unsaved)" after the name, and the menu bar shows the project's name with an **Unsaved** badge in the accent colour. File → Quit has no key (Ctrl + Q was removed), so a slip can't close the app.
- Before New, Open, Quit or the window's close button, unsaved changes bring a question: Save / Don't save / Cancel, with `Enter`, `Ctrl + D` and `Esc` as keys.
- "Unsaved" means the scene changed since the last save or open, so undoing back to the saved state still counts as a change. That's simple and errs on the side of asking.
- The interface never touches files. It asks the app (`WorkbenchAction::Save`/`Open`), and the app shows native dialogs through `rfd` (MIT) and calls `fileio`. On Linux, `rfd` uses the desktop portal, so no GTK is needed.
- A project path given on the command line opens at start. That's what a file association will use once there is an installer.

## Consequences

- Phase 2's gate can be checked: a scene of primitives can be built, edited, undone, saved and reopened.
- Every later change to what a project holds needs a new format version with an upgrade step from the previous one, and a new sample file.
- Saving reads the whole file back, which doubles the disk work. That's cheap at today's sizes; revisit if large projects make saving slow.
