# 0022. Undo history bounded by a memory budget

- **Status:** Accepted
- **Date:** 2026-10-11

## Problem

Until now the undo history kept every step for as long as the app was open. That is fine for renames and settings, but once steps hold whole meshes (Make editable, and from Phase 3 every modelling edit on large meshes) a long session could use up the computer's memory. DESIGN.md §4.3 and wish W3 ask for **no fixed step limit**: the history should be bounded by memory instead.

## Options

1. **A step limit** (say 100 steps). Simple, and common in other programs, but it's the wrong measure: 100 renames take almost nothing, while 100 steps holding large meshes can take gigabytes. The project lead specifically asked not to have one.
2. **A memory budget.** Each step's size is estimated when it's recorded, a running total is kept, and the oldest steps are forgotten once the total goes over the budget.
3. **A memory budget that spills old steps to disk** instead of forgetting them. Nothing is ever lost, but it needs a file format for steps, which arrives with saving projects. It can be added on top of option 2 later.

## Decision

Option 2, with option 3 left for later.

- **Measuring.** Each step's size is estimated as it's kept (`scene::memory`): names, settings, layers and operations, and meshes counted in full (`Mesh::memory_size`). A mesh shared between the scene and the history is counted every time a step holds it, so the estimate errs on the side of using less memory than the budget.
- **What gets forgotten.** When the total goes over the budget, the oldest undo steps go first. If lowering the budget still leaves it over, the redo steps furthest away go next. **The newest step is always kept**, however large, so the last change can always be undone.
- **What counts.** Undo and redo steps both count, since both hold memory. A drag merged into one step is measured again as it grows. A step that only changes settings stays small however long the drag.
- **The standard budget is 2 GB.** That is enough for a long session on a computer with 8 GB or more. *Edit → Undo memory* offers 256 MB, 512 MB, 1 GB, 2 GB, 4 GB, 8 GB or Unlimited. The menu shows the memory in use and the number of steps, and the choice is remembered between sessions.
- **Telling the user.** The first time steps start to be forgotten, the status bar says so once and points to the setting.

## Consequences

- Long sessions with heavy meshes can't run the computer out of memory through undo alone.
- Sizes are estimates. They track real use closely enough for a budget, but they aren't exact, so the menu says "In use", not an exact count.
- Selection is still not an undo step. DESIGN.md §4.3 plans to make it one (coalesced), and that will be decided with the selection tools in Phase 3.
- Keeping undo history across restarts (in the project file) and spilling to disk are left for after saving exists.
