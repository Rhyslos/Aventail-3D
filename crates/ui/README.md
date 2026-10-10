# ui

**Layer 3 · Front end**

The interface: the dockable layout, menus, palettes, Attribute and Object Managers, the UV Editor panel, and the theme.

## Purpose

Draws everything around and on top of the viewports with egui and egui_dock: menu bar, command palette, mode palette, the Attribute Manager (top right) and Object Manager (below it), the Coordinates panel, status bar and the calm dark theme. Hands viewport input to `tools` and carries out the commands they return. See DESIGN.md §5.

## Must never

- Change the scene except by executing commands
- Hold modelling logic: that belongs in `tools`, `mesh` or `scene`
- Use proprietary icons or artwork copied from other software

## Depends on

`tools` (the camera; `scene` and `config` planned). Outside libraries: `glam` (maths), `egui` and `egui_dock` (interface and docking), `serde` (saving the layout).

## Status

The window's layout is in place:

- **`Workbench`**: draws the whole window each frame and returns actions (`Quit`) for the app. Takes the app's name in `AppInfo`, so this crate never names the app.
- **Panels** (`Panel`): Viewport, Attributes, Objects, Coordinates, docked with egui_dock. Not closeable, not floating. The standard layout (`layout::default_layout`) puts Attributes top right, Objects below, Coordinates under the viewport; a damaged saved layout is replaced with a status-bar note.
- **Menu bar** in the design's order (File, Edit, Create, Modes, Select, Tools, Mesh, UV, View, Window, Help), with the layout tabs (Standard; Modelling and UV Edit greyed out) on the right. Working: File → Quit (Ctrl+Q), Modes, View → Reset view, Fine grid lines and Reset panel layout, Window → Accent colour, Help → About. Everything else is listed greyed out with the phase that brings it.
- **Panel menu lines** (`panels::menu_line`): Attributes (Mode, Edit) and Objects (Edit, View, Object), entries greyed out until Phase 2.
- **Palettes** (`palette`): the command palette (top, all commands greyed out for now) and the mode palette (left; Model, Object, Points, Edges, Polygons and UV can be picked, and the status bar says which is active).
- **Theme** (`theme`): the darker studio look (decision 0014), applied at startup by the app: colours, the viewport backdrop, the Inter font (Regular and SemiBold), text sizes, spacing, frames and the docked tabs' style. **`Accent`**: red (standard), blue, marine green, plum, olive or petrol blue, each with fill, light and soft shades; `set_accent` switches it app-wide and `accent` reads it back from egui's memory. Tests check every text colour is readable (WCAG 4.5:1), every accent's marks are visible, and neighbouring areas stay gently close.
- **Layout** (`layout`): besides the standard arrangement, `hide_viewport_tab` hides the viewport's tab each frame (its menu line replaces it) unless another panel shares its group.
- **Icons** (`icons`): 27 original icons drawn with egui shapes on a 24 × 24 grid.
- **Viewport** (`viewport`): its own menu line (`viewport::chrome`: View, Cameras, Display, Options, Panel, and buttons to drag for pan, zoom and orbit), the image from a `ViewportRenderer` sized in physical pixels, overlays (view and camera names, axis marker, grid spacing) and the navigation controls picker (`viewport::picker`) in the bottom-right corner. Without a GPU (`NoRenderer`), it shows the backdrop and says why. Turns drags and wheel turns into `tools::Navigator` moves using the chosen `NavigationSettings` (the drag kind is fixed when the drag starts) and sets the mouse cursor while navigating. `ViewOptions` (fine grid lines, navigation settings) is saved by the app between sessions.
- **Placeholders**: the other panels say what will appear in them and when.

Tests run the real interface without a window (egui_kittest): every tab and menu is present and in order, Attributes sit above Objects, the viewport has its menu line instead of a tab, Ctrl+Q and File → Quit quit, modes can be picked, unbuilt commands cannot be clicked, real drags and wheel turns navigate in each scheme, the header buttons navigate when dragged, the picker lists and switches schemes, the accent can be changed and is restored, layouts are reset and broken ones explained, and the About box opens.
