# 0302. Scripts change the scene (Phase 14, milestone 2)

- **Status:** Accepted, built while the project lead was away; its further choices are listed for review in [docs/plans/phase-14.md](../plans/phase-14.md#choices-to-review) (23 to 29), with the plan's own
- **Date:** 2026-11-17
- **Follows:** [0299](0299-scripting-plan.md) (the scripting plan, choice 4), [0301](0301-script-runner.md) (the script runner), [0006](0006-base-plus-operation-stack.md) (changes only through commands and operations)

## Problem

Milestone 2 of scripting: let scripts create, build, move, rename, hide, delete, select, bevel, extrude, inset, mark hard edges and seams, unwrap, give materials and make shapes editable, each through the same commands and operations as the app's tools, so every change can be undone and shows in Edit layers. A whole run must land as one undo step, nothing may land from a run that fails or is stopped, and while a script runs on a copy of its project the real project must not change under it (plan, choice 4), while other tabs stay free.

## Options

1. **How a run's changes reach the document:** (a) the script's copy of the scene replaces the real one at the end (no record of what changed, so no undo); (b) each change is carried out on the copy as the script makes it and kept in a list, and the same list is carried out again on the real document at the end, as one step: commands are deterministic (DESIGN.md §4.2), so the two end the same.
2. **One step from many changes:** run each through `execute_merging` with one key (the history folds transform and kind changes into earlier ones, and measures the step again on every change: slow for thousands); or a new `Document::execute_batch` that carries out commands and selection changes one after another and records them once.
3. **Keeping the real project still while a script runs:** (a) disable the interface for that project (many places change the document; one missed would break the landing); (b) **hold** the document itself: while held, every change to it (commands, undo, redo, selecting) is turned away with a reason, in one place.
4. **A change the scene refuses, caught by the script with `pcall`:** carry on and land the rest (the copy has used up an object id the real document hasn't, so later changes could land on the wrong object); or spoil the run, so nothing lands.
5. **A run in a tab not in use:** wait until the tab is used again; or keep taking in what it says from every tab each frame, so it lands when it ends.

## Decision

- **Changes as a list, landed again** (1b). The `script` crate turns the run's `Snapshot` into a `scene::Document` (keeping no undo history of its own); every changing call makes a `scene::Change` (a `Command`, often `Command::Record` with an `Operation` exactly as the tools build it, with its anchors and stretch; or a change of selected objects or elements), carries it out on the copy at once, so the script reads its own results, and keeps it. A run that ends well sends `Event::Land` with the changes and the copy's final scene before it ends.
- **`Document::execute_batch(changes, label)`** (2, second way) in the `scene` crate: commands and selection changes one after another as one undo step called `label`; if one is refused, everything before it is taken back (scene, selection, selected elements) and `Refused { at, why }` says which. The landing of 10,000 small changes takes milliseconds.
- **The document is held** (3b): `Document::hold`, `release` and `is_held`. While held, `execute` and its kin give `SceneError::Held` ("a script is running on this project: wait for it to finish, or stop it with Esc"), and undo, redo, selecting and `abandon` do nothing. The interface holds the project when a script starts and lets go when it ends; Ctrl + Z while held says why it can't. The landing checks the document is still held (the same project), lands the changes as one step named after the script, and checks the result is the copy's scene, noting a bug in the output if it ever isn't.
- **A refusal spoils the run** (4, second way): the script stops with the scene's own reason ("shape:setName(name) couldn't be done: a name can't be empty."); if it catches the mistake, every later change is refused and nothing lands.
- **Every tab's scripts are taken in each frame** (5, second way), so a run lands when it ends in whichever tab it was started; a tab with a running script doesn't go to sleep; closing a tab stops its script.
- **Version 1's changing calls:** `Scene:add(Primitive kind)`, `Scene:build(name, points, polygons)`, `Scene:group(shapes)`; `shape:setName`, `setPosition`, `setRotation` (degrees), `setScale`, `show`, `hide`, `delete`, `bevel(edges, width, segments)`, `extrude(polygons, distance)`, `inset(polygons, amount)`, `markHard(edges)`, `markSeams(edges)`, `unwrap()`, `setMaterial(polygons, name)`, `makeEditable()`; `Selection:set(shapes)`, `setPoints`, `setEdges`, `setPolygons` (one shape's, the other kinds kept) and `clear()`. A `Primitive` state lists the ready-made shapes. Elements a shape hasn't got are refused with their ids; empty lists, and edges already hard or already seams, change nothing.
- **Limits:** 100,000 changes a run; a build of at most 100,000 points and 100,000 polygons with 1,000,000 corners in all; 5,000,000 points and polygons built in a run (kept in the app's memory, which the script's own limit doesn't count); a list given to one call at most 100,000 items. The scene's own limit on how far an edit may grow a shape (30 million polygons) stands.

## Consequences

- Nothing a script does is hidden: each edit is the tool's own, in Edit layers, and the whole run is one undo step.
- While a script runs, its project turns changes away rather than queueing them: the person waits, or presses Esc. Changes are never lost silently: each one refused says so.
- A long edit inside one call (a bevel on a heavy shape) finishes before Stop is seen, as with any single call; the app never waits for it.
- Landing carries out the commands again on the interface thread: as quick as the same edits made by hand (the edits' shapes are worked out later, as for tools).
- `Scene:add` is called that rather than the plan's `create`, which is one of Lust's own words; `Scene:build` takes `Polygon`s, since Lust's native methods can't take lists of lists (choices 23 and 25).
- File format unchanged (111).
