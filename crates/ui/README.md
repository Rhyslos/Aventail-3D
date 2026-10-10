# ui

**Layer 3 · Front end**

The interface: the C4D-style dockable layout, menus, palettes, Attribute and Object Managers, the UV Editor panel, and the theme.

## Purpose

Draws everything around and on top of the viewports with egui and egui_dock: menu bar, command palette, mode palette, the Attribute Manager (top right) and Object Manager (below it), the Coordinates panel, status bar and the calm dark theme. Hands viewport input to `tools` and carries out the commands they return. See DESIGN.md §5.

## Must never

- Change the scene except by executing commands
- Hold modelling logic: that belongs in `tools`, `mesh` or `scene`
- Use proprietary icons or artwork copied from other software

## Depends on

`scene`, `tools`, `config` (planned).

## Status

Empty. The window and panel layout arrive later in Phase 0.
