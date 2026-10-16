# 0087. One look for every window, and the Export window rebuilt

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0075](0075-export-presets.md) (export presets), [0076](0076-engine-export-paths.md) (game project folders), [0009](0009-theme.md) and [0014](0014-studio-look-and-accent.md) (the theme)

## Problem

The project lead found the Export window hard to look at: a long column of radio buttons, headings and greyed-out settings, with the game project folder at the bottom and no sign of what would go out. Each window had grown its own layout too: Settings had a side list, the bake check a plain list, the questions their own buttons in their own order, and the menus were cramped, their keys pressed against the names.

He asked for one calm, unified look, approved a set of mockups (2026-10-15), and chose:

- **Made for** as one drop-down list, not big tiles: Godot, Unity, Unreal Engine and Substance Painter, each with its file type beside it, then **Custom settings** for any other program (his name for it).
- **One window with a short side list**, not folded boxes: General first, Game project last, and Materials, Mesh, Rig and Animations listed only once their features exist.
- A plain explanation of the game project export inside General, and a taller window than the first mockup.

## Options

- **Where the shared look lives:**
  - **a `dialog` module in `ui` with the parts every window uses** (chosen): one place to change spacing, colours and buttons, and every window follows;
  - styling each window by hand, which is how they drifted apart.
- **How a preset's settings show:**
  - **one locked line ("glTF · metres · Y up · triangles"), with Custom settings to change them** (chosen, as in the approved mockup): nothing greyed out to puzzle over, and the window never claims a preset it isn't following;
  - greyed-out controls, as before.
- **Custom settings starts from:**
  - **the preset that was chosen** (chosen): only what differs needs changing;
  - fixed standard values, which would undo the preset's choices.
- **Sections without settings yet (Materials, Mesh, Rig, Animations):**
  - **not listed until their features exist** (chosen, with a note at the foot of the list), so there is nothing empty to click through;
  - listed and greyed out.
- **Esc with a window open:**
  - **closes the window** (chosen; the footer's Cancel), and selecting nothing waits until no window is open;
  - selects nothing, as before, leaving the window open.

## Decision

1. **`ui::dialog`** holds the parts:
   - a window frame (`Dialog`) with a title bar (name, anything beside it, ×);
   - a body, or a side list of sections with a word on the right (on, off);
   - a footer with a quiet note or a button on the left and the buttons on the right, the main one filled in the accent colour.
   - Inside: section headings, help text, rows with the name in a fixed-width column, segmented choices for two to four options, drop-down lists for longer ones, chips, a locked value, a path box and number tiles.
   - Enter presses the main button and Esc cancels, unless a text box is being typed in. Questions (`.modal()`) dim the app until answered and have no ×.
2. **The Export window**: Made for in the title bar; General (a sentence on the preset, Objects, Includes with Mesh and Materials, the file settings, a **Before baking** button for Substance Painter, where every save exports to with a link to Game project, and **What goes out**: objects, triangles and materials); the Game project section (the folder, its engine, Export on every save, Choose folder…, Clear). Export… is greyed out when nothing would go out.
3. **`fileio::export::summary`** counts what an export would hold without making the file; the window keeps the count until the scene, the selection or Objects changes.
4. **Settings, Check for baking, About and the questions** (unsaved changes, dissolving across a corner, recovering backup copies) use the same parts. Settings' units are segmented choices. The Keys window keeps its own frame: it is resizable and draws a keyboard that scales with it.
5. **Menus**: every menu bar's menus get roomier rows (`dialog::menu_style`) and a minimum width (`dialog::menu`), so keys line up on the right.

## Consequences

- New windows are built from `dialog`, and look like the rest without extra work.
- When Materials, Mesh, Rig and Animations gain export settings, each becomes a section in the side list with its state on the right; Includes gains Skeleton and Animations then.
- The Keys window and the viewport's navigation picker are the two places still drawn their own way.
- Tests that click settings further down a window scroll to them first, as a person would.
