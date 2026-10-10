# 0018. The scene document: an object tree changed by undoable commands

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

The first Phase 2 milestone, the Object Manager, is the first time the user changes the scene. Everything later (primitives, the operation stack, saving, modelling tools) builds on how the scene is held and changed. Deleting an object with no way back would also be the first thing a user can lose, so undo is needed from the start.

## Options

1. **Change the scene directly from the interface**, adding undo later. Quick now, but every change would have to be rewritten as a command later, and nothing is safe until then.
2. **Copy the whole scene for every undo step.** Simple and always right, but memory grows with scene size times steps, which DESIGN.md §4.3 rules out.
3. **Commands made of small reversible edits.** Every command (create, rename, move, delete, group…) is carried out as a list of tiny edits: insert an empty object at a place, remove an empty object, move an object from one place to another, rename, show or hide. Each edit knows its opposite, so undo applies the opposites in reverse order. This is the approach Halberd's document uses (inserted, removed and modified objects), extended to a tree.

## Decision

Option 3, in the `scene` crate:

- **`Scene`** is a tree of objects. Each object stores only its *local* transform; world positions are worked out from its parents (DESIGN.md §4.5). Order among brothers and sisters matters (it is the Object Manager's order).
- **Object ids** count up and are never reused, so undo and redo bring back the very same ids, and replaying the same commands gives the same ids (§4.2).
- **`Command`** is every change a user can make; **`Document::execute`** carries it out and records its edits as one undo step. A command that fails changes nothing; one that changes nothing records no step.
- **The interface never changes the scene itself**: the Object Manager, menus and keys hand commands to the workbench, which executes them.
- **New objects go at the top of the list**, as in the program the layout follows, so they are seen without scrolling.
- **Selection is not an undo step yet**; DESIGN.md §4.3 wants it undoable but coalesced, which comes with the history milestone.
- **History is unlimited for now.** Steps are small (a few edits each), and the memory budget of §4.3 is still its own Phase 2 milestone; it will cap this list rather than replace it.
- **Keys** follow Halberd for undo (`Ctrl + Z`), redo (`Ctrl + Y`, `Ctrl + Shift + Z`), delete (`Delete`, `Backspace`) and deselect (`Esc`), and add `Alt + G` (group) and `Shift + G` (ungroup), which are free in Halberd ([0016](0016-controls-follow-halberd.md)).

## Consequences

- Every later feature adds commands (and, for modelling, operations, [0006](0006-base-plus-operation-stack.md)) instead of changing data directly, and gets undo for free.
- Undo can't drift from the scene: an edit that no longer fits (which would be a bug) is refused, and the step is put back as it was rather than half undone.
- Saving (later in Phase 2) saves the tree and the next id, so ids stay unique across sessions.
- Until real primitives exist, the scene starts with one object, the test cube, which the renderer draws when it is in the scene and shown.
