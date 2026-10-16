# Decision records

Short notes on major technical choices: the problem, the options, the choice and why. When a later change touches an area, its record explains why things are the way they are.

- Numbered files, never renumbered. A reversed decision gets a new record that says which one it replaces.
- Copy `template.md` to start a new one.
- Decisions made before this folder existed are also summarised in the decision log in [DESIGN.md](../../DESIGN.md) §13.

| # | Decision | Status |
| --- | --- | --- |
| [0001](0001-record-decisions.md) | Record major decisions | Accepted |
| [0002](0002-rust-wgpu-egui.md) | Rust, wgpu and egui | Accepted |
| [0003](0003-layered-crates.md) | Layered single-purpose crates with generic names | Accepted |
| [0004](0004-gpl-license.md) | GPL 3.0 license, permissive dependencies | Accepted |
| [0005](0005-safety-lints.md) | No unsafe, no panics in library code | Accepted |
| [0006](0006-base-plus-operation-stack.md) | Objects are a base plus an operation stack | Accepted |
| [0007](0007-rhy-project-format.md) | The project format (extension replaced by 0010) | Accepted, extension replaced |
| [0008](0008-window-and-panels.md) | App window: eframe, egui_dock, drawn icons | Accepted |
| [0009](0009-theme.md) | The calm dark theme and the Inter font | Accepted, colours replaced by 0014 |
| [0010](0010-avtl-file-extension.md) | Project files use `.avtl` | Accepted |
| [0011](0011-lua-scripting.md) | Scripting in Lua (Luau, through mlua) | Accepted |
| [0012](0012-z-up-axes.md) | Z is up | Accepted |
| [0013](0013-viewport-rendering.md) | Viewport rendering | Accepted |
| [0014](0014-studio-look-and-accent.md) | A darker studio look, with a choice of accent colour | Accepted |
| [0015](0015-navigation-schemes.md) | A choice of navigation controls | Accepted |
| [0016](0016-controls-follow-halberd.md) | Keyboard and mouse controls follow Halberd | Narrowed by 0019 |
| [0017](0017-picking-the-point-under-the-mouse.md) | Finding the point under the mouse | Accepted |
| [0018](0018-scene-document-and-undo.md) | The scene document: an object tree changed by undoable commands | Accepted |
| [0019](0019-only-the-tool-keys-come-from-halberd.md) | Only the tool keys come from Halberd | Accepted |
| [0020](0020-primitives.md) | Primitives: how they are built, placed and drawn | Accepted |
| [0021](0021-object-stack-foundation.md) | The object stack foundation: how edits are stored, evaluated and baked | Accepted |
| [0022](0022-undo-memory-budget.md) | Undo history bounded by a memory budget | Accepted |
| [0023](0023-one-settings-window.md) | One Settings window for app-wide preferences | Accepted |
| [0024](0024-saving-projects.md) | Saving projects: the `.avtl` file in practice | Accepted |
| [0025](0025-mesh-topology.md) | Mesh topology: a derived index over a face list | Accepted |
| [0026](0026-windows-keyboard-focus.md) | One small unsafe exception: giving the window the keyboard on Windows | Accepted |
| [0027](0027-component-modes.md) | Points, Edges and Polygons modes: what is edited, and finding what is under the mouse | Accepted |
| [0028](0028-ask-about-controls.md) | Ask the project lead about controls before building a feature | Accepted |
| [0029](0029-selection.md) | Selection: controls, where element selections live, and selecting through | Accepted |
| [0030](0030-transform-gizmos.md) | Move, rotate and scale: gizmos, drags as one step, snapping and the Coordinates panel | Accepted |
| [0031](0031-project-format-version-2.md) | Project file format version 2 | Accepted |
| [0032](0032-edits-find-their-points-again.md) | Edits find their points again when a shape's settings change | Accepted |
| [0033](0033-heavy-mesh-safeguard.md) | Heavy meshes: a safeguard, checked at phase gates | Accepted |
| [0034](0034-uv-phase-after-modelling.md) | UVs as one phase after modelling, aiming to be the best UV tools | Accepted |
| [0035](0035-heavy-meshes.md) | Heavy meshes: shared parts, flat lists, kept indexes, background work | Accepted |
| [0036](0036-autosave.md) | Autosave: backup copies in the app's own folder, found again by file locks | Accepted |
| [0037](0037-tool-menu-and-command-list.md) | The tool menu (E) and one command list | Accepted |
| [0038](0038-extrude-and-inset.md) | Extrude and inset: drag anywhere, Keep together, ids that stay, file format version 3 | Accepted |
| [0039](0039-drags-follow-the-pull.md) | Drags follow the pull: measured from what they change, a fixed direction only as fallback | Accepted |
| [0040](0040-bevel.md) | Bevel: edges and points, drag away from them, file format version 4 | Accepted |
| [0041](0041-saves-that-always-reopen.md) | Saves that always reopen: verified saves, one safe-write helper, gaps closed at the source | Accepted |
| [0042](0042-fast-edits.md) | Fast edits: checkpoints inside a layer, bevel without quadratic loops, finding edits again through a grid, topology in undo memory | Accepted |
| [0043](0043-attribute-layers-and-the-builder.md) | Attribute layers (point, edge, polygon, corner), hard edges as the first; the mesh builder; deleting polygons | Accepted |
| [0044](0044-edits-that-hold.md) | Edits that hold: ids and switches, ids trusted when the structure is the same, file format version 5 (stretching new rows tried and dropped) | Accepted |
| [0045](0045-limits-for-files.md) | Limits for files (unpacked size, a memory budget while reading, counts) and a size guard for shapes; recovery that can't loop | Accepted |
| [0046](0046-our-own-names.md) | Our own names: Attributes panel, Objects panel, Paint select, Snap to | Accepted |
| [0047](0047-loop-cut.md) | Loop cut: hover, click or drag; Cuts in the Attributes panel; Z frames, Shift + X/Y/Z lock; file format version 6 | Accepted |
| [0048](0048-ctrl-wheel-changes-a-tools-count.md) | Ctrl + wheel changes a tool's count (Bevel's segments, Loop cut's cuts, and every tool to come) | Accepted |
| [0049](0049-slit.md) | Slit: a line of points over the shape, cut as seen (or through); file format version 7 | Accepted |
| [0050](0050-pipe-bends.md) | Pipe bends: an extrude that turns a corner and keeps its size (smooth elbow or mitre joint); file format version 8 | Accepted |
| [0051](0051-connect-bridge-weld.md) | Connect (J), Bridge (C or drag) and Weld (M, Ctrl + M); file format version 9 | Accepted |
| [0086](0086-groove.md) | Groove: a thin line cut in or raised out along edges on K, made as a half-width bevel then a pushed strip, straight or sloped sides, clean ends; file format version 24 | Accepted |
| [0085](0085-bevel-control.md) | Bevel control: even width, skew with flip side (following each chain of edges), stop at overlaps, weld new points within a distance; file format version 23 | Accepted |
| [0084](0084-view-dice.md) | The view dice (faces, edges, corners; Ctrl + click the opposite side) replaces the corner axes; Ctrl while orbiting snaps to its views; Display → Switch to orthographic when clicking the dice | Accepted |
| [0083](0083-groove-and-bevel-control.md) | Groove (K) and more control over Bevel (weld overlapping points, skew, even width, stop at overlaps); planned for right after the open pull requests | Accepted (planned) |
| [0082](0082-edits-keep-their-shape-on-stretched-objects.md) | Extrudes, insets and bevels keep their shape on stretched objects, through one way to work any edit out as seen; file format version 22 | Accepted |
| [0081](0081-bends-keep-their-shape-on-stretched-objects.md) | Bends keep their shape on stretched objects (the bend records the object's stretch); polygons bent each on their own keep their size; file format version 21 | Accepted |
| [0079](0079-splash-screen.md) | The splash screen: name, version, progress and the start-up step under way, in the main window's first frames | Accepted |
| [0078](0078-kit-export.md) | Kit export (each selected piece its own file, pivot at the origin) and a Collision setting Godot reads from names; file format version 20 | Accepted |
| [0077](0077-bake-check.md) | The bake check (model part): File → Check for baking…, problems in plain words with Select and one-click fixes; `mesh::bake` | Accepted |
| [0076](0076-engine-export-paths.md) | Engine export paths: a game project folder per project, the engine found from it, Export on every save; only the app's own files replaced; file format version 19 | Accepted |
| [0075](0075-export-presets.md) | Export presets: Made for Godot (glTF) or Substance Painter (OBJ, metres, Y up, triangles) in the Export window | Accepted |
| [0074](0074-material-slots.md) | Material slots: a slot number per polygon, a named, coloured list per object; Assign in the Attributes panel, Select by material; colours in the viewport; OBJ `.mtl` and glTF materials; file format version 18 | Accepted |
| [0073](0073-mikktspace-tangents.md) | Exports cut polygons the viewport's way; MikkTSpace tangents in glTF for shapes with UVs (`bevy_mikktspace`) | Accepted |
| [0072](0072-gltf-export.md) | glTF export: binary `.glb`, each object a node at its pivot, metres, Y up, triangles; `serde_json` | Accepted |
| [0071](0071-obj-export-and-import.md) | OBJ export and import: File menu entries, shading as seen, exports prepared once for every format | Accepted |
| [0080](0080-navigation-feel.md) | Navigation feel: a pivot you can see over empty space, pans at the grabbed depth, steady close zoom (through surfaces), no lost movement, an optional turning-point ring | Accepted |
| [0070](0070-extrude-freedom.md) | Extrude straight out as standard, with direction, end size, steps and a flat end; the Scale squares scale equally; file format version 17 | Accepted |
| [0069](0069-orthographic-view.md) | An orthographic view that orbits, on P; flat views orbit into it | Accepted |
| [0068](0068-pivot-editing-and-scene-units.md) | Pivot editing in the Object mode (an offset between pivot and shape; Pivot to centre, world origin, object; Object to world origin) and units saved with the project; file format version 16 | Accepted |
| [0067](0067-clearer-names.md) | Clearer names (Fix normals, Isolate, Duplicate as instance, Select same facing) and a README introduction for game developers | Accepted |
| [0066](0066-clickable-axes.md) | Clickable corner axes: click X, Y or Z to look along it (again: the other side); Cavity beside Hard edges in the Display menu; heavy shapes reshaded in the background while dragging | Accepted |
| [0065](0065-cavity.md) | Cavity: ridges brightened and creases darkened from the surfaces' facing on screen, on as standard; Automatic hard edges moved to the Mesh menu | Accepted |
| [0064](0064-automatic-hard-edges.md) | Automatic hard edges: a switch (on as standard; in the Mesh menu since 0065) shading every shape by angle after every change; edges set by hand kept | Accepted |
| [0063](0063-wireframe.md) | Wireframe (Shaded, Shaded with wires, Wireframe) from a button beside the corner axes, which move to the bottom left; the choices for cavity, clickable axes and automatic hard edges | Accepted |
| [0062](0062-spirals.md) | Spirals: bends past one turn (up to 3600°) climb clear of the loop before, plus Rise per turn; Segments per turn; mirrored copies wind the other way; file format version 15 | Accepted |
| [0061](0061-drawn-keyboard.md) | The Keys window draws a keyboard: lit keys with situation colours, point or click for a key's jobs, search, mouse actions; seven layouts, guessed from the system language | Accepted |
| [0060](0060-mode-keys-put-tools-away.md) | With a modelling tool in use, 1, 2 and 3 (and the mode buttons) put it away and select in that mode | Accepted |
| [0059](0059-symmetry-for-every-edit.md) | Symmetry for every edit (mirrored edits, together or as copies) and the Symmetry menu (axes, mirror through pivot, middle or world, offsets); file format version 14 | Accepted |
| [0058](0058-closed-slit.md) | Closed slits: click the first point to close the line; a loop inside a polygon cuts its shape in, the ring round it cut in two; saved as the first point again, no format change | Accepted |
| [0057](0057-after-phase-4-testing.md) | Fixes and keys after Phase 4 testing: Tab steps back, Connect on A, Fill hole on J, click selects in Weld and Bridge, weld snap, bend Angle resets, Dissolve warns on sharp corners, proper triangulation | Accepted |
| [0056](0056-shortcut-viewer.md) | The keyboard shortcut viewer: Keys on the menu bar, every key from the app's own constants | Accepted |
| [0055](0055-basics.md) | The basics: hide and isolate (Shift/Ctrl/Alt + H), copies and instances (Ctrl/Alt + C and V), slide (Alt + Move drag), split off and join; file format versions 12 and 13 | Accepted |
| [0054](0054-symmetry.md) | Symmetry: a live mirror while moving, rotating and scaling points; palette switch, X/Y/Z in the Attributes panel | Accepted |
| [0053](0053-hard-and-soft-edges.md) | Hard and soft edges by hand or by angle; Display menu shows hard edges and normals; file format version 11 | Accepted |
| [0052](0052-clean-up.md) | Clean-up: Delete deletes, Backspace dissolves; Fill hole, Flip normals, Fix facing, Select flipped, Select similar facing; file format version 10 | Accepted |
