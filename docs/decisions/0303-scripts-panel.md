# 0303. The Scripts panel: editor and console (Phase 14, milestone 3)

- **Status:** Accepted, built while the project lead was away; its further choices are listed for review in [docs/plans/phase-14.md](../plans/phase-14.md#choices-to-review) (30 to 37), with the plan's own
- **Date:** 2026-11-17
- **Follows:** [0299](0299-scripting-plan.md) (the scripting plan, choices 10, 11, 12 and 17), [0301](0301-script-runner.md) (the script runner), [0302](0302-scripts-change-the-scene.md) (scripts change the scene)

## Problem

Milestone 3 of scripting: write and try scripts inside the app. The Scripts panel grows from an output into a place to work: your scripts on the left, an editor that colours code, numbers its lines and underlines mistakes as you type, the output, and a console line for one line at a time. Scripts must also be easy to edit in another program, reloading when their file changes, and the description files (`aventail/*.lsc`) must be on disk so such a program knows the app's calls. Typing must stay smooth on long scripts, checking must never run the script, and nothing typed may be lost.

## Options

1. **Colouring:** (a) ask the Lust compiler for its tokens (it has no public lexer, and Lua would need another); (b) a small reader of our own in the `script` crate, for both languages, that never fails (`script::tokens`).
2. **Checking while typing:** (a) on the interface thread after each key (a long LustScript takes milliseconds to seconds to check); (b) on a thread of its own, a short pause after typing stops, through the same one-at-a-time checking runs use (`script::check`), the newest result shown.
3. **Laying out long scripts:** (a) give egui the whole coloured text each frame (it hashes and copies the whole job every frame, even unchanged); (b) keep the laid-out text until it changes, and when it does, colour and lay out again only the lines that changed, put together with egui's own `Galley::concat`.
4. **Saving:** (a) Ctrl + S saves the script while typing in the editor; (b) scripts save themselves a second after typing stops, when another is opened, when run, and when the editor closes; Ctrl + S keeps meaning save the project (plan, choice 11).
5. **A file changed by another program:** (a) reload always; (b) reload when there's no typing waiting to be saved; while there is, ask which to keep.
6. **Running part of a script (Ctrl + Enter with a selection):** (a) only the selected text, as is; (b) the selected lines, every other line left blank so mistakes keep their line numbers, with the header and LustScript's `fetch` lines kept and `end` added.
7. **The console line:** (a) one Luau kept for the whole session, so variables carry from line to line (against the one-fresh-sandbox-per-run rule and the copy-per-run model); (b) each line a run of its own, landing as one undo step.

## Decision

- **Our own reader** (1b): `script::tokens(text, language)` gives keywords (Lust's reserved words, Lua's and Luau's), names starting with a capital, other names, numbers, text, comments and symbols, by byte range; unfinished text ends at its line, an unclosed block comment or long string at the text's end. The editor colours them with `theme::code`, every colour readable on the field (a test checks the contrast).
- **Checked on its own thread** (2b): 400 ms after typing stops, `script::check` on a thread of its own; never runs the script. Mistakes in the file are underlined (the word, text in quotes, or the whole line when no column is known); the first one is named over the editor, and clicking it, or a mistake in the output, goes to its line.
- **Lines laid out once** (3b): `scripts_panel::code_view::Coloured` keeps the laid-out text until the text, its mistakes, the font or the screen's scale change (or egui's glyph atlas is made again); each line is kept by what it holds and looks like, so a key press lays out one line. Cuts between colours are merged from lists already in order rather than sorted. In a debug build a key in a 10,000-line script takes about 0.18 s a frame, an unchanged frame about 0.06 s (the whole window).
- **Scripts save themselves** (4b), whole or not at all (`fileio::project::safe_write`), and say once if saving fails.
- **Changed outside** (5b): the file is looked at once a second (one look at its size and time); with no typing waiting it's loaded again and the output says so; with typing waiting, a bar asks **Load theirs** or **Keep mine**, and nothing is written over theirs meanwhile. A file removed outside stays open and is written again.
- **Selected lines** (6b): `script::only_lines`; the run is named "Bevel hard edges (lines 4 to 6)". The **Run** button and Ctrl + Enter without a selection run the whole script.
- **Console lines are runs of their own** (7b): `script::console_script` puts a scripting version line and, in LustScript, every description file's names fetched (`describe::fetch_everything`) before what was typed, and `end` after; mistakes are shown at the typed line. The console's language follows the script opened and can be changed beside it. Each line lands as one undo step called "Console line". **Enter** runs, **Shift + Enter** adds a line, **Up** and **Down** (from the first and last line) go through the newest 100 lines, giving back what was being typed past the newest.
- **Your library** (plan, choice 12): `scripts` in the app's own data folder (`AppInfo::script_library`), listed on the left (`script::library::Library`, at most 1,000, looked at every 2 s while shown). **+** makes a new LustScript or Lua script from a starter that names the selected shapes; its file is named as typed (names a file can't have on any system are refused, with why). Double-click edits; right-click: Edit, Run, Open in your own editor, Show the folder. The description files are written into `aventail/` there (again when they differ).
- **Open in your own editor**: `WorkbenchAction::OpenOutside`, which the app opens with the computer's own program for the file (`explorer`, `open` or `xdg-open`, the path handed over without a shell).
- **Window → Scripts** and **F11** show or put away the panel; shown beside Coordinates, its band is made 320 px tall (`layout::make_room_for_scripts`), and put away, Coordinates goes back to its slim band.

## Consequences

- Writing, trying and fixing a script happens in one place; nothing typed is lost (saved within a second, and when the editor closes).
- Checking a big LustScript never holds up typing; mistakes may show a moment late, and stay shown until the next check comes back.
- Running selected lines inside a LustScript `component` or `loop` runs only those lines, which may not make sense alone: the mistake says so at its line.
- Variables don't carry from one console line to the next (choice 31); a few lines typed together with Shift + Enter run as one.
- No new file format (111). The keys added: F11 (anywhere), Ctrl + Enter (in the editor), Enter, Shift + Enter, Up and Down (on the console line); Ctrl + Z and Ctrl + Y undo typing in either, as in every text box.
