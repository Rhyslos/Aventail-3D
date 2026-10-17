# 0094. An optional quad menu

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0037](0037-tool-menu-and-command-list.md) (the tool menu and the command list); DESIGN.md §5 (a Max-style quad menu as an optional sprinkle) and Phase 6

## Problem

3ds Max users right-click for a menu of four lists around the mouse (the quad menu). DESIGN plans one as an optional "sprinkle" in Phase 6. Here, the tool menu (E) already holds the tools at the mouse, and a right-click already has jobs: it cancels drags, and some navigation controls turn the view with a right drag.

## Options

- **On or off as standard:**
  - **off, switched on in Settings → Interface** (chosen): "optional", as planned, and a right-click keeps doing nothing for those who don't want it;
  - on, which would surprise users of the other navigation schemes.
- **What opens it:**
  - **a right-click (press and let go without dragging) in the viewport** (chosen; recommended, as 3ds Max, with no clash). A right drag still turns the view. A right-click that ends something never opens it: one that cancels a drag, one that comes while the app waits for a click on an object, or one while a window or the tool menu is open;
  - a key, which E already covers.
- **What's in it:**
  - **Show** (top left): Frame selected, Hide selected, Isolate, Show all;
  - **Modes** (top right): Model, Object, Points, Edges, Polygons;
  - **Edit** (bottom left): Undo, Redo, Add edit layer, and Snap to… / Align… in the Model and Object modes;
  - **Tools** (bottom right): the current mode's tools from the command list, as the tool menu has them.
  
  This is 3ds Max's layout with our own contents. Each entry shows its key, and an entry that can't be used now is greyed out with the reason.
- **Keys:** the project lead was asleep; the recommended choice (right-click, off as standard) was taken ([0028](0028-ask-about-controls.md)).

## Decision

1. `quad_menu::QuadMenu` draws the four lists around the click, as foreground areas, with a dot where it was opened. It returns the `QuadAction` picked. Esc, or a press outside the lists, closes it.
2. The viewport reports right-clicks (`ViewportPanel::take_right_click`) unless a gizmo or tool drag had them. The workbench opens the menu only when `ToolMenuSettings::quad_menu` is on and nothing was under way when the button went down (`note_right_press`, `busy_for_quad`).
3. `workbench::quad` fills the lists (`quadrants`) and carries out the pick (`quad_action`) through the same paths as the menus and keys.
4. The setting is saved with the tool menu's (`ToolMenuSettings`), off as standard. The Keys window lists the right-click.

## Consequences

- 3ds Max habits have somewhere to go, without changing anything for those who leave it off.
- New commands reach the Tools list through the command list.
