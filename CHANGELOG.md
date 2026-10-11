# Changelog

Every change a user would notice, newest first, in plain words.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- **Make editable** is now **Ctrl + E** (was C), so plain letters stay free for tools.
- **Quit has no key any more** (Ctrl + Q is gone), so a slip can't close the app: use File → Quit or the window's ×.
- The m/cm switch beside the Shape title is gone; the Attributes panel's Units menu and Settings do the same.
- Selected objects, the highlighted shape in the shape list, the chosen navigation scheme and other chosen items now use the same red as the active mode button.
- Unsaved changes are easier to see: an **Unsaved** badge next to the project's name on the right of the menu bar, and "● name (unsaved)" in the window's title instead of a small `*`.

### Fixed

- On Windows, pressing keys no longer plays the system error sound after the app starts.
- The accent colour moved from the Window menu, and undo memory from the Edit menu, into the new Settings window.
- Every new shape is 1 m across as standard (a 1 m cube, a 1 m square plane, cylinders and cones 1 m across and 1 m tall, and so on), instead of 2 m.
- The grid button on the left palette now steps through three levels: a line every metre (standard), every 50 cm (each square cut into four), and every 10 cm. The View and Display menus list all three as **Grid lines**. A finer grid chosen before is kept as the 10 cm level.
- Sizes in the Attributes and Coordinates panels are shown in metres as standard; a switch beside the Shape title (and the Attributes panel's Units menu) changes to centimetres. Remembered between sessions.
- Selected objects' names in the Objects panel are now in the accent colour instead of orange, matching their outline in the viewport.
- The small ring that marks the point you orbit around is only shown when orbiting around the point under the mouse; turning around the middle of the view no longer shows it.
- A new scene starts empty: the test cube is gone, now that real shapes can be created.
- Faces show from both sides (the underside of a plane is a little darker), and the grid no longer shows through shapes lying on the ground.
- A darker, calmer look, closer to the approved mockup: darker panels with plain text tabs, small menu lines inside the panels, and a viewport with a grey sky over a slightly darker ground, a horizon, and a grid that fades out around the point you're looking at (at least 18 m around it, reaching farther as you zoom out). A green line rises from the origin to show Z, and the test cube is a mid grey.
- The viewport has its own menu line (View, Cameras, Display, Options, Panel) in place of its tab, with buttons you can drag to pan, zoom and orbit, and labels showing the view, the camera, which way the axes point, and the grid spacing.
- The active mode is now marked by a filled red square (the new accent colour) instead of orange; the opening view is a little lower, so the horizon shows.
- Axis colours are now X red, Y blue, Z green (most programs use Y green and Z blue), in the viewport, the Coordinates panel and the palette's axis locks.
- The viewport looks softer: gentler light with a slight gradient across each side, a little shade where shapes meet the ground, a quieter grid and fainter edges.
- The Coordinates panel is now just tall enough for its rows, and every panel can be dragged much smaller than before.
- Palette icons follow the mockups: soft white, with the shapes you can create in blue. Commands that aren't built yet are only slightly dimmed.
- The Coordinates panel follows the mockup: coloured X, Y and Z letters, wide dark value fields, and Object (Rel) and Apply on the right.
- Z is up: X and Y lie on the ground and Z is height, as in 3ds Max and Blender. Shown in the manual; the viewport is built this way from Phase 1.
- Project files will be saved as `.avtl` (short for Aventail) instead of `.rhy`. Nothing has been saved yet, so no files are affected.

### Added

- A **Mesh** section in the Attributes panel for every shape: points, edges, polygons and **triangles** (what a game engine draws), whether the surface is closed or open, and a check for problems (edges shared by three or more faces, faces facing the wrong way, edges with no length, faces with no area, loose points). Underneath, the app now knows how every mesh connects (edge loops and rings, borders, separate pieces), ready for the selection tools.
- **Saving and opening projects** (`.avtl` files): File → New, Open…, Open recent, Save and Save As… (Ctrl + N, O, S and Shift + S). Projects keep every object, its settings and edits, plain meshes, and the selection. Saving is safe (the old file stays until the new one is complete, and the previous save is kept as a `.bak` backup). The app asks before unsaved changes would be lost (Enter saves, Ctrl + D doesn't, Esc cancels), and the title shows the project's name with `*` for unsaved changes. A project given on the command line opens at start. New manual page: Saving and opening projects.
- A **Settings** window (**Edit → Settings…**, or **Ctrl + ,**) gathers every app-wide preference in one place: accent colour and metres/centimetres (Interface), navigation controls and grid lines (Viewport), and undo memory (Undo). **Restore standard** puts them all back. New manual page: Settings.
- Undo is now limited by memory instead of being kept forever: steps are kept until they fill 2 GB (the standard), and then the oldest are forgotten first. There's still no limit on the number of steps. **Edit → Undo memory** shows how much is in use and offers 256 MB to 8 GB or Unlimited; the choice is remembered. The status bar says once when old steps start being forgotten.
- **Make editable** (**C**, also in the Mesh menu and the Objects panel's Object and right-click menus): turns the selected shapes into plain meshes, baking in their settings. Undo brings the settings back. The Attributes panel shows a mesh's points and polygons, and the Objects panel gives it the polygon icon.
- Shapes now keep edits apart from their settings: the modelling tools coming in Phase 3 will record edits on top of a shape, so its settings (a cube's segments, say) stay editable afterwards and the edits are made again on the new shape. The Attributes panel will count a shape's edits and flag any that no longer fully fit.
- Small step arrows beside every value in the Attributes panel: click to add or take away one step (1 cm, or 1 segment) for fine tuning; hold Shift for ten.
- Press **B** for a list of shapes by the mouse: **B** again moves down it, **Shift + B** back up, and **Enter** or **Space** creates the highlighted shape. Clicking a shape in the list creates it too; **Esc** closes it.
- Cylinder and cone ends, and discs, can be cut into rings (**Cap segments**, **Ring segments**), and their middle can be one n-gon (standard) or triangles meeting at a **Centre point**.
- **Copy settings** and **Paste … settings** in the Objects panel's right-click menu.
- The Attribute Manager works: select one object to see its name, a Visible switch, and for a shape its settings (sizes in centimetres, segments, sides, closed ends and so on). Drag a value to watch the shape change, or click and type one; each drag or typed value is one undo step. Edit → Copy settings and Paste settings give several shapes of the same kind the same settings. New manual page: The Attribute Manager.
- Primitives: Create → Cube, Plane, Cylinder, Sphere, Disc, Tube or Cone (or the blue shape buttons on the command palette). Each stands on the ground at the centre of the scene, with crisp edges where it should have them and smooth curves elsewhere, and is listed in the Objects panel. Selected objects are outlined in the accent colour in the viewport, and the Attributes panel names the selection. New manual page: Primitives.
- The Object Manager works: the scene's objects are listed as a tree (for now the test cube, plus any groups you make). Click to select (Ctrl adds, Shift selects a run), double-click to rename, drag to put objects inside each other or reorder them, click the eye to show or hide, and press Delete to delete. Create → Group (or the palette's group button) makes an empty group; Alt + G puts the selection in a new group and Shift + G takes it out again. Right-click a row for these commands. F now frames the selected objects. New manual page: Objects and the Object Manager.
- Undo and redo for every change to the scene: Ctrl + Z, Ctrl + Y (or Ctrl + Shift + Z), the arrows on the command palette, or the Edit menu, which names what would be undone.
- Top, front and right views beside the perspective one, and a four-view layout: **F1** to **F4** show one view (perspective, top, right, front), **F5** shows all four, and a middle click switches between one and four (also the square button on the viewport's menu line, **Panel → Four views** and **Cameras**). The top, front and right views are flat: no perspective, so sizes read true; they pan and zoom but don't orbit, and their grid faces you. The last view clicked is the active one, marked with a thin accent line. Each view keeps its own camera, and the layout is remembered.
- Frame all (**H**) and frame selected (**F**), also in the View menus: the view keeps looking the same way and glides until everything (or the selection) fits, with a little room around it. Until objects can be selected, F frames everything.
- Orbit and zoom around the point under the mouse: grab the model where you want to turn around it, or point at a spot and roll the wheel to close in on it. A small ring marks the point while you orbit, and zooming while holding a button keeps to that point. On by default; switched off in the navigation controls or Options, orbiting and zooming work around the middle of the view.
- Navigation controls to choose from, with the button in the bottom-right corner of the viewport (or Options → Navigation controls…): **Mouse only** is now the standard (right drag orbits, middle drag pans, the wheel zooms), and Alt + mouse (as before), Mouse only with middle zooming, Middle pans with Alt orbiting, and Middle orbits can be picked instead. Alt + mouse can keep working in every scheme, and orbiting can be inverted. The status bar shows the controls in use, and the choice is remembered.
- Accent colour: red as standard, or Blue, Marine green, Plum, Olive or Petrol blue, from Window → Accent colour. Remembered between sessions. Each colour has a small swatch next to its name.
- Layout tabs on the right of the menu bar: Standard, with Modelling and UV Edit to come.
- Move around the 3D view as in Cinema 4D: hold Alt and drag with the left mouse button to orbit, the middle button to pan, the right button to zoom; the mouse wheel zooms too. The status bar shows the controls, and **View → Reset view** brings back the opening view. New manual page: Navigating the view.
- A grid detail switch at the bottom of the left palette (also **View → Fine grid lines**): the grid now shows only 1 m lines by default, for a calmer ground, and the 10 cm lines come back when you switch it on. The choice is remembered.
- The 3D viewport: a calm grey background, a grid on the ground (10 cm and 1 m lines, fading softly into the distance), the red X and blue Y axes, and a softly lit 2 m test cube. Without a usable graphics card, the viewport says why instead.
- The calm dark look from the mockups: gentle greys, a warm orange for the active mode, the Inter font, and a viewport that fills its panel edge to edge. The app stays dark whatever your system prefers.
- The app window, laid out like Cinema 4D: menu bar, command palette, mode palette, viewport, Attributes (top right), Objects (below it), Coordinates (under the viewport) and a status bar.
- Panels can be dragged by their tabs, stacked and resized; **View → Reset panel layout** brings the standard layout back. The layout and window size are remembered between sessions.
- Pick a mode (Model, Object, Points, Edges, Polygons, UV) from the left palette or the Modes menu.
- Commands that aren't built yet are listed greyed out, with the phase that brings them.
- File → Quit (Ctrl+Q) and Help → About.
- Command-line options `--version` and `--help`.
- User manual: Interface tour page.
- Project foundations: Cargo workspace with 9 single-purpose crates in five layers, with generic names so renaming the app never touches them.
- `layer-check` tool whose tests enforce the architecture rules on every change.
- Workspace-wide safety lints: no `unsafe` code, no `unwrap`/`expect`/`panic!` in library code, documentation required on every public item.
- Continuous integration: formatting, lint, tests and docs on Windows, Linux and macOS; license and vulnerability audit; downloadable builds for all three platforms; changelog check on pull requests.
- API reference published to GitHub Pages from `main`.
- Documentation: README with progress tracker, architecture overview, contributing guide, security policy, decision records, pull request and issue templates, user manual structure with a concepts page.
- Program skeleton that reports its name and version, with the app's name kept in one place (`branding.rs`).
