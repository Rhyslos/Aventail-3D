# 0299. The scripting plan (Phase 14)

- **Status:** Proposed — chosen by Claude while the project lead was away; to review
- **Date:** 2026-11-17
- **Follows:** [0011](0011-lua-scripting.md) (Lua scripting, through Luau and mlua), [0119](0119-roadmap-after-the-uvs.md) (LustScript the default, plain Lua too)

## Problem

The project lead asked on 2026-11-17 for Phase 14's milestones, scripting, to be added, but couldn't answer questions that day. Decision 0011 chose Luau through `mlua`; decision 0119 then made **LustScript**, the project lead's own language, the default, with plain Lua too, and asked for 0011's design to be revisited for it when the phase is planned. LustScript's licence had to be permissive for the app to use it.

Studying Lust (alpha 73) answered the open questions: it is **MIT**; `.lsc` files (LustScript) are turned into plain Lua 5.1; and its compiler is a Rust library (`lust`, without its default features) that checks a program against **description files** with `native` methods and gives the Lua to run. Solis, the project lead's game engine, already holds it this way, running the Lua in Luau through `mlua` ([Solis decisions 0016 and 0028](https://github.com/Rhyslos/solis/blob/main/docs/decisions/0028-lust-runs-as-luau-while-editing.md)).

The phase needed: how scripts run and stay safe, what they can do, the shape of their calls, where they appear in the app, how they're kept and shared, how old scripts keep working, the controls, and the milestones.

## Options

The full list of questions, each with its alternatives and the cost of changing it later, is in [docs/plans/phase-14.md](../plans/phase-14.md#choices-to-review). The main forks:

1. **How LustScript runs.**
   - **(a) Compiler inside the app, Lua run in Luau** (as Solis): nothing to install, Luau's sandbox, checks in a fraction of a second.
   - **(b) LustScript built to Rust:** every user would need Rust and its build tools, and each change takes seconds or minutes.
   - **(c) Plain Lua first, LustScript later:** not needed, since Lust can be held today.
2. **Where a script runs.**
   - **(a) On another thread, on a copy of the scene,** landing as one undo step: the app never freezes.
   - **(b) On the interface thread in slices:** simpler; long scripts stutter the app.
   - **(c) Alongside the user's own edits, merged after:** clashes are hard to resolve reliably.
3. **How much scripts are trusted.**
   - **(a) Nothing outside the project unless approved,** with time and memory limits, and project scripts off until trusted.
   - **(b) Full trust,** as Blender, Maya and 3ds Max give their scripts. Unsafe for scripts shared between strangers.
4. **The shape of the calls.**
   - **(a) Solis's game API guidelines** (methods on things, a few named objects, fixed choices as states), from one table that makes the LustScript files, the Lua names and the manual's reference.
   - **(b) Calls shaped like Blender's Python operators.**

## Decision

Proposed, all to be reviewed by the project lead (plan, "Choices to review"):

- **LustScript (`.lsc`) is the default, plain Lua works in `lua { }` blocks and as whole `.lua` files.** The app holds the Lust compiler (pinned to a release tag, from Lust's repository, allowed in `deny.toml`), and runs the Lua in **Luau through `mlua`**, as Solis does. Time limits use mlua's safe interrupt, so the app stays free of `unsafe` code ([0005](0005-safety-lints.md)).
- **A new `script` crate** (layer 2) holds the compiler, the sandbox, the description table and the running. Scripts change the scene only through `scene::Command`s and recorded operations ([0006](0006-base-plus-operation-stack.md)).
- **A run works on a copy of the scene on another thread** and lands as **one undo step**; the project's scene waits while it runs; Stop works at any time.
- **Safety:** no files, network or programs by default; three file permissions approved once per script; no network in Phase 14; 10 seconds and 256 MB a run (Settings → Scripts); scripts saved in projects don't run until trusted on this computer.
- **Scripts describe themselves in header comments** (name, about, API version, what they ask for, a count setting), read without running. **A command is a Lust `component`** whose fields are its settings, shown in Attributes, with a `run()` method; in Lua, `command(name, settings)`.
- **The calls follow Solis's guidelines,** from one table, under a folder named after the app (`aventail/`, Lua's `aventail.`), as Solis uses `solis/`. **One API version number**, with old calls kept working a version longer.
- **In the app:** a **Scripts** menu, command search, the command palette and keys of your own for script commands; a **Scripts** panel (editor, output, console line) beside Coordinates; your library for every project, or kept with a project (file format 112); single files to share; event scripts for project opened, saved and export starting.
- **Controls:** F11 for the Scripts panel, Ctrl + Enter to run, Esc to stop a running script, Enter / Up / Down in the console line, Ctrl + wheel for a script command's count ([0048](0048-ctrl-wheel-changes-a-tools-count.md)), no keys for script commands as standard, Ctrl + S still saves the project.
- **Six milestones:** the script runner; changing the scene; the Scripts panel (editor and console); scripts as commands; keeping, sharing and permissions; event scripts, examples and the gate.

## Consequences

- Nothing is built until the project lead has reviewed the choices; each answer that differs updates the plan, this record and DESIGN.md §4.7 before milestone 1.
- Decision 0011's Luau and `mlua` stand; its "Lua" language becomes LustScript first, with Lua beside it.
- The app depends on an alpha language: Lust is pinned to a tag, and each move to a newer one is its own pull request that follows Lust's ENGINES.md. Plain `.lua` scripts don't go through the compiler, so they keep working whatever happens to Lust.
- `deny.toml` gains Lust's repository as the one allowed Git source, and the build needs a C++ compiler for Luau.
- The scripting folder's name (`aventail/`) is a new place the app's name appears: milestone 1 adds it to `RENAMING.md`, and a rename after scripts are shared must keep the old name working.
- Solis's sandbox, checking in turns and plain Lua errors may be copied in and adapted (the project lead's own MIT code).
- The project file format moves to 112 in milestone 5.
