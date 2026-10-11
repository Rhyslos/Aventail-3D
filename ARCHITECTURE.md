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

Planned later: a booleans crate (layer 1, wrapping manifold), a `script` crate (layer 2, Lua scripting through the same commands as everything else; Phase 9, see [decision 0011](docs/decisions/0011-lua-scripting.md)), and animation support inside `scene` (Phase 10).

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
2. **Window** (`app` with eframe): open the window, named and identified through `branding.rs`, apply the theme (`ui::theme`; see [decision 0009](docs/decisions/0009-theme.md)), and restore the panel layout saved last time. See [decision 0008](docs/decisions/0008-window-and-panels.md).
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

## Viewport

The viewport holds four views (perspective, top, front, right), each with its own `tools::Navigator`; it shows the active one, or all four in a 2 × 2 layout. Each frame, for each view shown:

1. **Navigate** (`ui` → `tools`): the panel turns this frame's drags (with the keys held) and wheel turns into plain moves on a `tools::Navigator` (the camera and its pivot); `tools::controls` decides which movement a button and keys start, for the scheme the user picked. `tools` knows nothing about egui.
2. **Ask** (`ui`): the panel works out its size in physical pixels and the camera's matrices (`tools::Camera`), and asks a `ViewportRenderer` for an image, with the point the camera looks at (the grid is drawn around it) and display options such as the grid level (1 m, 50 cm or 10 cm lines).
3. **Draw** (`render`, through `app`'s `GpuViewport`): the backdrop with its horizon, faces, grid and lines are drawn into that view's own offscreen image (one per view, keyed by `ViewportView::slot`) on the window's GPU device. Flat (orthographic) views get a plain backdrop and a grid in the plane facing them.
4. **Show** (`ui`): the image is drawn into the panel as an egui texture, with the view's label on top.

While the mouse hovers over a view, the panel also asks the renderer what surface is under it (`ViewportRenderer::pick`); the answer arrives on a later frame and is used as that view's pivot when an orbit or zoom starts; only the view under the mouse asks, one question at a time ([decision 0017](docs/decisions/0017-picking-the-point-under-the-mouse.md)).

In the Points, Edges and Polygons modes, the element under the mouse is found on the CPU instead (`ui::components` → `tools::PickScene`), from the hovered view's camera and mouse position, after the views are drawn. The workbench turns the selected objects' elements into a small `ComponentOverlay`, which the app uploads as a separate `render::Overlay` layer; a mouse move rebuilds only that layer, never the scene's shape ([decision 0027](docs/decisions/0027-component-modes.md)).

Selecting follows the same path ([decision 0029](docs/decisions/0029-selection.md)): the viewport only records what the mouse did (`selecting::Gesture`), and once the frame's drawing is known the workbench's `Selector` looks it up in the same `PickScene` and changes the document. Whole objects are selected directly, as in the Object Manager. Elements are kept per object in `scene::Document` and changed through `select_elements`, so every change is an undo step; the selection rules themselves (all, invert, grow, shrink, loops, rings) are pure functions in `mesh::selection`.

Rendering sits behind the `ViewportRenderer` trait, so the panel runs in tests with no GPU. See [decision 0013](docs/decisions/0013-viewport-rendering.md).

## Axes

**Z is up**, right-handed: X and Y form the ground plane. Every crate that deals with positions (`mesh`, `scene`, `render`, `tools`) uses this; only `fileio` converts, to Y-up for glTF. See [decision 0012](docs/decisions/0012-z-up-axes.md).

## The scene

- The single source of truth for an open project: the object hierarchy, each object's base and its operation stack. See [decision 0006](docs/decisions/0006-base-plus-operation-stack.md).
- An object is a **base** (a parametric primitive, an imported mesh, or an editable mesh) plus a **stack of layers**, each holding recorded operations. Base parameters stay editable; the stack replays on top. Built so far ([decision 0021](docs/decisions/0021-object-stack-foundation.md)): the base is the object's kind (a primitive or a plain mesh), the stack a list of layers with one implicit *Edits* layer, and `Scene::evaluated` works out the final shape from a per-layer cache inside the scene. The interface draws and frames evaluated shapes.
- Every mesh element has a stable ID that is never reused. Operations create IDs deterministically, so replaying an unchanged history gives identical IDs.
- The scene changes only through commands, which gives undo, redo and future scripting from one mechanism. Projects are saved and opened by `fileio` through a file model kept separate from the scene's types, so the scene can change without breaking old files ([decision 0024](docs/decisions/0024-saving-projects.md)); the interface asks the app to save or open, and the app shows the dialogs. Undo history is bounded by a memory budget, not a step count: 2 GB as standard, oldest steps forgotten first, the newest always kept ([decision 0022](docs/decisions/0022-undo-memory-budget.md)).
- **Built so far** ([decision 0018](docs/decisions/0018-scene-document-and-undo.md)): `scene::Document` holds the `Scene` (a tree of objects with stable ids that are never reused), the undo history and the selection. `Document::execute` carries out a `Command` as small reversible edits (insert, remove, relocate, rename, show/hide) and records them as one step; `undo` applies their opposites in reverse order. The workbench owns the document: the Object Manager, menus and keys hand it commands, and the viewport reads a summary: the shown objects' meshes, world placements and selection (rebuilt only when the scene or selection changes), and the boxes to frame. An object's shape comes from its kind: a primitive's settings build a `mesh::Mesh` ([decision 0020](docs/decisions/0020-primitives.md)); the app turns the meshes into one GPU shape when the drawing's version changes.
- Transforms are stored relative to the parent; world transforms are always derived.

## Files

| Format | Read | Write |
| --- | --- | --- |
| `.avtl` (projects; versioned, compressed, crash-safe; see decisions [0007](docs/decisions/0007-rhy-project-format.md) and [0010](docs/decisions/0010-avtl-file-extension.md)) | Planned | Planned |
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
| A user's own tool or command (Phase 9) | A Lua script, through `script` |

## Security

- Files from strangers are hostile: every reader checks sizes before allocating and has a fuzz test.
- No usernames or local paths are written into exported files. No telemetry.
- Dependencies are audited for licenses and known vulnerabilities on every change (`cargo deny`).

## Decisions

Why things are the way they are: see [docs/decisions](docs/decisions).
