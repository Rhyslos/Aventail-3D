# 0309. Scripts that run by themselves, example scripts, and the end of Phase 14 (milestone 6)

- **Status:** Accepted, built while the project lead was away; its further choices are listed for review in [docs/plans/phase-14.md](../plans/phase-14.md#choices-to-review) (58 to 68), with the plan's own
- **Date:** 2026-11-18
- **Follows:** [0299](0299-scripting-plan.md) (the scripting plan, choice 13), [0301](0301-script-runner.md), [0302](0302-scripts-change-the-scene.md), [0303](0303-scripts-panel.md), [0305](0305-scripts-as-commands.md), [0307](0307-keeping-sharing-and-permissions.md); [0033](0033-heavy-mesh-safeguard.md) (the heavy-mesh check at a phase's end), [0076](0076-engine-export-paths.md) (exporting into the game project on every save)

## Problem

The last scripting milestone. Scripts must be able to run by themselves at three moments (a project opened, a project saved, an export starting), only when each script is switched on for it, for at most 2 seconds, with the status bar saying when one ran. A script at an export starting may warn or stop the export with a reason, but not change the scene. A project's scripts must still wait for trust, and a slow or failing script must never stop a save. The plan also asks for example scripts explained in the manual, the heavy-mesh check with scripts, and the phase gate.

## Options

1. **How a script says it reacts to a moment:** (a) a header line (`-- runs when: saved`) running the whole file; (b) **Lust's own events**: the app declares the moments as events, a component reacts with `on ProjectSaved() { … }`, and Lua, as Solis's Lua does, with a function named `on_` and the moment's name.
2. **How saving and exporting wait:** (a) run the scripts on the interface thread (the app would freeze up to 2 s a script); (b) **hold the save or export back** while the scripts run on their own threads, one after another, and hand it to the app once they've ended.
3. **What an export-starting script gets:** (a) nothing but the moment (it reads the scene); (b) **calls for what goes in** (`Export:shapes()`, `Export:format()`) and **`Export:cancel(reason)`**.
4. **What happens to a script that still asks to reach files:** (a) the access window opens while the save waits; (b) **it's left out, with a note**, until allowed in the script manager.

## Decision

- **Lust events** (1b; choice 58): `aventail/moments.lsc` declares `event ProjectOpened();`, `event ProjectSaved();` and `event ExportStarting();` (from `script::describe::HAPPENINGS`), fetched like the calls. A component's `on` reacts; in Lua a command's `on_ProjectOpened` (and so on). `script::commands_of` reads the moments a script reacts to (`Described::moments`); a component or Lua command that only reacts isn't a command. `Run::start_moment` runs the script's start, then each component reacting (in name order; Lua's in the order made) with its settings as last used (choice 60), at most `script::MOMENT_LIMIT` (2 s).
- **Held back** (2b): `Workbench::hold_for_moments` takes the frame's saves and exports whose moments have scripts switched on and queues them as jobs (`script_moments::Job`); `poll_moments` runs the scripts one after another and hands the save or export to the app (`released_actions`, from a tab not in use too) when they've ended. Opening a project queues its moment after it opens. A run lands as usual: one undo step named after the script. A project replaced while something waits forgets it. When (choice 59): opened, just after; saved, just before writing (what the script changes is in the file); export, before anything is written or asked.
- **Every export** (choice 61): the Export window's, a kit export, Export to game project, and the export into the game project after every save; a script stopping that last one lets the save go on without it.
- **The `Export` calls** (3b; choice 62): `Export:shapes()` (every shown object going in, in the Objects panel's order), `Export:format()` (a new `Format` state: `Glb`, `Obj`, `Stl`) and `Export:cancel(reason)` (the first reason counts, whatever the script does after; files it wrote aren't written). They work only while an export starts; a change to the scene then stops the script and keeps nothing.
- **Left out with a note** (4b; choice 63). A failing, stopped (Esc) or slow script is said in the output and the status bar, and the save or export goes on; the Scripts panel isn't opened for these runs (choice 64). The status bar says which ran: "Saved crate.avtl. Ran by itself as the project was saved: Tidy names."
- **Switched on in the script manager** (choice 65): under each script that reacts to a moment, "Runs by itself when: □ a project opens □ a project is saved □ an export starts" (only those it reacts to, and any still on), off as standard, kept in the settings by script (`ScriptSettings::moments`; yours by file name, following renames; a project's by fingerprint, shown once its scripts are on). A project's scripts take part only once turned on, and **Turn them off** (choice 66) turns them off again.
- **`Mode.Sculpt`** (choice 67): scripts now see Sculpt mode as its own option.
- **Example scripts** (choice 68): six LustScript files in `crates/script/src/examples/` (Rename by pattern, Gear, Check before export, Export each shape, Select polygons facing up, Tidy names), `script::examples`; **Scripts → Example scripts** copies one into your library (numbered beside one of yours) and opens it in the editor. Each is explained in the manual's Scripting page, and a test runs each and checks it does what the manual says.
- **The heavy-mesh check** gains `heavy_script_check`: a script reading every polygon of the heavy shape 100,000 at a time, and one changing it (renaming, extruding a polygon, a material on 100,000 polygons), landing as one undo step.

## Consequences

- Saving and exporting can wait up to 2 seconds a script switched on; nothing waits when none is.
- Scripts written with `check` over `Mode` need a `when Sculpt`; none is shared yet.
- No new keys, no new file format: switches live in the settings file beside approvals and trust.
- Phase 14's milestones are all built; its gate waits for the project lead.
