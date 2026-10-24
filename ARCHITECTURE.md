# Architecture

Aventail 3D is a Rust program split into small, single-purpose crates: the mesh at the bottom, the scene document in the middle, rendering and interface on top. **Lower layers never depend on higher ones**, so each piece can be built, tested and replaced on its own. The `layer-check` tool enforces this on every change.

Crate names are generic on purpose (no product name), so renaming the app never touches the code's structure; see [RENAMING.md](RENAMING.md).

## Layers

| Layer | Name | Crates | Rule |
| --- | --- | --- | --- |
| 4 | App | `app` | Wires everything together and owns the frame loop. No feature logic. Home of `branding.rs`. |
| 3 | Front end | `render`, `tools`, `ui` | Reads the scene; changes it only through commands. |
| 2 | Services | `fileio` | Talks to the outside world: project files, OBJ, glTF, the settings file. |
| 1 | Core | `scene`, `uv`, `booleans`, `config` | The scene document, UV mapping, booleans and settings. No files, GPU or UI. |
| 0 | Geometry | `mesh`, `words` | The polygon mesh and its operations, and the language layer every text goes through. Know nothing about the app; `words` depends on nothing at all, and `mesh` only on it. |

Each crate's `README.md` states its purpose and what it **must never** do. Read it before changing the crate.

Planned later: a `script` crate (layer 2, scripting (LustScript and Lua) through the same commands as everything else; Phase 15, see [decision 0011](docs/decisions/0011-lua-scripting.md) and [decision 0119](docs/decisions/0119-roadmap-after-the-uvs.md)), and animation support inside `scene` (Phases 17 and 18). Every text a person reads, in any crate, goes through one language layer, the `words` crate, with English as the one language package (`crates/words/lang/en.txt`) for now ([decision 0117](docs/decisions/0117-language-support.md), [decision 0132](docs/decisions/0132-one-language-layer.md), [decision 0134](docs/decisions/0134-shared-language-crate.md)).

## How a frame runs

The interface thread draws 60+ frames a second and must never wait on slow work.

1. **Input:** winit delivers mouse and keyboard events.
2. **Interpret:** the active tool (`tools`) turns input into an intent ("extrude these faces by 50 cm").
3. **Command:** the intent becomes an operation plus a command, applied to the scene. Every command can undo itself.
4. **Evaluate:** the object's stack is re-evaluated from the changed layer upward, using cached results below it. Inside the changed layer, it resumes from the longest run of unchanged edits, at a kept shape (after all the layer's edits, or all but the last), so adding, dragging or changing the last edit applies only that edit ([decision 0042](docs/decisions/0042-fast-edits.md)).
5. **Sync:** the renderer updates GPU data only for meshes that changed, and for a mesh whose points only moved, only the vertices around them.
6. **Draw:** viewports render with wgpu; panels (egui) draw on top.

Slow work (large unwraps, booleans, exports) runs on background threads. **Background jobs never touch the scene.** They send results as messages, and the interface thread applies them at the start of the next frame.

## Startup

1. **Command line** (`app`): `--version` and `--help` print and exit; anything unknown is explained, with exit code 2; `--` ends the options, so a file name may start with `-`.
2. **Window** (`app` with eframe): open the window, named and identified through `branding.rs`, apply the theme (`ui::theme`; see [decision 0009](docs/decisions/0009-theme.md)), and restore the panel layout saved last time. See [decision 0008](docs/decisions/0008-window-and-panels.md).
3. **Each frame** (`ui`): the workbench draws the menu bar, the command palette, the mode palette, the status bar and the docked panels, and returns any actions (such as Quit) for the app to carry out.
4. **Closing**: eframe saves the window size and the panel layout (`app.ron`, in a per-user folder named after `APP_ID`).

## Window and panels

The `ui` crate's workbench owns everything inside the window:

```text
┌──────────────────────────────────────────────────────────────┐
│ Menu bar                                                     │
│ Command palette (tools, primitives)                          │
├──┬──────────────────────────────────┬──┬─────────────────────┤
│M │ Viewport                         │Mo│ Attributes          │
│o │                                  │di├─────────────────────┤
│d │                                  │fs│ Objects, Edit layers│
│e ├──────────────────────────────────┴──┤                     │
│s │ Coordinates                         │                     │
├──┴─────────────────────────────────────┴─────────────────────┤
│ Status bar                                                   │
└──────────────────────────────────────────────────────────────┘
```

