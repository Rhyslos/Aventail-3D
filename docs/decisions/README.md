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
