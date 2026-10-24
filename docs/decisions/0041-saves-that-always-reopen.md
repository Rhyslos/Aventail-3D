# 0041. Saves that always reopen

- **Status:** Accepted; the save now runs on another thread ([0163](0163-saving-and-exporting-in-the-background.md)). The check itself stands.
- **Date:** 2026-10-13
- **Follows:** [0024](0024-saving-projects.md), [0036](0036-autosave.md); from the [whole-app review](../reviews/2026-10-13-whole-app-review.md)

## Problem

The whole-app review found two ways a session could save a project that the app then refused to open:
- a bevel on back-to-back polygons recorded a "hard edge" from a point to itself;
- moving points of a shape scaled to 0 recorded a move that wasn't a number.

In both cases the save itself succeeded, so the project, and every backup copy written after it, was lost without warning. The cause underneath was that saving checked the bytes reached the disk, but never that the app could read them.

The review also found that saving wrote through a fixed temporary name and copied onto the backup's name. Links planted beside a project (from a zip or a git checkout) could therefore redirect the save onto another file.

## Decision

1. **Every save and backup copy is read back with the real loader before it replaces anything** (`project::to_bytes`).
   - If it wouldn't open, saving fails with a plain message (`ProjectError::WouldNotReopen`) and the previous save is untouched.
   - The unreadable bytes go to `name.avtl.unreadable` for a bug report.
   - Saving also fails, instead of quietly leaving the edit out, if an edit is of a kind the file format can't store.
2. **One safe-write helper** (`project::safe_write`) for every file the app writes. It:
   - creates a new, uniquely named temporary file beside the target (never one that already exists, so no link is followed);
   - writes, syncs and reads it back;
   - renames it over the target (which replaces a link rather than writing through it);
   - syncs the folder on Linux and macOS.

   The backup is made the same way, rather than copied onto its name. Exports and settings will use it too.
3. **Close the gaps at the source:**
   - `Mesh::set_hard` ignores a point joined to itself and points that don't exist.
   - `Command::Record` and `ReplaceOperation` refuse edits holding numbers that aren't real (`Operation::numbers_are_real`).
   - The gizmo offers no points to move on a shape scaled flat.
   - The Coordinates panel ignores a size of 0.
4. **Open leniently where it's harmless.** A hard edge that names a missing point, or a point twice, only changes shading, so it is left out when opening instead of refusing the whole project.

Also fixed from the review:
- huge polygons are checked for repeated corners without comparing every pair;
- object ids from a file can't be large enough to make the id counter wrap;
- backup copy numbers can't overflow.

## Consequences

- Saving costs one extra read of the project from memory, small next to compressing it. On a 3-million-polygon project it adds a little time.
- A bug that would have lost a project now shows as a failed save with a message, and the work is still in the open window, so it can be saved another way (Save As after undoing the last step) and reported.
- The rest of the review is the remaining Foundations work: faster edits, room for UVs and materials, and limits on hostile files.
