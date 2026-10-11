# 0023. One Settings window for app-wide preferences

- **Status:** Accepted
- **Date:** 2026-10-11

## Problem

App-wide preferences had ended up in whichever menu was nearest when each was built: the accent colour under Window, undo memory under Edit, navigation in the viewport's corner, units in the Attributes panel. The project lead found this "semi-random" and asked for one place to see them.

## Options

1. **Leave them in the menus,** each in its most fitting menu. Nothing to build, but there's no single place to look, and the menus fill up with things rarely changed.
2. **A Settings window** with sections, opened from the menu bar and a key. This is what most programs do (often called Preferences).
3. **A Settings panel** docked like the other panels. It would always take up room, for things changed rarely.

## Decision

Option 2: a **Settings** window, opened with **Edit → Settings…** and **Ctrl + ,**, the place and key most programs use. It has three sections:
- **Interface:** accent colour, lengths in metres or centimetres.
- **Viewport:** navigation controls and their switches, grid lines.
- **Undo:** undo memory, with what's in use now.

Changes apply at once. **Restore standard** resets them all; the panel layout keeps its own reset in the View menu.

Rarely changed preferences live **only** in Settings: the accent colour (taken out of the Window menu) and undo memory (taken out of the Edit menu). Switches that are flipped while working stay where they're used **and** show in Settings, changing the same value: the navigation button in the viewport's corner and the grid button. *(The m/cm switch beside the Shape title was removed on 2026-10-11 at the project lead's request: the Attributes panel's Units menu already does it, and the switch sat far from everything else.)*

## Consequences

- New app-wide preferences go into Settings, in the section they belong to (or a new one), not into a menu.
- A preference that also gets a quick switch elsewhere must change the same stored value, so the two never disagree.
- The window is not a dock panel, so it can't be lost in the layout. It is also not modal: the scene stays usable while it's open.
