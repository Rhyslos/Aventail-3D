# script

**Layer 2 · Services** (beside `fileio`)

Scripting: LustScript (`.lsc`) and Lua (`.lua`) files run safely in Luau, on a thread of their own, against a copy of the scene.

## Purpose

Lets people run their own small programs on a project (Phase 14; [decision 0299](../../docs/decisions/0299-scripting-plan.md), [decision 0301](../../docs/decisions/0301-script-runner.md), plan in [docs/plans/phase-14.md](../../docs/plans/phase-14.md)). It holds the Lust compiler as a library and `mlua` with Luau, and everything between a script's text and what it says:

- **`header`**: `read_header`, the comment lines at a script's top (`-- name:`, `about:`, `api:`, `count:`, `asks:`), read as text, never run; lines it can't read are noted with their line.
- **`describe`**: the one table of every call (`CALLS`, with `OWNERS`, `CLASSES`, `STATES`, `API_VERSION`), and what's made from it: the description files (`descriptions`, `aventail/scene.lsc` and the rest, whose `native` methods Lust checks every call against) and the manual's reference page (`reference_page`; `crates/app/tests/script_reference.rs` fails when `docs/manual/script-reference.md` is out of date).
- **`sandbox`** and **`prelude.lua`**: a fresh Luau per run, without `io`, `os`, `debug`, `require`, `loadstring`, `getfenv` and `setfenv`, text always loaded as text, libraries and the app's global read-only; mlua's safe interrupt for the time limit and Stop, sticky once past (so `pcall` in a loop can't hold it off); the memory limit; Lua's `command(…)`, `print` into the output, shapes as read-only values, one per shape, and the builders of a chunk of points or polygons (`polygons_from`, `points_from`: `natives` packs a chunk into a Luau buffer, and the tables are made in Luau, several times quicker than making each one from Rust).
- **`turns`** and **`compile`**: `check` and the run's checking, on a thread of their own, one at a time; a check past `CHECK_LIMIT` (30 s) is given up on and none starts until it ends; a compiler crash is a mistake.
- **`natives`** and **`values`**: the bodies of the calls (`LUST_NATIVES`, which Lua's `aventail.` reaches too), each value read one at a time so a wrong one is a mistake naming the call and the value; lists (ids, edges, vec3s, polygons' corners, shapes) at most `MAX_LIST` items. The reading calls read the run's own copy of the document.
- **`changes`**: the changing calls. Each makes a `scene::Change` (a command, a recorded operation built as the tools build it, or a selection change), carries it out at once on the run's copy and keeps it; a refused change stops the script and spoils the run (nothing lands, even if caught). Limits `MAX_CHANGES`, `MAX_BUILD`, `MAX_CORNERS`, `MAX_BUILT`.
- **`tokens`**: the pieces of a script's text for colouring it (`tokens`, `Token`, `TokenKind`), a small reader of our own for both languages that never fails.
- **`pieces`**: `only_lines` (the selected lines, ready to run, line numbers kept) and `console_script` (a console line, with every call fetched in LustScript).
- **`library`**: your script library on disk (`Library`: `prepare` writes the description files, `scripts`, `create` from `starter`, `add` (installing, a number added rather than writing over yours), `rename`, `remove`; `file_name`, `save`, `stamp`), saving through `fileio`.
- **`access`** and **`files`**: what a run may reach outside the project (`Access`, from the app: a folder and whether it may read and write), every path checked (`place`: inside the folder, no `..`, whole paths, `:`, links, reserved or over-long names), what a run writes held (`Writes`) until it ends well and then written whole; `Project:readText`, `writeText` and `exportShapes` (with the `Preset` state, through `fileio::export`). `fingerprint`: a script's SHA-256, for trusting it.
- **`commands`**: a script's commands and their settings (`commands_of`, run on the caller's thread: the start run once in a fresh sandbox against an empty scene, at most `DESCRIBE_LIMIT`, nothing kept; `Described`, `CommandInfo`, `Setting`, `SettingKind`, `SettingValue`, `words_of`), and setting chosen values on a command for `Run::start_command` (`Choice`).
- **`moments`**: scripts that run by themselves (decision 0309): `Moment` (`ProjectOpened`, `ProjectSaved`, `ExportStarting`, Lust events declared in `aventail/moments.lsc` from `describe::HAPPENINGS`), what reacts to one (a component's `on`, a Lua command's `on_` function), `ExportInfo` and the `Export` calls (`shapes`, `format`, `cancel`; the `Format` state), `MOMENT_LIMIT` (2 s). `Run::start_moment` runs them.
- **`examples`**: the six example scripts (`examples`), kept as `.lsc` files in `src/examples/` with `%folder%` where the scripting folder's name goes.
- **`problem`**: `Problem` (file, line, column, message), Lust's own messages, and Lua errors turned back into the script's own file and line.
- **`output`** and **`run`**: `Run::start` (and `Run::start_command`, one command with chosen settings) (a `ScriptFile` against a `Snapshot`, with `Limits` and `Names`), its `Event`s (lines said, warned and noted, mistakes, progress, `Land` with the run's `Landing`, the `Ending`), `stop` (also when the `Run` is dropped), `ending_words`.

The scripting folder's name and who made the description files come in `Names`, from the app's branding (`SCRIPT_NAME`): the app's name is never written here.

## Must never

- Let a script reach the network or other programs, or files outside the one folder its approved permission names (`Access`); write anything for a run that doesn't end well
- Run anything while reading a header, or while checking
- Crash the app on any script: every failure is a `Problem` or an `Ending`
- Change the scene: scripts work on a copy, and what they did comes back only as `scene::Change`s in a `Landing`, which the app carries out as one undo step
- Use `unsafe` code (decision 0005): the time limit uses mlua's safe interrupt
- Know about egui or the GPU

## Depends on

`scene` (the copy a script reads), `mesh` (points, polygons, edges, selections), `words` (the language layer), `fileio` (saving library scripts and what scripts write whole or not at all; exporting shapes). Outside libraries: `lust` (MIT, from its repository at a release tag, without default features: Rust's own library only), `mlua` with Luau (MIT; Luau is built from C++ source, so the build needs a C++ compiler), `glam`, `sha2` (MIT or Apache-2.0, a script's fingerprint).

## Status

**Milestone 1, the script runner** ([decision 0301](../../docs/decisions/0301-script-runner.md)): version 1's reading calls (`Scene`, `Shape`, `Selection`, `Project`) and `Script`'s (`api`, `say`, `warn`, `progress`); a file's top-level code runs, then each command it makes once with its starting settings. 10 s and 256 MB a run (`Limits`), 1,000 printed lines of 4 KB (`MAX_LINES`, `MAX_LINE_BYTES`), chunks of 100,000 (`MAX_CHUNK`), files of 1 MB (`MAX_SCRIPT_BYTES`). Messages from `prelude.lua` are in English; every message from Rust goes through `words`.

**Milestone 2, changing the scene** ([decision 0302](../../docs/decisions/0302-scripts-change-the-scene.md)): version 1's changing calls (`Scene:add`, `build`, `group`; a shape's `setName`, `setPosition`, `setRotation`, `setScale`, `show`, `hide`, `delete`, `bevel`, `extrude`, `inset`, `markHard`, `markSeams`, `unwrap`, `setMaterial`, `makeEditable`; `Selection:set`, `setPoints`, `setEdges`, `setPolygons`, `clear`) and the `Primitive` state; 100,000 changes a run.

