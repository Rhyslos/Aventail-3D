# 0092. The edit list: open, switch, keep, freeze and delete any edit

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0044](0044-edits-that-hold.md) (edit ids, on/off and pinned switches), [0090](0090-stable-element-names.md) (stable names), [0091](0091-edit-layers.md) (edit layers); the 2026-10-13 log entry (the project lead's request)

## Problem

Only the last edit could be changed; every edit has carried an id and on/off and pinned switches since 0044, with no way to use them. The project lead asked (2026-10-13) for an edit list per object: click any edit to change it in the Attributes panel and see everything after it made again; switch edits off; pin (keep) edits to keep them editable and freeze the old unpinned ones to keep big models fast; and plain warnings only for what truly can't be found.

## Options

- **Where an edit opens:**
  - **in the Attributes panel, in the Last edit's place** (chosen), with the same setting rows, so there's one way to change an edit;
  - a window of its own, one more thing to place and close.
- **When it closes:** **with Close, or when a new edit is made** (chosen; then the new edit is the one you'd expect to see); keeping it open after a new edit would hide the new one's settings.
- **What Freeze bakes:**
  - **the base and every edit up to the chosen one, into a plain mesh base** (chosen): the bottom of the stack is the only part that can be baked without changing what the edits above it are made on. The shape doesn't change, the edits after it keep their ids and anchors and apply as before;
  - only the chosen edit, which can't be done: an edit is made on the shape below it.
- **Kept and switched-off edits in the way:**
  - **Freeze is refused, saying why** (chosen): it never bakes an edit you asked to keep, and won't guess whether a switched-off edit (or layer) should be thrown away or baked in;
  - stopping just before the first kept edit, which freezes less than asked without saying.
- **Warnings:** **only for what truly can't be found** (chosen, as asked): "parts missing" and "skipped" marks on the edit; an edit that found its points again by place is fine and shows nothing.
- **Finding by name first** (held back in 0090) **stays held back**: the edit list is now there to try it on real changes. It needs the project lead's look first, because it changes what existing projects make.
- **Keys:** the project lead was asleep; **no new keys**, the recommended choice ([0028](0028-ask-about-controls.md)).

## Decision

1. **Scene:**
   - `Command::DeleteEdit { id, edit }` removes one edit ("Delete extrude 1 polygon").
   - `Command::Freeze { id, edit }` bakes the base and the edits up to `edit` (`stack::evaluate_once` on that part of the stack) into `ObjectKind::Mesh`. It removes those edits and the layers below `edit`'s, which are now empty; `edit`'s own layer stays with the edits after it. It is one undo step ("Freeze 3 edits", counted by `frozen_count`).
   - `Stack::freeze_problem` says why not: a kept or switched-off edit, or a switched-off layer, among them.
   - Changing any edit's settings is the existing `Command::ReplaceOperation`, and on/off and keep are `SetEditEnabled` and `SetEditPinned`.
   - No file format change: a frozen base is a plain mesh, which files have always stored.
2. **The interface:**
   - **The Edit layers panel's edit rows** can be clicked. Each has:
     - an eye;
     - its name, with "(off)" when switched off;
     - marks on the right: **kept**, **parts missing**, **skipped**;
     - a right-click menu: Open in Attributes, Switch off, Keep editable, Freeze up to here, Delete edit.
   - **The Attributes panel** shows an opened edit (`attributes::OpenedEdit`) as **Edit: …** with:
     - where it is ("Edit 2 of 5 in Edits");
     - its problem, if any;
     - **On** and **Keep editable**;
     - its settings (the Last edit's rows, `edit_settings`);
     - **Freeze up to here** (greyed out with the reason) and **Delete edit**;
     - **Close**.

## Consequences

- Any edit can be changed, switched off or taken out. Edits after it are made again and find their elements by anchors and names. An edit that loses some of its parts says so on its row.
- Big models can be kept quick by freezing what's settled, without losing the edits still being worked on.
- Freezing a primitive's edits turns it into a plain mesh (its settings are baked), as Make editable does.
