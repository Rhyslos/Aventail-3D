# Architecture

Aventail 3D is a Rust program split into small, single-purpose crates: the mesh at the bottom, the scene document in the middle, rendering and interface on top. **Lower layers never depend on higher ones**, so each piece can be built, tested and replaced on its own. The `layer-check` tool enforces this on every change.

Crate names are generic on purpose (no product name), so renaming the app never touches the code's structure; see [RENAMING.md](RENAMING.md).

## Layers

| Layer | Name | Crates | Rule |
| --- | --- | --- | --- |
| 4 | App | `app` | Wires everything together and owns the frame loop. No feature logic. Home of `branding.rs`. |
| 3 | Front end | `render`, `tools`, `ui` | Reads the scene; changes it only through commands. |
| 2 | Services | `fileio` | Talks to the outside world: project files, OBJ, glTF, the settings file. |
| 1 | Core | `scene`, `uv`, `config` | The scene document, UV mapping and settings. No files, GPU or UI. |
| 0 | Geometry | `mesh` | The polygon mesh and its operations. Knows nothing about the app. |

Each crate's `README.md` states its purpose and what it **must never** do. Read it before changing the crate.

Planned later: a booleans crate (layer 1, wrapping manifold) and animation support inside `scene` (Phase 9).

## How a frame runs

The interface thread draws 60+ frames a second and must never wait on slow work.

1. **Input:** winit delivers mouse and keyboard events.
2. **Interpret:** the active tool (`tools`) turns input into an intent ("extrude these faces by 50 cm").
3. **Command:** the intent becomes an operation plus a command, applied to the scene. Every command can undo itself.
4. **Evaluate:** the object's stack is re-evaluated from the changed layer upward, using cached results below it.
5. **Sync:** the renderer updates GPU data only for meshes that changed.
6. **Draw:** viewports render with wgpu; panels (egui) draw on top.

Slow work (large unwraps, booleans, exports) runs on background threads. **Background jobs never touch the scene.** They send results as messages, and the interface thread applies them at the start of the next frame.

## Startup

1. **Command line** (`app`): `--version` and `--help` print and exit; anything unknown is explained, with exit code 2.
2. **Window** (`app` with eframe): open the window, named and identified through `branding.rs`, and restore the panel layout saved last time. See [decision 0008](docs/decisions/0008-window-and-panels.md).
3. **Each frame** (`ui`): the workbench draws the menu bar, the command palette, the mode palette, the status bar and the docked panels, and returns any actions (such as Quit) for the app to carry out.
4. **Closing**: eframe saves the window size and the panel layout (`app.ron`, in a per-user folder named after `APP_ID`).

## Window and panels

The `ui` crate's workbench owns everything inside the window:

```text
┌──────────────────────────────────────────────────────────────┐
│ Menu bar                                                     │
│ Command palette (tools, primitives)                          │
├──┬─────────────────────────────────────┬─────────────────────┤
│M │ Viewport                            │ Attributes          │
│o │                                     ├─────────────────────┤
│d │                                     │ Objects             │
│e ├─────────────────────────────────────┤                     │
│s │ Coordinates                         │                     │
├──┴─────────────────────────────────────┴─────────────────────┤
│ Status bar                                                   │
└──────────────────────────────────────────────────────────────┘
```

The four middle panels are dockable (egui_dock); the bars and palettes around them are fixed. Palette icons are drawn in code (`ui/src/icons.rs`). The workbench never touches the window or files itself: it returns actions for `app` to carry out.

## The scene

- The single source of truth for an open project: the object hierarchy, each object's base and its operation stack. See [decision 0006](docs/decisions/0006-base-plus-operation-stack.md).
- An object is a **base** (a parametric primitive, an imported mesh, or an editable mesh) plus a **stack of layers**, each holding recorded operations. Base parameters stay editable; the stack replays on top.
- Every mesh element has a stable ID that is never reused. Operations create IDs deterministically, so replaying an unchanged history gives identical IDs.
- The scene changes only through commands, which gives undo, redo and future scripting from one mechanism. Undo history is bounded by a memory budget, not a step count.
- Transforms are stored relative to the parent; world transforms are always derived.

## Files

| Format | Read | Write |
| --- | --- | --- |
| `.rhy` (projects; versioned, compressed, crash-safe; see [decision 0007](docs/decisions/0007-rhy-project-format.md)) | Planned | Planned |
| OBJ | Planned | Planned |
| glTF / GLB | Later | Planned |
| FBX | No | Late, optional |
| Settings file | Planned | Planned |
| `app.ron` (window size and panel layout; written by eframe) | Yes | Yes |

## Extension points

| To add | Plug into |
| --- | --- |
| A modelling tool | The `Tool` interface in `tools` |
| A mesh operation | A pure function in `mesh` |
| An edit | A `Command` in `scene` |
| A primitive | A primitive kind in `scene` |
| A panel | A panel registered with the dock in `ui` |
| A file format | `fileio` |

## Security

- Files from strangers are hostile: every reader checks sizes before allocating and has a fuzz test.
- No usernames or local paths are written into exported files. No telemetry.
- Dependencies are audited for licenses and known vulnerabilities on every change (`cargo deny`).

## Decisions

Why things are the way they are: see [docs/decisions](docs/decisions).