**Milestone 3, the Scripts panel** ([decision 0303](../../docs/decisions/0303-scripts-panel.md)): `tokens`, `pieces` and `library` for the panel's editor, console line and script list; `describe::fetch_everything`.

**Milestone 5, keeping, sharing and permissions** ([decision 0307](../../docs/decisions/0307-keeping-sharing-and-permissions.md)): `access` and `files` (`Project:readText`, `writeText`, `exportShapes`, `Preset`; `MAX_READ_BYTES` 16 MB, `MAX_WRITES` 1,000, `MAX_WRITE_BYTES` 256 MB, `MAX_PATH_BYTES`, `MAX_NAME_BYTES`, `MAX_DEPTH`), `Snapshot::access`, `fingerprint`, `Library::add`. Tests: `access_tests.rs` (every path trick, links out and in, 3,000 fuzzed paths, reading and writing inside the folder, writes refused without permission with nothing written, an unsaved project, failing and timed-out runs writing nothing, caps, a chosen folder, exports with presets in both languages and refused ones), `library_tests.rs` (installing beside your own, fingerprints).

**Milestone 6, scripts that run by themselves and examples** ([decision 0309](../../docs/decisions/0309-event-scripts-and-examples.md)): `moments`, `examples`, `Run::start_moment`, `Snapshot::{moment, export}`, `Event::Cancelled`, `Described::moments`, `Mode::Sculpt`; a scene change refused while an export starts. Tests: `moments_tests.rs` (reacting in both languages in name order, settings chosen, reading and stopping an export, no scene changes then, the `Export` calls refused elsewhere, 2 s, files kept only when the export goes on, 40 fuzzed scripts), `examples_tests.rs` (each example checks and does what the manual says).

**Milestone 4, scripts as commands** ([decision 0305](../../docs/decisions/0305-scripts-as-commands.md)): `commands` and `Run::start_command`; `Library::rename` and `remove`; a note for scripts written for an older scripting version.

Tests: `commands_tests.rs` (every kind of setting read and set in both languages, wrong kinds and settings gone keep their start, only the chosen command runs, no commands, two of one name, reserved names, mistakes, newer versions and slow starts, reaching no scene, 60 fuzzed settings tables), `tokens_tests.rs` (every kind of token in both languages, unfinished text and comments, 3,000 fuzzed texts, 10,000 lines), `pieces_tests.rs` (selected lines and console lines run, land and place their mistakes), `library_tests.rs` (description files written once and again when changed, starters that run in both languages, names refused, 3,000 fuzzed names, listing, saving), `changes_tests.rs` (every changing call in both languages landed on a held document ending as the copy did, through undo and redo; each call alone; fuzzed changes; bad generators and a million polygons; missing elements; empty lists; failing, refused, caught, stopped and too-big runs landing nothing; millions of shapes; landing 10,000 changes), `header_tests.rs` (every line, case and byte-order mark, notes, long values, 3,000 fuzzed headers), `describe_tests.rs` (the table, the description files checked by Lust on their own, another folder name, the reference lists every call), `sandbox_tests.rs` (read-only libraries and global, own globals per script, the sticky clock, Stop), `natives_tests.rs` (every call has a body, chunks), `run_tests.rs` (reading in LustScript and Lua, every call in both, commands, empty and `end`-less scripts, placed mistakes, versions, endless loops and recursion, `pcall` and `xpcall` loops, memory bombs, deep nesting, reaching outside the sandbox, nothing left between runs, Stop, printing too much, 150 fuzzed scripts, the manual's examples, a benchmark of a small script), `turns.rs` and `output.rs` (their own).
