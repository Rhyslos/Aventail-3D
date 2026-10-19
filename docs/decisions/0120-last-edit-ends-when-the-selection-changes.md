# 0120. The Last edit ends when the selection changes

- **Status:** Accepted
- **Date:** 2026-10-20
- **Follows:**
  - [0038](0038-extrude-and-inset.md) (the Last edit);
  - [0048](0048-ctrl-wheel-changes-a-tools-count.md) (Ctrl + wheel);
  - [0092](0092-the-edit-list.md) (opening an edit from the edit list).

## Problem

The project lead found extrudes "a one-time use". The problem:

1. After an extrude, the Attributes panel shows it under **Last edit**, and its settings change it.
2. He then selected other polygons and changed the settings, meaning to set up his next extrude.
3. The panel was still showing the last extrude, so the change went to it and reshaped what he'd already made.

**Ctrl + wheel** did the same. The Last edit only moved on when a new edit was made, so there was no way to set up the next one without changing the one before.

## Options

1. **The Last edit ends when something else is selected (chosen).** Then the panel shows the tool's own settings for its next use, and Ctrl + wheel changes those. This is how other 3D programs behave: Blender's "adjust last operation" panel goes away once you do something else. A small line keeps the earlier edit at hand: **Last edit: …** with a **Change it** button, which opens it the way the edit list does.
2. Show both the Last edit and the next use's settings at once. Twice the rows, and it's easy to change the wrong one.
3. A pin to choose which one the panel changes. One more thing to understand, for something most people expect to just work.

## Decision

- The Attributes panel records, for each object, the edit it would show as the Last edit and the document's element revision then (`AttributesPanel::track_last_edit`, called every frame).
- The Last edit is "live" only while neither has changed (`last_edit_is_live`). The panel's Last edit section and `Workbench::wheel_count` both follow this.
- A tool that moves the selection on as part of its own edit keeps the edit live (`keep_last_edit_live`). Extruding edges does this when it selects the new outer edges.
- When the Last edit isn't live, the panel shows the tool's settings, and `left_edit_line` shows the earlier edit with **Change it** (`CHANGE_LAST_EDIT_LABEL`), which opens it as an opened edit.

## Consequences

- No new keys. Selecting anything (a click, a box, select all, grow, undoing a selection) ends the Last edit.
- Selecting another object and coming back keeps the Last edit live, as long as no points, edges or polygons were selected in between.
