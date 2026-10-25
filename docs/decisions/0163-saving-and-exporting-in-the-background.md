# 0163. Saving and exporting in the background

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-25
- **Builds on:** [0036](0036-autosave.md) (backup copies already save on another thread), [0041](0041-saves-that-always-reopen.md) (a save is read back before it replaces the file), [0076](0076-engine-export-paths.md) (exporting into a game project on every save), [0078](0078-kit-export.md) (kit export), [0142](0142-modifier-stack-and-remesh.md) (modifiers worked out for an export), [0033](0033-heavy-mesh-safeguard.md) (the heavy-mesh check)
- **Replaces part of:** [0041](0041-saves-that-always-reopen.md), where saving waited on the window's thread

## Problem

A review of Phase 10 measured saving and exporting on the window's thread: the window froze until they were done. Saving reads the whole file back to check it opens (0041), 3.4 s for a 3-million-polygon project; exporting a million-polygon shape to glTF took 3 s, and 40 subdivided objects 12.5 s. Exporting also built every object's shape twice, once in the world and once round its pivot, though each format writes only one.

## Options

1. **Do the work on another thread, on a copy of the document (chosen),** as backup copies already are (0036). The copy is quick: shapes are shared, never copied, and the scene's worked-out results (edits, booleans, modifiers, collision, levels of detail) come with it (`Scene::copy_with_results`), so nothing is worked out twice.
2. Keep the window's thread, with a "Saving…" note drawn first: honest, but the window still freezes.
3. Save without reading the file back: quicker, but gives up 0041's promise that a save always reopens.

## Decision

- **Only what the format needs** (`fileio::export::Frames`): glTF builds each object round its pivot only (`local`), OBJ in the world only (`world`); `ExportFormat::frames` says which, `prepare_for` and `prepare_piece` take it, and the other is left empty. The files written are byte for byte the same; a million-polygon glTF export went from 3.2 s to 1.9 s.
- **Save, Save As, Export, Export again, Export to game project and Export kit run on another thread** (`app::file_work::FileJobs`), each with a copy of the document taken when it starts. A save still reads its file back before it replaces the old one (0041), on that thread. The export into a game project that follows every save (0076) runs after the save, on the same thread.
- **The status bar says what is being done** ("Saving crate.avtl…", "Exporting crate.glb…") with a small turning wheel, until it reports "Saved crate.avtl." or why it couldn't (`Workbench::file_work_started`, `file_work`). There is no percentage: neither job knows how far it has got.
- **One at a time.** While one runs, another save, export, open, import, restore or new project waits: it isn't started, and the status bar says "… Try again when it's done." (`Workbench::hold_file_actions`, `file_work_refused`). Anything waiting for that save (a new project after "Save changes?") is called off.
- **Editing during a save is allowed.** The save holds the project as it was when it started; it's reported as saved at that revision (`Workbench::saved_at`), so changes made meanwhile still mark the project unsaved, and the note adds "Changes made while it was saving aren't in it yet."
- **Closing waits.** Closing the window or File → Quit while a save or export runs keeps the window open, saying "… The app closes when it's done."; when it's done the window closes (asking about any changes made during the save first). If it failed, the window stays open so the reason is seen. A close the window can't refuse still finishes the work first (`App::on_exit`).

## Choices to check

Made for the project lead (decision 0028):

- A second save or export while one runs is **refused with a note**, not queued: try again when the turning wheel stops.
- Edits made while a save runs **aren't in that save**; the title keeps its unsaved dot.
- Closing during a save **waits and then closes**; a failed save or export keeps the window open.
- No new keys or mouse actions.

## Consequences

- The window keeps drawing during saves and exports, however big the project; the heavy-mesh check's save row measures the work itself, now off the window's thread.
- Opening a project still happens on the window's thread (a new document can't be used before it is read).
- The manual's Saving and opening and Export pages say so.
- Tests: `app/src/file_work.rs` (a save reported when done, changes made meanwhile stay unsaved, a failed save keeps the window open, closing waits for an export, finishing waits), `ui/src/workbench/file_work_tests.rs`, `fileio/src/export/frames_tests.rs` (each format's file is the same from its own placing alone), `scene/src/results_copy.rs`.
