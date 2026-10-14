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
