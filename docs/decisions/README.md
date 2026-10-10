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
| [0007](0007-rhy-project-format.md) | The `.rhy` project format | Accepted |
| [0008](0008-window-and-panels.md) | App window: eframe, egui_dock, drawn icons | Accepted |