The five middle panels are dockable (egui_dock; Edit layers is a tab beside Objects); the bars and palettes around them are fixed. The modifiers palette is drawn inside the viewport's panel, on its right edge, so it always sits between the viewport and the panels beside it ([decision 0091](docs/decisions/0091-edit-layers.md)). Palette icons are drawn in code (`ui/src/icons.rs`, with one file per family of icons in `ui/src/icons/`). The workbench never touches the window or files itself: it returns actions for `app` to carry out.

## Viewport

The viewport holds four views (perspective, top, front, right), each with its own `tools::Navigator`; it shows the active one, or all four in a 2 × 2 layout. Each frame, for each view shown:

1. **Navigate** (`ui` → `tools`): the panel turns this frame's drags (with the keys held) and wheel turns into plain moves on a `tools::Navigator` (the camera and its pivot); `tools::controls` decides which movement a button and keys start, for the scheme the user picked. `tools` knows nothing about egui.
2. **Ask** (`ui`): the panel works out its size in physical pixels and the camera's matrices (`tools::Camera`), and asks a `ViewportRenderer` for an image, with the point the camera looks at (the grid is drawn around it) and display options such as the grid level (1 m, 50 cm or 10 cm lines).
3. **Draw** (`render`, through `app`'s `GpuViewport`): the backdrop with its horizon, each object's mesh (kept on the GPU in its own space and placed in the world), the grid and lines are drawn into that view's own offscreen image (one per view, keyed by `ViewportView::slot`) on the window's GPU device. Flat (orthographic) views get a plain backdrop and a grid in the plane facing them.
4. **Show** (`ui`): the image is drawn into the panel as an egui texture, with the view's label on top.

While the mouse hovers over a view, the panel also asks the renderer what surface is under it (`ViewportRenderer::pick`); the answer arrives on a later frame and is used as that view's pivot when an orbit or zoom starts; only the view under the mouse asks, one question at a time ([decision 0017](docs/decisions/0017-picking-the-point-under-the-mouse.md)).

In the Points, Edges and Polygons modes, the element under the mouse is found on the CPU instead (`ui::components` → `tools::PickScene`), from the hovered view's camera and mouse position, after the views are drawn. The workbench turns the selected and hovered elements into a small `ComponentOverlay`, which the app uploads as a separate `render::Overlay` layer; a mouse move rebuilds only that layer, never the scene's shapes ([decision 0027](docs/decisions/0027-component-modes.md)). The edited objects' own edges and points are drawn with their meshes.

**Heavy shapes** ([decision 0035](docs/decisions/0035-heavy-meshes.md)). The work an action costs follows what it changes, not the size of the mesh:
- **Meshes** share their parts between copies until one changes them: points in chunks, faces in one flat list, attribute layers (hard edges among them). Tools make geometry through the mesh builder, which carries every attribute layer over ([decision 0043](docs/decisions/0043-attribute-layers-and-the-builder.md)). A recorded move copies only the chunks it touches, and `Mesh::moved_since` tells caches what moved. Connections (`Mesh::topology`) and the box around the points are kept with the parts they depend on.
- **Picking:** `tools::MeshPick` indexes one mesh in its own space (bounding volume hierarchies over its triangles, points, edges and polygon middles). The `ui` `ComponentTracker` keeps one per object, builds new heavy ones on another thread (`prepare`), updates them around moved points, and keeps its `PickScene` as it was while a drag is under way.
- **Drawing:** the app's `GpuViewport` keeps each object's `render::MeshGeometry` on the GPU in its own space (`ViewportRenderer::set_mesh`), works out heavy new ones on another thread, uploads only the vertices around moved points (`update_mesh`), and places objects each frame (`set_draws`, a uniform per object). Dense wireframes fade in the shader, and are left out when too dense to see.
- **The mesh check** in the Attributes panel runs on another thread for heavy shapes.
- **Triangles:** drawing, picking and the selection tint cut polygons into triangles with one function, `mesh::triangulate` ([decision 0057](docs/decisions/0057-after-phase-4-testing.md)), so what is picked is what is drawn. Triangles and flat polygons without dents take a quick path; bent and dented ones are cut along lines inside them. The cut is made when a mesh's geometry or pick index is built, and kept while only points move.

`tools/heavy/heavy-check.sh` times all of this on a 3-million-polygon shape at the end of each phase.

Hidden polygons ([decision 0055](docs/decisions/0055-basics.md)) are a view setting kept by the workbench: the drawing cache gives each item a `shown` mesh without them, which is what the renderer draws and the pick index searches, while the whole `mesh` stays for the tools.

Selecting follows the same path ([decision 0029](docs/decisions/0029-selection.md)): the viewport only records what the mouse did (`selecting::Gesture`), and once the frame's drawing is known the workbench's `Selector` looks it up in the same `PickScene` and changes the document. Whole objects are selected through `Document::set_selection` (as the Objects panel does) and elements are kept per object in `scene::Document` and changed through `select_elements`; both record undo steps; the selection rules themselves (all, invert, grow, shrink, loops, rings) are pure functions in `mesh::selection`.

Moving, rotating and scaling ([decision 0030](docs/decisions/0030-transform-gizmos.md)) uses the same split. `tools::gizmo` does the maths (handles, hit testing, `Change`s); the viewport draws the gizmo and hands over `GizmoEvent`s; the workbench turns each frame's change into commands with `transforming::Session`. A session takes back its previous frame with `Document::abandon` and executes the whole change again under one merge key, so a drag is one undo step and one recorded operation, and cancelling is one more `abandon`. Commands the user can pick are listed once in `ui::commands` (name, icon, key, explanation, when it arrives); the tool menu (`E`), the menus, the palette and the keys read from that list, and the workbench carries any of them out through one `run_command` ([decision 0037](docs/decisions/0037-tool-menu-and-command-list.md)). Tools still never change the scene directly: objects move by `Command::SetTransforms`, and in the Object mode their pivots by `Command::MovePivots` ([decision 0068](docs/decisions/0068-pivot-editing-and-scene-units.md)), points by recorded `MovePoints` and `TransformPoints` operations, extrudes and insets by `ExtrudePolygons`, `InsetPolygons` and `ExtrudeEdges` ([decision 0038](docs/decisions/0038-extrude-and-inset.md)) and bent extrudes, spirals included, by `ExtrudeBent` ([decisions 0050](docs/decisions/0050-pipe-bends.md) and [0062](docs/decisions/0062-spirals.md)), bevels by `BevelEdges` and `BevelPoints` ([decision 0040](docs/decisions/0040-bevel.md); rounded corners and stopping at neighbours are `mesh::bevel`'s own work behind `BevelOptions::improved`, [decision 0141](docs/decisions/0141-improved-bevel.md), so old recorded bevels keep their shape), loop cuts by `LoopCut` ([decision 0047](docs/decisions/0047-loop-cut.md)), Connect, Bridge and Weld by `Connect`, `Bridge` and `Weld` ([decision 0051](docs/decisions/0051-connect-bridge-weld.md)), clean-up by `DeleteElements`, `Dissolve`, `FillHoles`, `FlipPolygons` and `FixFacing` ([decision 0052](docs/decisions/0052-clean-up.md)), hard and soft edges by `SetHardEdges` and `HardEdgesByAngle` ([decision 0053](docs/decisions/0053-hard-and-soft-edges.md)), materials by `SetMaterial` ([decision 0074](docs/decisions/0074-material-slots.md)), and slits by `Slit` ([decision 0049](docs/decisions/0049-slit.md), stored as a line in the object's own space, with no ids), whose drags work the same way (`workbench::modelling` and `workbench::loop_cut`, `viewport::tool_drag`; Loop cut and Slit take clicks too; Bridge and Weld take a drag only when it starts on a highlighted element, so clicks still select; `workbench::slit` keeps the line being drawn) and whose last one can be changed with `Command::ReplaceOperation`. Those also record `mesh::Anchors`, where their elements were, so when a base's new settings renumber its points the stack finds them again by position ([decision 0032](docs/decisions/0032-edits-find-their-points-again.md)). With symmetry on ([decision 0054](docs/decisions/0054-symmetry.md)), the target is mirrored when a change begins: `mesh::symmetry::mirror_plan` groups the points with their mirrors, and each group is recorded as its own move or transform in the same undo step. Every other modelling edit made with symmetry on is wrapped as `Operation::Mirrored` with the object's `mesh::symmetry::Mirror` (axes and middle, from `Workbench::mirror_for`) ([decision 0059](docs/decisions/0059-symmetry-for-every-edit.md)); `scene::mirroring` makes it on both sides when the shape is worked out, together or as mirrored copies.

Rendering sits behind the `ViewportRenderer` trait, so the panel runs in tests with no GPU. See [decision 0013](docs/decisions/0013-viewport-rendering.md).

## Axes

**Z is up**, right-handed: X and Y form the ground plane. Every crate that deals with positions (`mesh`, `scene`, `render`, `tools`) uses this; only `fileio` converts, to Y-up for glTF. See [decision 0012](docs/decisions/0012-z-up-axes.md).

## The scene

- The single source of truth for an open project: the object hierarchy, each object's base and its operation stack. See [decision 0006](docs/decisions/0006-base-plus-operation-stack.md).
- An object is a **base** (a parametric primitive, an imported mesh, or an editable mesh) plus a **stack of layers**, each holding recorded operations. Base parameters stay editable; the stack replays on top. Built so far ([decision 0021](docs/decisions/0021-object-stack-foundation.md)): the base is the object's kind (a primitive or a plain mesh), the stack a list of layers with one implicit *Edits* layer, and `Scene::evaluated` works out the final shape from a per-layer cache inside the scene, which also keeps each layer's shape before its last edit ([decision 0042](docs/decisions/0042-fast-edits.md)). Each recorded edit is an entry with an id and on/off and pinned switches; edits trust their ids while the structure below them is unchanged ([decision 0044](docs/decisions/0044-edits-that-hold.md)). The interface draws and frames evaluated shapes.
- Every mesh element has a stable ID that is never reused. Operations create IDs deterministically, so replaying an unchanged history gives identical IDs.
- The scene changes only through commands, which gives undo, redo and future scripting from one mechanism. Projects are saved and opened by `fileio` through a file model kept separate from the scene's types, so the scene can change without breaking old files ([decision 0024](docs/decisions/0024-saving-projects.md)); the interface asks the app to save or open, and the app shows the dialogs. While the app runs it also saves backup copies of the scene in the background: the workbench decides when (every interval while the scene changes, on its own clock separate from saving) and asks the app, which writes them on another thread into its own session folder, kept locked so the next start can tell a crash from a running app and offer the copies back ([decision 0036](docs/decisions/0036-autosave.md)). If the app panics, a panic hook (`app::emergency`) writes one more copy, the *emergency copy*, from a snapshot of the document the app keeps after each frame in which it changed (at most twice a second, behind a lock the hook only tries to take), so a crash loses at most a moment of work. Undo history is bounded by a memory budget, not a step count: 2 GB as standard, oldest steps forgotten first, the newest always kept ([decision 0022](docs/decisions/0022-undo-memory-budget.md)).
- **Built so far** ([decision 0018](docs/decisions/0018-scene-document-and-undo.md)): `scene::Document` holds the `Scene` (a tree of objects with stable ids that are never reused), the undo history and the selection. `Document::execute` carries out a `Command` as small reversible edits (insert, remove, relocate, rename, show/hide) and records them as one step; `undo` applies their opposites in reverse order. The workbench owns the document: the Objects panel, menus and keys hand it commands, and the viewport reads a summary: the shown objects' meshes, world placements and selection (rebuilt only when the scene or selection changes), and the boxes to frame. An object's shape comes from its kind: a primitive's settings build a `mesh::Mesh` ([decision 0020](docs/decisions/0020-primitives.md)); the app turns the meshes into one GPU shape when the drawing's version changes.
- Exports are prepared once (`fileio::export::prepare`: objects, polygons or triangles, normals as seen, UVs, in the file's units and up axis) and every format writer works from that ([decision 0071](docs/decisions/0071-obj-export-and-import.md)); polygons carry their material slot and objects their materials, joined by name across objects ([decision 0074](docs/decisions/0074-material-slots.md)). The workbench asks for an export or import with a `WorkbenchAction`; the app asks where (native dialogs), calls `fileio`, and reports back. A project's game project folder (`scene::ExportTarget`, only text in the scene) is exported into by the app after each successful save, through `fileio::engine::export_into`, which never creates folders and only replaces the app's own files ([decision 0076](docs/decisions/0076-engine-export-paths.md)).
- **Live booleans** ([decision 0135](docs/decisions/0135-live-booleans.md)). An object's `booleans` name other objects (its cutters) and what they do; they change only through `Command::AddBoolean` and `Command::SetBooleans`. `Scene::cut_shape` works the cut out with the `booleans` crate (layer 1, wrapping `manifold-rust`): the evaluated shape, each cutter's whole shape placed into its space, rebuilt into polygons with their values; it is cached until anything it is made from changes. `Scene::unmodified_shape` is the cut with the live mirror after it, and what the modifiers start from; edits and picking keep working on `Scene::evaluated`. A heavy cut is handed out as a `CutJob` (`Scene::cut_state`), which the interface's drawing cache runs on another thread and gives back with `Scene::store_cut`, drawing the last cut meanwhile. Cutters are drawn as outlines (`DrawItem::outline`, `render::MeshDraw::faces` off) and left out of exports. The document keeps the order objects were selected in (`Document::selection_order`): the first selected is the one kept.
- **Modifiers** ([decision 0142](docs/decisions/0142-modifier-stack-and-remesh.md)). An object's `modifiers` (`scene::Modifier`: a `ModifierKind` with its settings, on/off, Show in viewport) are worked out after its edits, booleans and live mirror (`Scene::unmodified_shape`), first to last, for a `Purpose` (the viewport, or an export and Bake into shape); they change only through `Command::AddModifier`, `ChangeModifier`, `RemoveModifier` and `MoveModifier`. The scene keeps every step's result per object and purpose and reuses unchanged steps; `Scene::stack_state` hands out a `StackJob` for the rest, which the drawing cache runs on another thread when heavy (`MODIFIERS_IN_BACKGROUND_FROM`) and gives back with `Scene::store_stack`, showing the last result meanwhile. `Scene::shown_shape` is the viewport's final shape, `Scene::exported_shape` what exports and Bake into shape use. The work itself is pure functions in `mesh` (Remesh: `mesh::remesh`). Decision 0142 has the recipe for adding a kind.
- **Curves** ([decision 0138](docs/decisions/0138-curves-sweep-lathe.md)). A curve object's kind (`ObjectKind::Curve`) holds its Bézier curve (`mesh::curve::Curve`, ready-made or points with stable ids) and its solid (a line only, a sweep or a lathe), changed only through `Command::EditCurve`, `SetCurveSolid` and `SetReadyCurve`. The solid is its base shape, built by `mesh::curve::sweep` and `lathe` (rotation-minimising frames) whenever it is evaluated, so it is drawn, cut, mirrored, exported and baked like any shape; a sweep taking another curve as its cross-section is evaluated with that curve, kept by the cache to tell when it changes. A curve takes no recorded edits. The interface draws every curve's line in the component overlay and, in the Points mode, gives a selected curve a mesh of loose points named by its parts, so the selection, picking and gizmo machinery work on it unchanged; Draw curve (`workbench::curve_drawing`) inserts one object when the curve is finished. Spline editing ([decision 0139](docs/decisions/0139-spline-editing.md)) adds `CurveEdit`s that keep the shape (insert by de Casteljau, delete stretching the neighbours' handles, extend, reverse, round corners, resample, close ends) and point handle kinds (`PointKind`, auto handles settled after every edit), plus `mesh::curve::join` and `split`, with the scene commands `JoinCurveEnds` and `SplitCurve` and `Join` on curves alone; the interface's curve commands live in `workbench::curve_commands`, the line double-click in the selector, carrying a curve on in `curve_drawing`.
- Transforms are stored relative to the parent; world transforms are always derived. An object's transform places its pivot; its `offset` places its shape relative to the pivot, so the shape's place in the world is `Scene::shape_matrix` (pivot × offset). Anything that works with a shape in the world uses that, not `world_matrix` ([decision 0068](docs/decisions/0068-pivot-editing-and-scene-units.md)).

## Files

| Format | Read | Write |
| --- | --- | --- |
| `.avtl` (projects; versioned, compressed, crash-safe; see decisions [0007](docs/decisions/0007-rhy-project-format.md), [0010](docs/decisions/0010-avtl-file-extension.md) and [0024](docs/decisions/0024-saving-projects.md)) | Yes | Yes |
| OBJ | Yes | Yes |
| glTF / GLB | Yes | Yes |
| FBX | No | Late, optional |
| Settings file | Yes | Yes |
| `app.ron` (window size and panel layout; written by eframe) | Yes | Yes |
| Backup copies (`autosave/session-*/copy-N.avtl`, ordinary projects, in the app's own data folder; `emergency.txt` names one saved as the app crashed) | Yes | Yes |

## Extension points

| To add | Plug into |
| --- | --- |
| A modelling tool | The `Tool` interface in `tools` |
| A mesh operation | A pure function in `mesh` |
| An edit | A `Command` in `scene` |
| A primitive | A primitive kind in `scene` |
| A modifier | A `ModifierKind` in `scene`, its work in `mesh` (the recipe in [decision 0142](docs/decisions/0142-modifier-stack-and-remesh.md)) |
| A panel | A panel registered with the dock in `ui` |
| A file format | `fileio` |
| A user's own tool or command (Phase 15) | A Lua script, through `script` |

## Security

- Files from strangers are hostile: every reader checks sizes before allocating and has a fuzz test. Project files are held to one limits rulebook (`fileio::limits`, [decision 0045](docs/decisions/0045-limits-for-files.md)): a cap on the unpacked body, a memory budget every list is paid from while it is read (`Bounded`), and counts of objects, edits and polygons. Shapes can't grow past `scene::MAX_SHAPE_POLYGONS` through their edits.
- No usernames or local paths are written into exported files. No telemetry.
- Dependencies are audited for licenses and known vulnerabilities on every change (`cargo deny`).

## Checks

Automatic checks, in `.github/workflows/`:

- **Every pull request and every change to main** (`ci.yml`): formatting, clippy, the tests on Windows, Linux and macOS, the API docs, the window and GPU tests on a virtual screen (`tools/smoke/window-smoke.sh`), licences and known vulnerabilities (`cargo deny`), release builds, and a changelog entry when code changes.
- **Every pull request: exported files checked by other programs** (the `gltf-check` job in `ci.yml`). `fileio`'s ignored test `files_for_outside_checkers` writes sample `.glb` files (every primitive side by side, each on its own, an edited shape with materials, and Godot's collision names) into the folder in `GLB_CHECK_OUT`. The Khronos glTF Validator (`tools/gltf-check/validate.cjs`, the npm package `gltf-validator`, pinned) fails the job on any error and prints warnings; then a pinned Godot (downloaded from Godot's releases and checked against its SHA-512) imports them headless into a fresh project (`tools/gltf-check/godot-import.sh`), failing on any import error, and opens each imported scene to check it holds meshes and that the collision names became collision shapes (`check-scenes.gd`). The files are kept as an artifact.
- **Every night, and by hand** (`heavy.yml`): the heavy-mesh check (`tools/heavy/heavy-check.sh`, [decision 0033](docs/decisions/0033-heavy-mesh-safeguard.md)) on main, its numbers on the run's summary page and kept as an artifact for 90 days. It still runs by hand at every phase gate.

To run the export checks locally (Node.js and a Godot 4 program needed):

```
GLB_CHECK_OUT=/tmp/glb cargo test -p fileio --lib files_for_outside_checkers -- --ignored
npm install --prefix /tmp/validator gltf-validator@2.0.0-dev.3.10
NODE_PATH=/tmp/validator/node_modules node tools/gltf-check/validate.cjs /tmp/glb
tools/gltf-check/godot-import.sh path/to/godot /tmp/glb
```

## Decisions

Why things are the way they are: see [docs/decisions](docs/decisions).
