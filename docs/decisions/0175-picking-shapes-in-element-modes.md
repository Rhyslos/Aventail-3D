# 0175. Picking a shape in the Points, Edges and Polygons modes

- **Status:** Accepted (the project lead's choice)
- **Date:** 2026-10-28

## Problem

In the Points, Edges and Polygons modes, a click picks the selected shapes' points, edges or polygons. With nothing selected there was nothing to pick, and a click on a shape did nothing: the project lead had to select it in the Objects panel ("I have to click on a list. I dislike this").

## Options

1. **A click on a shape selects it, and the mode stays** (recommended): with nothing selected, the click picks the shape as the Model mode would, and the next clicks pick its parts.
2. **Leave the mode when nothing is selected**: back to the Model mode, where a click selects a shape.

The project lead asked which one Cinema 4D uses, so the other could be a setting. As far as could be found, Cinema 4D does neither (it stays in the mode, and a shape is picked in the Object Manager or after going back to the Model mode); this couldn't be confirmed from Maxon's documentation.

## Decision

Option 1 as standard, the project lead's choice; option 2 as **Settings → Viewport → Selecting → Back to the Model mode when nothing is selected** (`ViewOptions::leave_element_modes_when_empty`, off as standard).

- With nothing selected in an element mode, a click picks the object under the mouse exactly as in the Model mode (`Selector::object_at`: a cage's line first, a cutter only where nothing else is, a curve's line), with the same keys to add or take away. With a shape selected, clicks pick its parts as before.
- The setting reacts only to the selection *becoming* empty in an element mode (`Workbench::leave_empty_element_mode`), so pressing 1, 2 or 3 with nothing selected still enters the mode; a modelling tool in use is put away, and the status bar says so.

## Consequences

- No trip to the Objects panel to start editing a shape.
- A click on another shape while one is selected still picks the selected shape's parts only, as before.
