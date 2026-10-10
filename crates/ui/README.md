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

None yet (`scene`, `tools`, `config` planned). Outside libraries: `egui` and `egui_dock` (interface and docking), `serde` (saving the layout).

## Status

The window's layout is in place:

- **`Workbench`**: draws the whole window each frame and returns actions (`Quit`) for the app. Takes the app's name in `AppInfo`, so this crate never names the app.
- **Panels** (`Panel`): Viewport, Attributes, Objects, Coordinates, docked with egui_dock. Not closeable, not floating. The standard layout (`layout::default_layout`) puts Attributes top right, Objects below, Coordinates under the viewport; a damaged saved layout is replaced with a status-bar note.
- **Menu bar** in the design's order (File, Edit, Create, Modes, Select, Tools, Mesh, UV, View, Window, Help). Working: File → Quit (Ctrl+Q), Modes, View → Reset panel layout, Help → About. Everything else is listed greyed out with the phase that brings it.
- **Palettes** (`palette`): the command palette (top, all commands greyed out for now) and the mode palette (left; Model, Object, Points, Edges, Polygons and UV can be picked, and the status bar says which is active).
- **Theme** (`theme`): the calm dark look from the mockups, applied once at startup by the app: colours, the Inter font (Regular and SemiBold), text sizes, spacing, the bars' and palettes' frames and the docked tabs' style. Tests check every text colour is readable (WCAG 4.5:1) and neighbouring areas stay gently close.
- **Icons** (`icons`): 22 original icons drawn with egui shapes on a 24 × 24 grid.
- **Placeholders**: the viewport shows its calm grey gradient; the other panels say what will appear in them and when.

Tests run the real interface without a window (egui_kittest): every tab and menu is present and in order, Attributes sit above Objects, Ctrl+Q and File → Quit quit, modes can be picked from the palette and the Modes menu, unbuilt commands cannot be clicked, View → Reset panel layout works, a broken saved layout is replaced and explained, and the About box opens.
