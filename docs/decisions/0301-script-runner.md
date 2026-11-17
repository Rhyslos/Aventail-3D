# 0301. The script runner (Phase 14, milestone 1)

- **Status:** Accepted, built while the project lead was away; its further choices are listed for review in [docs/plans/phase-14.md](../plans/phase-14.md#choices-to-review) (18 to 22), with the plan's own
- **Date:** 2026-11-17
- **Follows:** [0299](0299-scripting-plan.md) (the scripting plan), [0011](0011-lua-scripting.md), [0119](0119-roadmap-after-the-uvs.md)

## Problem

Phase 14's first milestone: run a LustScript (`.lsc`) or Lua (`.lua`) file safely inside the app, let it read the scene, and show what it prints and its mistakes in plain words. Decision 0299 settled the main shape (the Lust compiler held as a library, Luau through `mlua`, a copy of the scene on another thread, the sandbox and limits). Building it needed a few smaller choices the plan doesn't cover.

## Options

1. **Time limits:** Solis sets Luau's interrupt directly with `unsafe` code, for game code's speed; or mlua's own safe `set_interrupt`. Scripts don't run every frame, and the app forbids `unsafe` without a decision ([0005](0005-safety-lints.md)).
2. **What running a file does,** before commands exist (milestone 4): only the code at the top level; or that, then each command the file makes (a LustScript `component` with `run()`, Lua's `command(…)` with a `run` function), once with its settings as they start.
3. **How the calls know the scripting folder's name:** written in the crate; or handed down by the app from `branding.rs` (`SCRIPT_NAME`).
4. **Shapes handed to scripts:** plain tables a script can change; or read-only values, one per shape, so `==` works and a script can't point one at another shape by changing its id.

## Decision

- The **`script` crate** (layer 2) holds the pinned Lust compiler (`v2.0.0-alpha.73`, `default-features = false`, from its Git repository, which `deny.toml` allows as the one Git source) and `mlua` 0.12 with Luau (its source held at 0.740 in `Cargo.lock`, as Solis does). Copied in and adapted from Solis (the project lead's own MIT code, Solis decision 0007): the sandbox and prelude (`luau.rs`), checks taking turns (`check_turns.rs`), plain-words Lua errors (`lua_problem`) and values read one at a time (`native_values.rs`).
- **The sandbox:** a fresh Luau per run; no `io`, `os`, `debug`, `require`, `loadstring`, `getfenv` or `setfenv`; text always loaded as text; the libraries, the app's global and its calls read-only; `table.move` capped and `collectgarbage` only counts. **mlua's safe interrupt** checks the run's time (10 seconds) and Stop, and once either has stopped the script every check stops it again, so `pcall` in a loop can't hold it off. 256 MB of memory. Printed output cut to 1,000 lines and 4 KB a line; points and polygons read 100,000 at a time.
- **Checking:** a LustScript file is checked with the description files by `lust::to_lua_with`, a Lua file by Luau's own reading, on a thread of their own, one at a time; a check past 30 seconds is given up on and none starts until it ends; a crash in the compiler is a mistake, never a crash of the app. Headers are read as text without running anything.
- **The description table** (`script::describe`) lists every call once; the `aventail/*.lsc` description files, Lua's `aventail.` names and the manual's [Script reference](../manual/script-reference.md) are made from it, and a test in the `app` crate fails when the page is out of date. The folder's name comes from `branding::SCRIPT_NAME`, handed to the interface in `AppInfo::script_name` (choice 3, second way).
- **Version 1's reading calls** (`Scene`, `Shape`, `Selection`, `Project`) and `Script`'s (`api`, `say`, `warn`, `progress`). Shapes are read-only values, one per shape (choice 4, second way). States cross as their option's name (`"Polygons"`), as Lust hands them to engines.
- **Running a file** runs its top-level code, then each command it makes once with its starting settings (choice 2, second way): in name order for LustScript, in the order made for Lua.
- **In the app:** a **Scripts** menu (between Window and Help) with **Run a script file…**, **Stop the script** and **Scripts panel**; a **Scripts** panel beside Coordinates holding only the output, shown when a script runs; the status bar shows the running script, its progress when it reports one, and **Stop**; **Esc stops a running script**, only while one runs and not while typing in a text box. Scripts change nothing in this milestone.

## Consequences

- No `unsafe` code is added. The interrupt costs more per call than Solis's raw one; fine for scripts that run once, to be measured again if scripts ever run every frame.
- A single library call (a huge pattern search, say) finishes before Stop is seen; the app never waits for it, since it runs on its own thread.
- Esc's other jobs wait while a script runs. The time and memory limits are fixed until Settings → Scripts arrives with the script manager (milestone 4).
- Messages from the app's own Lua prelude (a call misspelt, `command` given the wrong things) are in English; every message from Rust goes through the language layer.
- `RENAMING.md` lists the scripting folder and Lua global; renaming the app after scripts are shared must keep the old name working.
