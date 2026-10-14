# 0045. Limits for files, and a size guard for shapes

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0024](0024-saving-projects.md), [0036](0036-autosave.md), [0041](0041-saves-that-always-reopen.md); from the [whole-app review](../reviews/2026-10-13-whole-app-review.md) (S1, S2, S4, S5)

## Problem

Project files can come from anywhere. The review found that a hostile or damaged file could:
- **run the app out of memory** (S1). A 4.6 KB file unpacked to 4 GB. Nothing limited how many objects, points, polygons or edits a file could hold, and lists could claim far more items than the file had bytes for.
- **freeze it** (S2). Replaying recorded edits had no budget: an 83-byte file took 11 s, and a few kilobytes could take hours.
- **trap recovery in a loop** (S5). After such a crash, recovery offered the same copy again, and restoring it crashed again.

Smaller loose ends (S4):
- backup folders readable by other users;
- two copies of the app starting at the same moment could take each other's new session for a leftover and delete it;
- a file name starting with `-` couldn't be opened from the command line.

## Decision

1. **One limits rulebook** (`fileio::limits`), applied while a file is read, so nothing is allocated before it is checked:
   - **unpacked body:** at most 512 MB. A body that says up front it is larger is refused without unpacking. Before, the limit was 4 GB.
   - **memory budget:** every list in the file model is a `Bounded` list. It saves exactly like a plain list, so old files are unchanged. While it is read, each item is paid for from a 2 GB budget for the whole file, before room is made for it. A list claiming more items than the budget allows is refused at once.
   - **counts**, checked before anything is built:
     - up to 100,000 objects;
     - up to 100,000 recorded edits;
     - about 30 million polygons before edits (plain meshes as stored, primitives estimated from their settings with `Primitive::polygon_estimate`, which never under-counts).

   Hitting a limit gives a plain message ("the file is damaged: it holds more than the app can open…"), never a crash. The limits are far above anything made in the app: the heavy-mesh check's 3-million-polygon project unpacks to about 90 MB, and opens in the same 0.6 s as before.
2. **A size guard for shapes** (`scene::MAX_SHAPE_POLYGONS`, 30 million).
   - Before an extrude, inset or bevel is applied, an upper bound on what it adds is worked out: corners for extrude and inset, edges times rows for bevel.
   - If the shape would pass the guard, the edit is skipped and the Attributes panel says so in amber.
   - This holds for edits from files and from the user alike: a 64-row bevel on every edge of a very dense shape can no longer freeze the app.
3. **Recovery can't loop.**
   - Before a backup copy is restored, a note naming it is left in its folder, and removed once the restore finishes.
   - If the note is still there at the next start, restoring that copy closed the app, so it isn't offered again. The file is kept, and the question says one copy was left out.
4. **Small fixes:**
   - backup folders are made readable only by their owner (Linux and macOS);
   - a session folder without its lock file is left alone for its first minute (an app may be starting);
   - `--` ends the command-line options, so a file name may start with `-`.
5. **Fuzz tests.** These must be refused or opened, never crash, and finish within seconds:
   - well-formed files with random nonsense inside (ids pointing nowhere, corners naming missing points, numbers that aren't real, huge settings, repeated edit ids, mesh layers for missing objects), with every shape that opens also worked out;
   - files that unpack to too much, with and without their size written up front;
   - existing tests for cut, flipped and random bytes.

## Consequences

- A project from a stranger can at worst be refused with a message or show a skipped-edit warning. It can't take the computer down with it.
- A very large project, beyond what the app can work with (over 30 million polygons), won't open. That is far beyond game assets; the limits can be raised if ever needed.
- Unpacking a hostile body can still take up to 512 MB for a moment before it is refused, when the file doesn't state its size. Reading straight from the compressed stream would avoid that; not worth it now.
- Working out shapes on another thread, with a way to cancel (review A8), stays planned for before Phase 7. Until then the guard keeps the worst case bounded.
