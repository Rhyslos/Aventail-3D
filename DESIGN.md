# Aventail 3D — Design Document

> Part of the Rhylib family of tools, alongside Halberd. Short name in everyday use: **Aventail**. An aventail is the chainmail curtain on a helmet: a mesh of interlocking rings.
>
> **Renaming is designed to be cheap:** the name lives in one place in code (`crates/app/src/branding.rs`), and `RENAMING.md` lists every other place it appears.

A free, open-source 3D modelling and UV-unwrapping tool for **hard-surface game assets**, written in Rust.
It starts as a faithful recreation of how Cinema 4D *looks, works and feels*, with the best parts of 3ds Max added on top. Over time it evolves into its own thing.

---

## 1. Vision and scope

### The pipeline it serves
```
Aventail 3D (model + UV)  ──glTF/OBJ──►  Substance Painter (bake + texture)  ──►  Godot (glTF)
```

### In scope
- Polygon modelling focused on hard-surface work
- UV seams, unwrapping, a UV editor and packing
- Clean export for Substance Painter and Godot

### Planned much later
- Scripting in Lua, so people can add their own tools and automate work (Phase 9, §4.7).
- Rigging, skinning and animation for game assets (Phase 10). Modelling and UV work always take precedence unless something urgent comes up.

### Out of scope (for now)
- Rendering beyond a good-looking viewport
- Sculpting, texturing and materials beyond a preview/checker

### Guiding principles
*Aim:* game development first, but good enough for a hobbyist generalist. If one hobbyist picks it up and enjoys it, the project has won.

1. **Feel first.** The viewport and interaction must feel calm, clean and smooth, like C4D. A smaller tool that feels great beats a larger one that feels clunky.
2. **Tools are the gateway.** As in 3ds Max, a deep, reliable modelling toolset is the core value.
3. **Never lose work.** Undo is effectively unlimited. Base object properties stay editable no matter how much has been done since.
4. **Copy C4D first, diverge later.** The C4D layout is the starting target. Changes come from real use, not guesses.
5. **Free forever.** Every dependency must allow free use and redistribution (see §9).
6. **Robust with heavy meshes.** Game assets are small, but someone may import a 2–3 million polygon model and build on top of it. The app must stay usable then: slower is acceptable, freezing or crashing is not (§4.8).
7. **Explains itself.** Warnings, checks and tooltips say in plain words what is wrong, why it matters and how to fix it, ideally with a one-click fix. Most 3D programs assume you already know them; this one shouldn't.
8. **Drags follow the pull.** Wherever a drag changes something in the viewport, the direction you pull decides what happens, measured from the thing being changed: its facing, its axis, its middle. Pull a polygon the way it faces and it comes out; pull an inset towards the polygon's middle and it shrinks. A fixed screen direction (right for more) is only the fallback when there is no way to follow. One rule for every tool, so the app speaks one language ([decision 0039](docs/decisions/0039-drags-follow-the-pull.md)).

---

## 2. Feature wishlist (from the user)

| # | Feature | Origin | Priority | Phase |
|---|---------|--------|----------|-------|
| W1 | C4D look and feel: calm, clean viewport, smooth interaction | C4D | Core | 0–1, ongoing |
| W2 | C4D layout: Object Manager + Attribute Manager + viewports | C4D | Core | 0, 2 |
| W3 | Near-unlimited undo history | Max | Core | 2 |
| W4 | Base object properties always editable, however much the object has been edited | Max | Core (architecture now) | 2, 6 |
| W5 | **Edit Poly stack**: a layered history of Edit Poly modifiers on top of each other. *"By far the most important tool."* | Max | **Top** (architecture now, UI later) | 2 (foundation), 6 (UI) |
| W6 | Zoom/orbit around the point under the mouse cursor | Max | High | 1 |
| W7 | Quick Align | Max | Medium | 6 |
| W8 | Clean, powerful UV unwrapping like C4D, aiming to be **the best UV tools available** (§7) | C4D | Core | 8 |
| W9 | Max-like breadth of modelling tools | Max | Core | 4, 7 |
| W10 | Scripting, so people can code their own functionality | Blender, Max, C4D | Later | 9 |
| W11 | Stays usable with a heavy imported mesh (2–3 million polygons in one object): a safeguard, not the expected use (§4.8) | Own | High (foundation now) | 4, checked at every phase gate |
| W12 | Never lose work to a crash: autosave and recovery | Own | High | 4 |
| W13 | **Bake readiness**: a live check that explains, in plain words, why a bake would go wrong (hard edge without a UV seam, overlapping UVs, missing high/low pair, flipped normals), each with a one-click fix | Own | High (sets the app apart) | 5, 8 |
| W14 | **Edits that survive changes underneath**: every recorded edit finds its elements again when a base's settings or a lower layer change ([0032](docs/decisions/0032-edits-find-their-points-again.md)) | Own, from W4/W5 | High (sets the app apart) | 3 (moves), 4 (every tool), 6 (layers), 8 (UVs) |
| W15 | **Game bevel** for the mid-poly workflow: bevel, weighted normals and hard edges and seams set in one step | Own | Medium (sets the app apart) | 7 |
| W16 | **Trim sheets and modular kits**: UV strips laid onto rows of a shared texture; pieces on a grid with sensible pivots, Godot collision meshes, each piece exported as its own file | Own | Medium (sets the app apart) | 5, 7, 8 |
| W17 | **Explains itself**: plain-language warnings and tooltips, a manual written for someone rusty, help one key away | Own | Ongoing | all |

> **W4 and W5 are the same idea underneath:** an object is a *base* plus a *stack of operations* that can be re-evaluated. This is why the architecture is designed for it from day one (§4), even though the Edit Poly stack UI comes later. Retrofitting non-destructive editing into a destructive modeller is close to a rewrite, so it must not be postponed at the architecture level.

---

## 3. Tech stack

| Concern | Choice | License |
|---|---|---|
| Language | Rust (stable) | — |
| GPU / viewport | `wgpu` | MIT / Apache-2.0 |
| Windowing | `winit` (via `eframe` or directly) | MIT / Apache-2.0 |
| UI | `egui` + `egui_dock` | MIT / Apache-2.0, MIT |
| Math | `glam` | MIT / Apache-2.0 |
| Element IDs | `slotmap` (generational keys) | Zlib |
| Serialization | `serde` + `ron` (project files), optionally `bincode` for history | MIT / Apache-2.0 |
| glTF export | `gltf` / `gltf-json` | MIT / Apache-2.0 |
| UV auto-unwrap + packing | xatlas (vendored C++, own thin bindings) | MIT |
| Linear solver (LSCM / relax) | `faer` (sparse) | MIT |
| Tangents | MikkTSpace port | Zlib-style (verify crate) |
| Booleans (later) | manifold3d via `manifold-csg`, or `manifold-rust` (pure Rust) | Apache-2.0 / MIT |
| UI font | Inter | SIL OFL 1.1 |
| Icons | Own SVGs, or Lucide (ISC) / Tabler (MIT) as a base | ISC / MIT |

Notes:
- The existing `xatlas-rs` crate is MIT but unmaintained since 2019. Prefer vendoring current xatlas sources (MIT) with our own small FFI layer.
- Substance Painter imports glTF, OBJ and FBX, and Godot imports glTF natively, so **glTF + OBJ cover the whole pipeline**. FBX is optional and late.
- Verify every license when a dependency is added; `cargo-deny` enforces this in CI (§9).

### Workspace layout
```
Aventail-3D/
├─ crates/
│  ├─ mesh/        # layer 0: mesh data structure, topology ops, geometry algorithms
│  ├─ uv/          # layer 1: seams, unwrapping, relax, packing (wraps xatlas/faer)
│  ├─ scene/       # layer 1: objects, operation stacks, evaluation, commands, undo
│  ├─ config/      # layer 1: settings, hotkeys, defaults
│  ├─ fileio/      # layer 2: .avtl projects, OBJ, glTF (FBX later), settings file
│  ├─ script/      # layer 2 (Phase 9): Lua scripting through commands
│  ├─ render/      # layer 3: wgpu viewport renderer, picking
│  ├─ tools/       # layer 3: camera, selection, gizmos, modelling and UV tools
│  ├─ ui/          # layer 3: egui panels, menus, palettes, theme
│  └─ app/         # layer 4: wires it together, frame loop, branding.rs
├─ tools/layer-check/  # enforces the layer rules in tests
├─ docs/           # manual/, decisions/
├─ licenses/       # license texts for embedded fonts, icons
└─ DESIGN.md, ARCHITECTURE.md, CONTRIBUTING.md, CLAUDE.md, RENAMING.md, CHANGELOG.md
```
Layers and their rules are in ARCHITECTURE.md and [decision 0003](docs/decisions/0003-layered-crates.md); a crate may only depend on its own layer or below, so `mesh` and `scene` never depend on rendering or UI and can be tested headlessly.
Crate names are deliberately generic (no product name in them), so renaming the app never touches code structure.

---

## 4. Core architecture

### 4.1 Mesh data structure (`mesh`)
- **Polygonal (n-gon) mesh** with full adjacency: vertices, edges, faces and face-corners (loops), similar to Blender's BMesh or a half-edge structure.
- **Stable generational IDs** (`slotmap`) for every element. Deleted elements never reuse a live ID.
- Attributes:
  - Vertex: position
  - Edge: **hard/soft flag** (smoothing), **UV seam flag**, crease (future)
  - Face: material slot, smoothing group (optional, Max-style)
  - Corner: **UV(s)**, custom normal (for weighted normals)
- Normals are computed from hard edges and face weights. They are never the source of truth unless explicitly custom.
- Every topology operation is a pure function `(&Mesh, params, selection) -> Result<(Mesh or delta, IdMap)>` with unit tests.
- Adjacency (`Topology`) must stay fast on 2–3 million polygons and update in part after small edits (§4.8).

### 4.2 Objects and the operation stack (`scene`)

An object is:
```
Object
 ├─ transform, name, visibility, parent/children   (C4D hierarchy)
 ├─ Base: Primitive { kind, params } | ImportedMesh | EditableMesh
 └─ Stack: [Layer]          // empty by default
      Layer { name, enabled, ops: [Operation] }
```
- **Evaluation:** `base → layer 1 → layer 2 → … → final mesh`. Each layer's output is cached. Changing something in layer *k* re-evaluates only from *k* upward.
- **Phase 2–5 behaviour (C4D-like):** the user sees a normal object. Edits are silently recorded as `Operation`s in an implicit single layer. Primitive parameters (e.g. cube segments) remain editable in the Attribute Manager, and the recorded edits replay on top (**W4**).
- **Phase 6 behaviour (Max-like):** the Edit Poly stack UI exposes layers. Users can add a new Edit Poly layer, disable one, reorder, collapse, or go back down and edit an earlier layer (**W5**).
- **Collapse** ("Make Editable" in C4D, "Collapse to" in Max) bakes the stack into an `EditableMesh` base. This is always available as an escape hatch.

**The hard problem — topological naming.** Operations refer to elements (e.g. "extrude faces 12, 15"). If a lower layer or base parameter changes the topology, those references can break.
Strategy:
1. IDs are generated **deterministically** by each operation, so replaying an unchanged history yields identical IDs.
2. Each operation also records **anchors**: where its elements were, relative to the shape's box. When the IDs no longer sit there (new settings renumbered the points), it finds its elements again by position ([decision 0032](docs/decisions/0032-edits-find-their-points-again.md)). Built for moves, turns and scaling; **every Phase 4 tool records anchors the same way**, and Phase 6's layers use them when a lower layer changes (**W14**).
3. When an element can't be found at all, the operation applies to whatever still matches and the layer shows a **warning badge**. Nothing is silently destroyed, and collapse/undo still work.
4. 3ds Max stops at the compromise in step 3; step 2 is where this app aims to do better. It does not need to be perfect, it needs to be predictable.

### 4.3 Undo / history (**W3**)
- Every user action is a `Command` that can apply and revert itself (storing a compact delta, not a full scene copy).
  - *To change (§4.8):* a few commands still keep whole meshes (Make editable keeps the mesh it replaced). On a 2-million-polygon object that's about 115 MB per step; they should keep only what changed, or spill to disk.
- **No fixed step limit.** History is bounded by a **memory budget** (default e.g. 2 GB, configurable, or "unlimited"). Older entries can spill to disk.
- Selection changes are undoable but coalesced (a drag-select is one step).
- Optional: save undo history inside the project file, so undo survives restarts.
- Undo history (time) and the operation stack (structure) are separate concepts: undoing can remove an operation from a layer, but the stack is not the undo list.

### 4.4 Tools (`app`)
A tool implements a common trait:
- **activate / deactivate**
- **viewport interaction** (hover, click, drag, with HUD feedback)
- **parameters** shown live in the Attribute Manager (C4D style: change "Offset" after extruding and the result updates)
- **commit** → produces an `Operation` + `Command`

Tools never mutate the mesh directly; they produce operations, which keeps undo and the stack consistent.

### 4.5 Keeping the door open for rigging and animation
Nothing animation-related is built before Phase 10, but these rules avoid a rewrite later:
- **Generic attribute layers.** Mesh attributes are stored as named, typed layers, so per-vertex **skin weights** (bone index + weight) can be added later as just another layer.
- **Extensible object types.** The scene graph's object kinds are an enum/trait that can gain `Joint`/`Bone` and other node types without restructuring.
- **Clean transforms.** Each object has a local transform relative to its parent, and world transforms are always derived from it. No code may assume world-space transforms are stored.
- **No "static scene" assumption.** Evaluation takes a context parameter (initially empty), so a current time/frame can be added without changing every function signature.
- **glTF as the target.** glTF supports skins and animation, and Godot imports them, so the same export path extends to animated assets.

### 4.6 Rendering (`render`)
- wgpu, MSAA, and a stable 60+ fps target on ordinary hardware for game-asset-sized meshes (up to ~1M triangles). Heavier meshes follow the safeguard targets in §4.8.
- Draw passes: shaded (soft studio lighting, matcap optional) → wireframe overlay → selection highlight → gizmos/HUD.
- **GPU picking** (ID buffer) for vertices/edges/faces, and **depth readback** for cursor-point navigation.
- Only meshes that changed are re-uploaded. *To improve (§4.8):* within a mesh, only the part that changed, so moving a few points of a huge mesh doesn't re-upload all of it.

### 4.7 Scripting (Phase 9)
Let people code their own functionality: custom tools, batch jobs, generators, export helpers. Blender, 3ds Max and Cinema 4D all have this, and a free tool gains a lot when its users can extend it.

**Language: Lua** ([decision 0011](docs/decisions/0011-lua-scripting.md)), using the **Luau** dialect through the `mlua` crate (both MIT):
- Small, fast and made to be embedded; adds about 1 MB, nothing to install.
- Luau has a built-in sandbox and optional type hints, which help editors autocomplete.
- The project lead already knows Lua from Garry's Mod, and Halberd plans Lua plugins too, so one language covers the whole Rhylib family.

**What scripts can do:**
- **Add commands:** a script registers a named command with parameters. It appears in menus, the command search and hotkeys, and its parameters show in the Attributes panel like any built-in tool's.
- **Read the scene:** objects, hierarchy, meshes, selection, UVs.
- **Change the scene only through operations and commands**, exactly like built-in tools (§4.2–4.4). So every script edit can be undone, and later shows up in the Edit Layers stack like any other edit.
- **Batch work:** rename, clean up, export every object with a preset, check meshes before export.

**What keeps it safe** (scripts get shared, so they are treated as untrusted):
- **Sandbox:** no file, network or system access by default. A script declares what it needs ("read and write files in the project folder") and the user approves it once per script.
- **Time and memory limits:** a runaway script is stopped with a plain-language message, never freezing the app or losing work.
- **Errors never crash the app:** they show the script name, line and message.

**Doors to keep open now** (cheap rules for the phases before 9):
- **Every action is a named command** (already planned in §5.4). The command list becomes the scripting API, so nothing built-in needs a second, script-only version.
- **Operations and commands take plain data** (numbers, IDs, settings), never UI objects, so a script can call them the same way a tool does.
- **The scripting API gets a version number.** Scripts say which version they target, so future changes can keep old scripts working.

### 4.8 Performance and robustness (**W11**)

The app is built for game assets (thousands to a few hundred thousand polygons), but must not fall over when someone imports a high-poly model (2–3 million polygons in **one** object) and models on top of it. This is a safeguard, not the expected use ([decision 0033](docs/decisions/0033-heavy-mesh-safeguard.md)).

**Targets** on ordinary hardware:

| | Game-sized scene | One 2–3M polygon object (safeguard) |
|---|---|---|
| Orbit, pan, zoom | 60 fps | 60 fps |
| Hover highlight, click to select | instant | under 50 ms |
| Drag a few points or a polygon | 60 fps | no visible stutter (30 fps or better) |
| Whole-shape rebuilds (import, a settings change, Make editable) | instant | a few seconds at most, never a freeze without feedback |
| Memory | small | the mesh plus a small multiple, not a copy per undo step |

**Before** (measured 2026-10-12 on a 2-million-polygon grid): holding, copying and moving the mesh was fast, but working out its connections took **3 s** and building the picking map **1.9 s**, both rebuilt whenever the shape changed. The first heavy-mesh check found selecting, hovering and dragging far too slow, and ran out of memory at 3 million polygons.

**Built** (the *Heavy meshes* milestone, [decision 0035](docs/decisions/0035-heavy-meshes.md)):
1. **Connections in flat lists**, built in one pass and kept with the faces, so moving points never rebuilds them.
2. **A spatial index** (a BVH: boxes inside boxes) per mesh for picking and snapping, built in the background for heavy shapes and updated only around moved points.
3. **Update only what changed**: meshes share their parts (points in chunks, faces in one flat list) between copies, so an edit copies only what it touches; the GPU keeps each object's mesh and uploads only the vertices around moved points; selecting or moving objects re-uploads nothing.
4. **Small undo steps**: steps share meshes with the scene rather than copying them; the memory budget still counts a shared mesh in full (to refine later).
5. **Long jobs in the background** with a note on screen: indexing a new heavy shape, its drawing geometry, and the mesh check. Import (Phase 5) will need the same for loading.

Also: dense wireframes fade out from afar (and aren't drawn when too dense to see), and **Help → Add a heavy test shape** adds a 3-million-polygon shape to try. Results on a 2-core test machine, 3 million polygons: every step under its target; selecting about 2 ms, hovering under 7 ms, a drag frame under 1 ms, 1–2 s of background work for a new shape. Drawing speed depends on the graphics card and is checked by trying the test shape.

**How it is checked**:
- Ordinary pull requests run the normal test suite, plus small benchmarks for code on a performance path (CONTRIBUTING). They do **not** run the heavy test.
- The **heavy-mesh check** (`tools/heavy/heavy-check.sh`) loads a 3-million-polygon object and measures the targets above: the interface, the drawing geometry and project files. It runs **at the end of every phase**, as part of the gate, and **whenever the project lead asks**. Its results go in the pull request.

---

## 5. UI specification — C4D first

### 5.1 Layout (default, dockable via `egui_dock`)
```
┌──────────────────────────────────────────────────────────────────┐
│ Menu bar                                                         │
│ Top command palette (primitives, tools)                          │
├──┬───────────────────────────────────────┬──┬────────────────────┤
│M │                                       │M │ Attribute Manager  │
│o │            Viewport(s)                │o │ (selected object / │
│d │   single perspective or 4-up          │d │  active tool)      │
│e │                                       │i ├────────────────────┤
│s │                                       │f │ Object Manager     │
│  │                                       │s │ (hierarchy tree,   │
│  │                                       │  │  later Edit Layers)│
├──┴───────────────────────────────────────┴──┴────────────────────┤
│ Status bar: hints, selection counts, snapping state              │
└──────────────────────────────────────────────────────────────────┘
```
- **Right column order:** Attribute Manager on top, Object Manager below. This deliberately differs from C4D's default (user preference, 2026-10-10).
- **Left palette:** mode switches (Model / Object / Points / Edges / Polygons / UV) and snapping, like C4D, with viewport display switches at the bottom (grid detail: fine 10 cm lines on or off).
- **Modifiers palette** (the project lead's choice, 2026-10-12): a thin column of icon buttons between the viewport and the right column, holding everything that changes a shape non-destructively: edit layers (Phase 6), mirror and array (Phase 7), booleans, subdivision, weighted normals and the game bevel. It sits right next to the Object Manager because that is where modifiers are applied and shown. C4D keeps its generators and deformers in a similar spot; we copy only the placement, with our own icons (drawn in code) and names. It appears with the first modifier (Edit Layers, Phase 6), not as an empty placeholder: a strip of buttons that do nothing would only confuse. How a click applies a modifier (to the selection, as a child or a parent, as a layer) is asked about before it is built ([0028](docs/decisions/0028-ask-about-controls.md)).
- **Object Manager:** tree with drag-to-reparent, rename, visibility toggles, and later the Edit Poly stack shown as a sub-list. Built: click, Ctrl and Shift selection with names in the accent colour, double-click to rename, drag onto a row to nest or onto its edge to reorder, an eye per row, fold arrows, a right-click menu, and Edit, View and Object menus ([0018](docs/decisions/0018-scene-document-and-undo.md)).
- **Attribute Manager:** tabbed properties for the selection or active tool.
- A **Coordinates panel** (C4D's position/scale/rotation fields) can be docked under the viewport.
- **Menu lines inside panels** ([decision 0014](docs/decisions/0014-studio-look-and-accent.md)): the viewport has its own (View, Cameras, Display, Options, Panel, and buttons to drag for pan, zoom and orbit) in place of a tab; Attributes and Objects have small ones too.
- **Overlays on the viewport:** view name top left, camera name top centre, axis marker top right, grid spacing bottom left, navigation controls picker bottom right.
- **Layout tabs** on the right of the menu bar: Standard now; Modelling and UV Edit arrive with their phases.
- The app never shows another program's name: commands, schemes and settings are named by what they do.

### 5.2 Visual style ("clean, pleasant, calm")
- Dark neutral greys with low contrast between panels; one accent colour for the active mode or tool and chosen options. **Red** is the standard; Blue, Marine green, Plum, Olive and Petrol blue can be chosen (Window → Accent colour). Selected objects (their names in the Object Manager and their outline in the viewport) use the accent colour, so selection follows the theme (2026-10-10). See [decision 0014](docs/decisions/0014-studio-look-and-accent.md).
- Viewport: a studio backdrop with a horizon (sky over a slightly darker ground), a finite grid that fades out around the point the camera looks at, the green Z axis rising from the origin, mid-grey shapes, thin anti-aliased wireframe, gentle highlight on hover.
- Typography: Inter, small but legible sizes; generous padding.
- Smooth motion: camera moves are immediate (no laggy smoothing by default), but UI transitions are short and soft.
- **We copy layout and behaviour, not assets.** No Maxon or Autodesk icons, names, logos or screenshots go into the repo.

### 5.3 Axes and navigation
**Z is up** ([decision 0012](docs/decisions/0012-z-up-axes.md)), as in 3ds Max and Blender, unlike Cinema 4D: X and Y lie flat on the ground grid, Z is height. Right-handed; X red, Y blue, Z green (the project lead's choice; most programs swap the last two). Rotation in the Coordinates panel keeps C4D's H, P and B, with heading turning around Z. Export converts to Y-up for glTF and Godot automatically.

| Action | Default ("Mouse only" scheme, Alt + mouse also on) |
|---|---|
| Orbit | RMB drag (or Alt + LMB) |
| Pan | MMB drag (or Alt + MMB) |
| Zoom/dolly | mouse wheel (or Alt + RMB) |
| Frame selected / all | `F` / `H` (§5.4). The view keeps its direction and glides there in 0.2 s; `F` frames everything while nothing is selected |
| Views | F1–F5: perspective, top, right, front, 4-up; a middle *click* (no drag) switches between one view and four. Flat views pan and zoom but never orbit |

**Navigation schemes** ([decision 0015](docs/decisions/0015-navigation-schemes.md)): picked from a button in the viewport's bottom-right corner. *Mouse only* (the standard: right orbits, middle pans), *Alt + mouse*, *Mouse only, middle zooms* (Shift + middle pans), *Middle pans, Alt orbits*, and *Middle orbits* (Shift + middle pans). Every scheme zooms with the wheel and leaves a plain left drag to the tools. Switches: *Alt + mouse works in every scheme* (on) and *Invert orbit direction* (off). The project lead chose *Mouse only* as the standard after trying them, because it is the easiest to start with; the choice of schemes stays, as easy customisation is part of making the app welcoming.

**Cursor-point pivot (W6)** — Max-style, on by default and toggleable (built: a one-pixel depth render under the mouse, read back without waiting; [decision 0017](docs/decisions/0017-picking-the-point-under-the-mouse.md)):
- On orbit start, read the depth under the cursor; if it hits geometry, orbit around that 3D point. If it hits nothing, fall back to the selection centre or the last pivot.
- Wheel zoom (and drag zoom) dollies towards the 3D point under the cursor, which stays under the cursor; over empty background, towards the ground under it, or along the line of sight.
- Show a small pivot marker while orbiting.
- **Switched off, it is exactly the original rotation:** orbit and zoom around the middle of the view (the project lead's explicit wish, 2026-10-10).

### 5.4 Commands and hotkeys
- **The tool keys come from Halberd** ([decision 0019](docs/decisions/0019-only-the-tool-keys-come-from-halberd.md), narrowing 0016): `Q` select, `W` move, `S` scale, `R` rotate, `T` all gizmos, `B` a new shape. Other keys are chosen by the project lead before each feature is built ([0028](docs/decisions/0028-ask-about-controls.md)); selection's are in [0029](docs/decisions/0029-selection.md). Others built earlier (common in 3D programs, no clashes with these): built so far `Ctrl+Z`/`Ctrl+Y` undo and redo, `Delete` delete, `Esc` deselect, `H`/`F` frame, `F1`–`F5` views, `Alt+G`/`Shift+G` group and ungroup; `Ctrl+S` save to come. Nothing else is taken from Halberd unless the project lead asks for it. Viewport navigation uses the schemes in §5.3.
- Every action is a named command with a configurable hotkey (stored in a settings file).
- **C4D-style modelling menu** (press a key, then a letter for the tool).
- **Max-style quad menu** on right-click as an optional "sprinkle" (Phase 6).
- Searchable **command palette** (Ctrl/Cmd+K or Shift+C, like C4D's Commander), reading from the command list ([0037](docs/decisions/0037-tool-menu-and-command-list.md)).
- **Tool menu** (`E`): the current mode's tools at the mouse, as a ring or a list ([0037](docs/decisions/0037-tool-menu-and-command-list.md)).

---

## 6. Modelling toolset (hard-surface focus)

**Selection:** click, rectangle, lasso, live selection brush (C4D), loop, ring, grow/shrink, select by hard edges/seams, invert, select connected; "visible only" toggle.

**Transform:** move/rotate/scale gizmos, axis/plane constraints, numeric input, grid/vertex/edge/face snapping, symmetry (live mirror).

**Tools — Phase 4 (core):**
- Extrude, Extrude Inner/Inset
- Bevel/Chamfer (edges and vertices; offset, segments, profile)
- Loop/Path Cut, Knife/Line Cut
- Connect (edges/vertices)
- Bridge, Weld/Merge (collapse, target weld)
- Dissolve, Delete, Fill hole, Flip normals
- Hard/soft edges, auto-smooth by angle

**Tools — Phase 7 (advanced hard-surface):**
- Booleans (union/subtract/intersect) via manifold
- Weighted normals
- Improved bevel (clamping, mitre patterns)
- **Game bevel** (**W15**): for the mid-poly workflow, where a model is bevelled and shaded well instead of baked. One step bevels the chosen edges, sets weighted normals, and marks the hard edges and seams that go with them.
- **Modular kit helpers** (**W16**): pieces sized to the grid, pivot presets (base centre, a corner, a snapping edge), and checking that pieces line up.
- Array, mirror and other non-destructive generators
- Subdivision preview with creases (optional for game assets)

---

## 7. UV toolset (W8, Phase 8)

The aim is the best UV tools available for game assets. Unwrapping itself is a mature field (RizomUV is the specialists' choice), so the difference comes from the workflow around it: UVs that don't break when you keep modelling, seams that place themselves well, and a UV editor that knows the bake. UVs come after the modelling phases, as one large phase, so the project lead learns modelling first ([decision 0034](docs/decisions/0034-uv-phase-after-modelling.md)).

**Foundations** (professional standard):
- **UV mode** in the left palette, with a dockable **UV Editor** panel (C4D style) and the UV Edit layout.
- Seams: mark/clear, painted with a brush in the viewport, and **"seams from hard edges"** (the most common hard-surface workflow).
- Unwrap: angle/conformal (LSCM via `faer`) using marked seams, **live** while seams change; automatic charting via xatlas.
- Relax, straighten (rectify to grid), rotate/flip/align islands, stitch/weld.
- **Texel density** display and normalization across islands/objects.
- Checker texture and seam/stretch visualisation in the viewport.

**What sets it apart:**
1. **UVs that survive modelling** (**W14**). UV work is recorded in the shape's history like any edit: seams follow their edges by their anchors ([0032](docs/decisions/0032-edits-find-their-points-again.md)), islands unwrap again after a change, and hand-made tweaks (rotating, straightening, placing an island) are kept and applied again. Model and UV in any order; a late bevel or loop cut no longer means redoing the UVs.
2. **Seams suggested for hard surfaces.** Proposed from hard edges, sharp angles and *hidden places* (undersides, backs, inside corners where a seam won't show). Each suggestion is accepted or rejected with a click, and the unwrap updates live while deciding.
3. **Bake-aware from the start** (**W13**). Texture size and target texel density (pixels per metre) are scene settings, with per-object overrides for hero assets. Every unwrap and pack respects them. Padding is in real pixels, with a warning when islands will bleed into each other in the texture's smaller mipmap levels (what a game shows at a distance). The promise: if the UV editor is happy, the bake is clean.
4. **UV editing on the model.** Move, rotate and scale a texture's placement by dragging the checker pattern directly on the surface. Hovering an island in 3D lights it up in 2D and the other way round, so many quick fixes never need the flat editor.
5. **Problems explained, with a fix** (principle 7). A stretch heatmap; clicking a red area suggests "cut a seam here" or "relax this", in plain words.
6. **Straight by default.** Hard-surface strips are straightened automatically, islands line up with the model's axes so panel lines and grain run the right way, and mirrored halves share UV space or not with one switch.
7. **Packing that rivals the specialists.** Rotation, nesting small islands inside the holes of large ones, keeping chosen islands together, one texture set per material, and **stacking identical parts** (bolts, panels) on the same UV space, with a warning about their bake offset. xatlas (MIT) is the starting point; matching the best packers means writing our own.

Also in this phase:
- **Bake readiness, UV part** (**W13**): hard edges that aren't seams, overlapping or flipped UVs, islands too close for the padding, uneven texel density; each explained, with a one-click fix where there is one.
- **Trim sheets** (**W16**): define the rows of a shared trim texture, then lay a strip of polygons straight onto a row (straightened, scaled to fit its height, moved along it).

---

## 8. Import / export

| Format | Direction | Phase | Purpose |
|---|---|---|---|
| Native project (`.avtl`) | in/out | 2 | Full scene, stacks, optional undo history |
| OBJ | in/out | 5 | Simple exchange, Substance Painter |
| glTF / GLB | out (in later) | 5 | Godot, Substance Painter |
| FBX | out | late, optional | Only if needed; own writer (Autodesk's SDK is not redistributable) |

**Bake readiness, model part** (**W13**, Phase 5): before exporting for Substance Painter, a check lists in plain words what would spoil the bake (flipped normals, n-gons that triangulate badly, non-manifold edges, faces with no area, a low-poly object without a high-poly partner or the other way round) with a one-click fix where there is one. High- and low-poly pairs are named the way Substance Painter matches them (`_high` / `_low`).

**Kit export** (**W16**, Phase 5): each selected piece to its own file in one go, and Godot collision meshes (a `-col` / `-colonly` suffix, generated from a simple shape or a chosen object).

Export options: triangulation (deterministic, so the bake matches the game), MikkTSpace tangents, scale and up-axis presets ("Godot", "Substance Painter"; the app is Z-up and glTF is Y-up, so glTF export converts automatically, see [decision 0012](docs/decisions/0012-z-up-axes.md)), export selected only.

### The `.avtl` project format
- **Header:** magic bytes `AVTL`, a format version number, and a flags field. The app can tell a `.avtl` file apart from anything else instantly, and refuses or upgrades old versions cleanly instead of crashing.
- **Body:** the scene, objects, operation stacks and settings, serialized with `serde` and compressed with zstd (BSD).
- **Optional sections:** undo history, and a small thumbnail image so file browsers and a future "recent files" screen can show previews.
- **Versioning:** every format change bumps the version, and the loader keeps upgrade steps for older versions, so old projects always open.
- **Safe saving:** write to a temporary file, then rename over the old one, so a crash mid-save never corrupts a project. Keep the previous save as a `.avtl.bak`.
- **File association:** the installer registers `.avtl` so double-clicking a project opens the app.
- **Extension:** `.avtl`, short for Aventail ([decision 0010](docs/decisions/0010-avtl-file-extension.md)). It was `.rhy` during planning; `.avt` was ruled out because CLO 3D and Marvelous Designer already use it for avatar files.

---

## 9. Licensing policy

- **Project license:** GPL-3.0-or-later. Anyone may use, modify, share and sell it, but distributed modified versions must stay open under the same license. Files people create with the app are theirs; the GPL covers the program, not its output.
- **Copyright stays with the owner:** code contributions from others are not accepted for now (ideas, bug reports and feedback are welcome). If that changes, contributors sign a Contributor License Agreement first, so future options (paid builds, dual licensing) stay open.
- **Allowed dependency licenses:** MIT, Apache-2.0, BSD-2/3, Zlib, ISC, Unicode-DFS, BSL-1.0; SIL OFL for fonts. All are GPL-3.0 compatible.
- **Dependencies stay permissive even though the app is GPL:** using GPL/LGPL/AGPL libraries would make dual licensing or other future commercial options impossible.
- **Not allowed:** GPL/LGPL/AGPL dependencies, anything proprietary or "free for non-commercial use only", the FBX SDK.
- **Enforcement:** `cargo-deny` runs in CI and fails on any disallowed license.
- **No proprietary assets:** nothing copied from Maxon or Autodesk software (icons, splash art, names). Recreating layout and behaviour is fine.

---

## 10. Roadmap

Each phase ends with something runnable. The user tests the *feel* at the end of each phase before the next starts. From Phase 4 on, every gate also includes the **heavy-mesh check** (§4.8).

### Phase 0 — Skeleton
- Repository set up in Halberd's style: license, README with progress tracker, security policy, contribution rules, decision records, templates
- Cargo workspace with the crates above and `layer-check`; CI (fmt, clippy, tests and builds on three platforms, docs, cargo-deny, changelog check)
- Window with egui + egui_dock: empty viewport, Attribute Manager (top right), Object Manager (below it), palettes, status bar
- Theme: colours, Inter font, spacing
- **Done when:** the app launches into the C4D-style layout with movable/dockable panels.

### Phase 1 — Viewport and navigation
- wgpu renderer inside the egui viewport panel; gradient background, grid, a test cube
- C4D navigation, with a choice of navigation schemes; cursor-point orbit/zoom (W6); frame selected/all; perspective/ortho views; 4-up layout
- **Done when:** navigating feels smooth and calm to the user.

### Phase 2 — Document, objects, history
- Scene graph and Object Manager (create, rename, reparent, hide, delete)
- Parametric primitives: cube, plane, cylinder, sphere, disc, tube, cone — parameters live in the Attribute Manager
- Object stack foundation (§4.2): base + implicit layer, cached evaluation
- Command system and memory-budgeted undo (W3)
- Save/load native project
- **Done when:** you can build a scene of primitives, edit their parameters, undo/redo freely, and save/reopen it.

### Phase 3 — Editable mesh and selection
- `mesh` data structure with tests
- Point/Edge/Polygon modes; all selection methods from §6; GPU picking
- Transform gizmos with constraints, snapping and numeric input
- Edits recorded as operations on top of the base (W4 works: change cube segments after moving points)
- **Done when:** you can select and transform components precisely, and base parameters remain editable.

### Phase 4 — Core modelling tools
- **Heavy meshes** first (**W11**, §4.8): flat connection lists, a spatial index for picking and snapping, partial updates, small undo steps, and the heavy-mesh check
- **Autosave and crash recovery** (**W12**): a copy saved in the background every few minutes, offered back after a crash; never overwrites the project itself. *Built* ([0036](docs/decisions/0036-autosave.md)): every 5 minutes after a change and the newest 5 kept (both in Settings → Autosave), in the app's own folder (Local AppData on Windows), "Recover unsaved work?" asked after a crash. Copies before risky steps come with the first tool that needs them
- **The tool menu** (`E`): the tools of the current mode at the mouse, as a ring (standard) or a list (Settings), every tool in a fixed place, from one **command list** that every menu, the palette and the keys read from. *Built* ([0037](docs/decisions/0037-tool-menu-and-command-list.md)). Keys for tools to come: `D` extrude, `I` inset, `V` bevel
- Tools from §6 Phase 4, each with live parameters and HUD, each recording anchors so it survives base changes (**W14**). *Extrude and inset built* ([0038](docs/decisions/0038-extrude-and-inset.md)): `D` and `I`, dragged anywhere in the view; polygons together or each on its own (Keep together); border edges extrude too; the last one changeable in the Attributes panel
- Hard edges and normals display
- **Done when:** you can model a simple hard-surface prop (crate, sci-fi panel) start to finish.

### Phase 5 — Export pipeline
- OBJ and glTF export with triangulation, tangents and presets
- Round-trip test: model → Godot and Substance Painter, shape, hard edges, normals and scale correct (the full bake round trip is the UV phase's gate)
- Bake readiness, model part, and kit export (§8; **W13**, **W16**)
- **Engine export paths** (to plan): presets for Godot, Unity and Unreal Engine that save straight into the engine's project folder and export again on every save. Godot and Unreal re-import by themselves. Unity reads glTF through its glTFast package (our own FBX writer would be extra work, and the Autodesk FBX SDK is ruled out). A live-link plugin per engine may come later
- A **splash screen** while the app starts (the app's name and version, and what it is loading), at the project lead's request
- **Done when:** a prop exports into Godot and Substance Painter and looks right (shape, hard edges, scale) with no fixes in other software.

### Phase 6 — 3ds Max sprinkles
- **The modifiers palette** (§5.1) between the viewport and the right column, starting with Edit Layers
- **Edit Poly stack UI (W5):** add/disable/reorder/collapse layers; edit lower layers and see upper ones re-evaluate, each layer finding its elements again by their anchors (**W14**); warning badges only for what truly can't be found
- Quick Align and align tools (W7)
- Optional quad menu
- **Done when:** the user's Max workflow of stacking Edit Polys feels right.

### Phase 7 — Advanced hard-surface
- Booleans, weighted normals, better bevel, generators (array, mirror)
- Game bevel and modular kit helpers (§6; **W15**, **W16**)
- Generators, booleans, weighted normals and the game bevel are added to the modifiers palette (§5.1)

### Phase 8 — UVs
Everything in §7, as one phase after the modelling phases ([decision 0034](docs/decisions/0034-uv-phase-after-modelling.md)):
- UV mode, the UV Editor panel and the UV Edit layout (and a Modelling layout alongside it), switched from the layout tabs on the menu bar
- Seams: marking, painting, from hard edges, and suggested seams
- Live unwrap, relax and straighten; automatic straightening, axis alignment and mirrored halves
- Texture settings: texture size and texel density for the scene, density display and normalising
- Packing: pixel padding aware of mipmaps, rotation, nesting, groups, texture sets per material, stacking identical parts
- Checker texture, and a stretch heatmap with explained fixes
- UV editing on the model, with 3D and 2D highlighting each other
- UVs that survive modelling
- Bake readiness for UVs, and trim sheets
- Round-trip test: model → Substance Painter bake → Godot looks correct
- **Done when:** a prop is unwrapped and packed in Aventail 3D with clean bakes in Substance Painter, and changing the model afterwards keeps its UVs.

### Phase 9 — Scripting
Lua scripting so people can add their own tools and automate work. Full design in §4.7.
- Script runner with the sandbox, time limit and permissions
- The scripting API: scene, selection, operations and commands (undoable like any edit)
- Scripts as commands: in menus, the command search and hotkeys, with parameters in the Attributes panel
- Script manager (install, enable, disable) and a reload button for script authors
- API reference and a "your first script" page in the manual; example scripts
- **Done when:** a user can write a short script (say, "bevel every hard edge of the selection by 2 mm") that shows up as a command, can be undone, and can't harm files outside what it was allowed.

### Phase 10 — Rigging, skinning and animation (far future)
Starts only once modelling and UVs are mature. Rough contents, to be designed properly when the time comes:
- **Aim (project lead, 2026-10-13):** every animation tool is for game developers, helping them animate for games in both first and third person: first-person arms and weapons, third-person characters, and loops and clips that export cleanly to engines.
- Joints/bones in the Object Manager hierarchy; bone display and editing
- Skinning: bind mesh to skeleton, weight painting, weight tools (normalize, mirror, smooth)
- Animation: keyframes on transforms, timeline and dope sheet, curve editor, playback in the viewport
- Export skinned and animated meshes to glTF for Godot

### To be designed: Texturing and a material builder (after Phase 8)
At the project lead's request (2026-10-13): a built-in texture painter, so no Substance Painter subscription is needed, and a material builder after it. Both still need a planning talk before they get phases and milestones. Rough parts:
- baking: normal, AO, curvature, thickness, position, ID;
- a layer stack with fill and paint layers and masks;
- smart masks driven by the baked maps;
- 3D painting;
- export to each engine's texture layout;
- later, a node-based material builder.

**Game-first, not a Painter copy.** The tools should be shaped by game work (texel density, trim sheets, texture sets, engine channel packing, mid-poly shading) and may well differ from Painter's. They still need to be good enough for a hobbyist generalist. Scripting and rigging move back to make room.

**Tied to the UV tools.** The project lead works in the 2D (UV) and 3D views side by side, and texturing should work the same way: paint in either view and see it in both. The UV phase's work should carry straight into texturing: texel density, trim sheets, texture sets and islands that survive modelling.

**Planning approach.** The project lead is new to texturing. Before this is planned, research what game texture artists actually use most (and what they skip) in Substance Painter and its free alternatives. Then bring a short, explained proposal to the planning talk. The manual should teach texturing from the basics, as it does modelling.

**Two styles on one core.** The project lead wants specialised tools for both hand-painted, stylised art (World of Warcraft, League of Legends, The Clone Wars) and realism. They share one core: layers, masks, baking, painting in both views and export. On top of that come:
- **Hand-painted kits:** an unlit/flat-colour view, brushes with pressure and smooth blending, colour picking from the model, gradient fills by height or direction, baked AO and curvature used as painting guides rather than final maps, and painting across UV seams without visible joins.
- **Realism kits:** PBR material layers (metal, roughness, normal, height), smart masks for wear, dirt and rust, and lighting previews.

Each style can have its own starting layout and presets.

**Hand-painted first.** The project lead's priority: stylised, hand-painted work is the main focus and gets built first. Realism follows and stays the lesser focus. The aim is to be exceptional for creative, artistic assets and good enough for realism.

**Outlines for cartoon looks.** One switch gives an object an outline:
- **In the viewport:** a live outline with a chosen width and colour.
- **On export,** two ways that can be combined:
  - an *inverted-hull* outline as real geometry (a slightly larger, inside-out copy that renders black), which works in every engine with no shader;
  - a ready-made outline shader file for each engine (a Godot shader first; Unity and Unreal after), set up by the engine export paths.
- Settings can vary the width per area (thinner on small details) and use vertex colours or a painted mask.

**Texturing choices from the project lead (2026-10-13):**

| Idea | Decision |
|---|---|
| Painting in the 3D and 2D (UV) views at once, one stroke showing in both | **Yes** |
| Colour palettes saved with the project, and picking swatches from the model | **Yes** |
| Light-and-shadow helper: a sun direction pre-paints soft light and shade as a starting layer | **Yes**, with a switch to turn it off for those who don't want lighting while texturing |
| Edge highlight from the curvature map | **Yes** |
| A small, good stylised brush set, with smudge and blend | **Yes** |
| A toon (cel-shaded) viewport preview, with the outlines | **Yes, almost a priority** |
| Material and style presets saved as files to reuse and share | **Yes** |
| Live preview in an open engine while painting, through the engine export paths | **Yes**: what the engine integration is for |
| Texture budget | **Yes, extended:** warnings while working, and on export a detailed report about each file (size, format, channels, memory) |
| Pen tablets | **Both mouse and pen** must feel good: pressure and tilt with a pen, sensible defaults with a mouse |
| Trim sheets | **Their own pipeline:** making and using trim sheets in modelling, UVs and texturing. One texture per prop stays the default |
| Where textures are saved | **Both:** inside the project and as image files beside it, to open in other apps |
| Low-resolution and pixel-art textures | **Later.** No full pixel-art pipeline: Aseprite does that well |
| Vertex colours | **Later**, after the texturing tools are robust |
| Reference images in the paint view | **Ideas backlog** |

### Later / maybe
- Make the UI our own (driven by real use)
- FBX export, LOD helpers, baking preview

---

## 11. Documentation

The user is relearning 3D modelling while building this, so the manual is a first-class deliverable, not an afterthought.

- **Where:** `docs/manual/` as plain Markdown, readable directly on GitHub. Table of contents in `docs/manual/SUMMARY.md`.
- **What it covers:** only features that exist in `main`. Planned features live in this design doc, not the manual.
- **Page shape** (one page per tool, panel or workflow): *What it does* → *When to use it* → *How to use it* (steps) → *Settings* (each one explained) → *Shortcuts* → *Tips and common problems*.
- **Concepts page** (`docs/manual/concepts.md`): plain-language explanations of 3D and game-asset ideas (normals, hard edges, seams, texel density, n-gons…). Every page links to it rather than re-explaining.
- **Kept current:** any PR that adds or changes user-facing behaviour updates the manual in the same PR. A PR without the docs update is not finished.
- **Changelog:** `CHANGELOG.md` (repository root, Keep a Changelog format) gets one plain-language line per user-facing change, so the user can see what's new since they last opened the app. CI fails a pull request that changes code without a changelog entry.
- **The app explains itself** (**W17**, principle 7): every warning and check in the app says what is wrong, why it matters and how to fix it, in the manual's plain language, with a hover explanation and a one-click fix where possible.
- **Later:** in-app help (a key on a tool opens its manual page, tooltips link to it).

## 12. Ideas backlog

Possible features noted along the way. Not scheduled; nothing here blocks other work.

### Backlog (status: to discuss)
| Item | Why it matters for game assets | Depends on / relates to |
|---|---|---|
| Material slots / polygon selection sets | Each slot becomes a texture set in Substance Painter and a material in Godot | Face attributes (§4.1), export |
| Pivot / axis editing | The object's origin is where Godot places, rotates and snaps the asset; Quick Align needs it too | Object mode, W7 |
| Scene units and export scale | Godot works in metres; a wrong scale means re-scaling every import | Export presets (§8) |
| Reference images in viewports | Blueprints/concept art to model hard-surface parts against | Renderer, ortho views |
| LODs | Lower-detail versions for distance; Godot can also generate them automatically | Export, possibly a decimate tool |
| Measure tool | Checking real-world sizes (door frames, crates) while modelling | Snapping |

## 13. Decision log
Decisions from the planning phase, one line each. **From 2026-10-10 on, major decisions get a full record in [docs/decisions](docs/decisions)**; add a one-line pointer here too, so this log stays a quick overview.

- 2026-10-09 — Rust + wgpu + egui chosen; C4D look and feel as starting target; hard-surface + UV for game assets as scope.
- 2026-10-09 — Object = base + operation stack from day one, to make W4/W5 possible without a rewrite.
- 2026-10-09 — glTF + OBJ first; FBX deferred (Substance Painter and Godot both accept glTF).
- 2026-10-09 — Permissive-only dependencies, enforced by cargo-deny.
- 2026-10-09 — Rigging, skinning and animation added as Phase 9. Modelling and UVs keep priority; architecture rules in §4.5 keep the door open.
- 2026-10-10 — Mockups approved. Right column swapped: Attribute Manager on top, Object Manager below.
- 2026-10-10 — User manual in `docs/manual/` is updated in the same PR as every user-facing change (§11).
- 2026-10-10 — Native project files use the `.rhy` extension (§8). *Replaced the same day: see the `.avtl` line below.*
- 2026-10-10 — Name chosen: **Aventail 3D** (replaces working title "Facet"). Crates renamed to generic names; product name kept in `branding.rs` + `RENAMING.md` so a future rename is quick.
- 2026-10-10 — Project license: GPL-3.0-or-later, so it stays free. No outside code contributions for now (or a CLA if that changes), keeping commercial options open. Repo: github.com/Rhyslos/Aventail-3D (public).
- 2026-10-10 — Rule to flag dependencies and companion features before building was tried and dropped, as it would slow work down. §12 kept as an unscheduled ideas list.
- 2026-10-10 — Same workflow as Halberd: one milestone per branch and pull request, never push to `main`, two sign-offs, CI on three platforms, decision records ([0001](docs/decisions/0001-record-decisions.md)). Crates split into 9 layered crates ([0003](docs/decisions/0003-layered-crates.md)).
- 2026-10-10 — Window built with eframe and egui_dock, palette icons drawn in code, unbuilt commands shown greyed out with their phase ([0008](docs/decisions/0008-window-and-panels.md)).
- 2026-10-10 — Calm dark theme from the mockups, always dark, Inter font, readability tested ([0009](docs/decisions/0009-theme.md)). Phase 0 milestones complete.
- 2026-10-10 — Project files use `.avtl` instead of `.rhy`, to match the app's name; `.avt` is taken by CLO 3D and Marvelous Designer ([0010](docs/decisions/0010-avtl-file-extension.md)).
- 2026-10-10 — Scripting added as Phase 9 (rigging moves to Phase 10). Lua chosen as the language, through mlua with the Luau dialect and its sandbox ([0011](docs/decisions/0011-lua-scripting.md)); confirmed by the project lead.
- 2026-10-10 — Z is up, X and Y form the ground plane, as the project lead prefers (and as in 3ds Max and Blender); glTF export converts to Y-up ([0012](docs/decisions/0012-z-up-axes.md)).
- 2026-10-10 — Phase 0 gate passed: the project lead tested the window, layout and theme. Feedback: Coordinates fits its rows and panels can be dragged smaller; icons and the Coordinates panel brought closer to the mockups (white-ish icons, blue shapes, coloured axis letters).
- 2026-10-10 — Viewport drawn on the GPU into an offscreen image shown in egui, as in Halberd; grid on the X–Y ground, studio shading, test cube until Phase 2 ([0013](docs/decisions/0013-viewport-rendering.md)).
- 2026-10-10 — Feedback on the first viewport: axis colours changed to X red, Y blue, Z green ([0012](docs/decisions/0012-z-up-axes.md)); the viewport made softer so it feels less like Blender: wrapped, gentler lighting with a slight gradient across faces, a little darkening near the ground, a quieter grid and fainter edges ([0013](docs/decisions/0013-viewport-rendering.md)).
- 2026-10-10 — C4D navigation built: Alt + left/middle/right drag orbits, pans and zooms around a camera pivot, the wheel zooms; the scene follows the mouse when orbiting, and the drag kind is fixed when the drag starts. Zoom is exponential (each step the same share of the distance), 2 cm to 1 km from the pivot. Cursor-point orbit and zoom (W6) stay the next milestone.
- 2026-10-10 — The project lead found the grid busy: a grid detail switch added to the left palette (and View menu), with the fine 10 cm lines **off by default** and the choice remembered between sessions.
- 2026-10-10 — The project lead found the app still felt like Blender. A mockup with their screenshots as reference was approved, then an accent colour picked from twelve versions: darker studio look, viewport with a horizon and a finite grid, menu lines inside panels, viewport overlays, layout tabs; **red** accent as standard with five others to choose from ([0014](docs/decisions/0014-studio-look-and-accent.md)). No other program's name appears in the app, its code or the manual.
- 2026-10-10 — Navigation schemes to try before choosing a standard, picked from the viewport's bottom-right corner, named by what they do ([0015](docs/decisions/0015-navigation-schemes.md)).
- 2026-10-10 — Keyboard and mouse controls copy Halberd's, so both Rhylib tools work the same; only viewport navigation keeps its own schemes while they're being tried ([0016](docs/decisions/0016-controls-follow-halberd.md)).
- 2026-10-10 — *Mouse only* becomes the standard navigation scheme (easiest to start with); the others stay as choices. The grid reaches three times farther when zoomed in (at least 18 m around the focus), as the closer fade felt claustrophobic.
- 2026-10-10 — Orbit and zoom around the point under the mouse (W6) built: the surface under the mouse is found with a one-pixel depth render read back a frame later, never stalling the interface ([0017](docs/decisions/0017-picking-the-point-under-the-mouse.md)). On by default, with a switch in the navigation controls.
- 2026-10-10 — Frame all (`H`) and frame selected (`F`) built: the view keeps its direction and glides 0.2 s until the scene's bounding box fits with a little room; `F` frames everything until selection exists. Keys chosen not to clash with Halberd. Confirmed: with orbit around the mouse switched off, the original rotation (around the middle of the view) is what's used.
- 2026-10-10 — Perspective, top, front and right views and the four-view layout built, completing Phase 1's milestones: `F1`–`F4` show one view on its own, `F5` all four (keys as planned, unused in Halberd), and a middle click switches between one and four. Top, front and right are flat (orthographic) views that pan and zoom but never orbit, each with its grid facing the camera. The last view clicked is the active one, marked with a thin accent line; the viewport's menus act on it. Each view keeps its own camera; the layout choice is remembered. In-app help, planned for `F1`, will need another key ([0016](docs/decisions/0016-controls-follow-halberd.md)).
- 2026-10-10 — **Phase 1 gate passed:** the project lead found the views and the four-view layout worked flawlessly and felt good.
- 2026-10-10 — Object Manager built, the first Phase 2 milestone: a scene tree changed only by undoable commands made of small reversible edits; basic undo and redo come with it so nothing can be lost, with the memory budget left to its own milestone ([0018](docs/decisions/0018-scene-document-and-undo.md)). Groups can be created (the only new kind until primitives); the test cube is an object. New objects go at the top of the list; selection is not an undo step yet. `Alt + G` groups and `Shift + G` ungroups.
- 2026-10-10 — The project lead clarified the Halberd rule: only the tool keys (`Q`, `W`, `S`, `R`, `T`, `B`) come from Halberd; other keys are chosen for this app, and nothing else is based on Halberd unless they name it. The fly camera, Keybinds viewer and other Halberd-only keys are dropped from the plan ([0019](docs/decisions/0019-only-the-tool-keys-come-from-halberd.md)).
- 2026-10-10 — Primitives built (cube, plane, cylinder, sphere, disc, tube, cone): they stand on the ground with the pivot at the base centre, round ends are single n-gons, round sides are soft with hard rims, settings are clamped, and a new scene starts empty. The mesh crate gets its first data structure (points, n-gon faces, hard edges, stable ids); faces are drawn from both sides and selected objects are outlined in the accent colour ([0020](docs/decisions/0020-primitives.md)).
- 2026-10-10 — At the project lead's request: selection (names and the viewport outline) uses the accent colour instead of orange, and the orbit marker shows only while orbiting around the point under the mouse, not around the middle of the view.
- 2026-10-10 — Primitive settings editable in the Attribute Manager (Basic: name, visible; Shape: the kind's settings). Edits are `SetPrimitive` commands; a drag, a typed value or a click is one undo step (`execute_merging` folds each frame of a drag into one step; a step that undo returned to never merges again). Copy and paste settings between shapes of the same kind. Settings of several selected objects at once are left for later.
- 2026-10-11 — At the project lead's request, added to the Attribute Manager milestone: every primitive is 1 m across as standard (2 m felt large); step arrows beside value fields (1 cm or 1 per click, Shift for ten); lengths shown in metres as standard with a switch to centimetres, saved between sessions (the scene keeps centimetres); round ends take cap or ring segments and a choice of one n-gon or a centre point; copy and paste settings in the Object Manager's right-click menu; three grid levels (1 m, 50 cm, 10 cm) replacing the fine-grid switch; and `B` opens a shape list by the mouse (`B` again moves down, `Shift + B` up, `Enter` or `Space` creates), `B` being Halberd's shape key ([0019](docs/decisions/0019-only-the-tool-keys-come-from-halberd.md), [0020](docs/decisions/0020-primitives.md)). The arrow keys, `Enter`, `Space` and `Esc` in the list were chosen as the usual list keys.
- 2026-10-11 — Object stack foundation built: an object's kind is its base (primitive settings, or a plain mesh) and every object has a stack of layers of recorded operations, edited through `Record` as small undoable edits; the scene evaluates and caches each layer's result, re-evaluating only from the first changed layer; references to missing points apply to what remains and warn; `MakeEditable` (key `C`, chosen as the usual key for it, per 0019) bakes base and edits into a mesh. The first operation, moving points, exists for the Phase 3 tools; no tool records one yet ([0021](docs/decisions/0021-object-stack-foundation.md)). The project lead merged PR #14 untested and will test it together with this milestone.
- 2026-10-11 — Undo bounded by memory, not steps: each step's size is estimated (meshes counted in full), and over the budget the oldest undo steps are forgotten first, then far redo steps; the newest step is always kept. The standard budget is 2 GB, with Edit → Undo memory offering 256 MB–8 GB or Unlimited (remembered). Spilling to disk and keeping history in the project file are left for after saving; selection undo is left for the Phase 3 selection tools ([0022](docs/decisions/0022-undo-memory-budget.md)). The project lead tested PR #14 and PR #15 together and both work as intended.
- 2026-10-11 — At the project lead's request, app-wide preferences are gathered in one **Settings** window (Edit → Settings…, `Ctrl + ,`, the usual place and key): Interface (accent, units), Viewport (navigation, grid), Undo (undo memory). Accent and undo memory left the Window and Edit menus; quick switches used while working stay where they are and change the same values ([0023](docs/decisions/0023-one-settings-window.md)). PR #16 (undo memory) merged.
- 2026-10-11 — Saving and opening `.avtl` projects built, the last Phase 2 milestone: a file model per format version kept separate from the app's types (with a committed version 1 sample that must always open), postcard encoding compressed with zstd, every value checked on reading, safe saving (temporary file read back, `.bak` backup, atomic rename), File → New/Open/Open recent/Save/Save As with the usual keys, a Save / Don't save / Cancel question before changes are lost, the project's name in the title, native dialogs through `rfd`, and a project path accepted on the command line. Undo history, camera views and a thumbnail are left for later format versions ([0024](docs/decisions/0024-saving-projects.md)). **Phase 2 is complete and awaits the project lead's gate check.**
- 2026-10-11 — The project lead merged PRs #17 and #18 untested and asked to start Phase 3, testing Phase 2's gate together with the next PR; the gate is recorded as **deferred, not passed**. First Phase 3 milestone, the mesh data structure: a topology index derived from the face list (adjacency, borders, non-manifold and flipped edges, loops, rings, pieces) and a mesh check shown in the Attributes panel; generic attribute layers are left for the phases that use them ([0025](docs/decisions/0025-mesh-topology.md)).
- 2026-10-11 — At the project lead's request: Make editable moves to `Ctrl + E` (plain letters stay free for tools); Quit loses its key (no `Ctrl + Q`, to avoid slips); the m/cm switch beside the Shape title is removed (the Units menu does it); chosen rows use the accent fill (the active mode button's red) instead of the soft tint; unsaved changes show as an accent badge in the menu bar and "● name (unsaved)" in the title. The Windows error sound on key presses is fixed by handing the window the keyboard when it is active without it, the first `unsafe` exception ([0026](docs/decisions/0026-windows-keyboard-focus.md)).
- 2026-10-11 — **Phase 2 gate passed**: the project lead tested PRs #17–#19 together ("it all felt great"). Second Phase 3 milestone, the Points, Edges and Polygons modes: keys `1`, `2`, `3` (common in 3D programs; `Q` stays the Select tool per [0019](docs/decisions/0019-only-the-tool-keys-come-from-halberd.md), and the same key again returns to Model). The selected objects are the ones being edited: their edges (and points) are drawn as an overlay, and the element under the mouse is found on the CPU, visible elements only, and lit in the accent colour with its name in the status bar. Clicking to select is the next milestone ([0027](docs/decisions/0027-component-modes.md)).
- 2026-10-11 — At the project lead's request, controls are now asked about before a feature is built, with a recommended choice for each ([0028](docs/decisions/0028-ask-about-controls.md), amending 0019). Third Phase 3 milestone, selection, with the project lead's controls: click (Ctrl + click flips), drag with Box, Lasso or Live selection (Shift adds, Ctrl removes; `Q` steps through them), double-click loop and Shift + double-click ring, Ctrl + wheel grow and shrink (brush size with the brush; Ctrl + wheel no longer zooms), Ctrl + A all, Ctrl + I invert, Esc unselects elements. Box, lasso and brush select through shapes as standard, with a **Select only visible** switch on the left palette. Element selections are kept per object in the document, apart for each kind, by stable IDs, and changing them is an undo step (a brush stroke is one); clicking shapes in the viewport selects them in the Model mode, outside undo like the Object Manager. Saving selections in project files is left for a later format version ([0029](docs/decisions/0029-selection.md)).
- 2026-10-11 — After testing the selection milestone, at the project lead's request: one key pattern for every way of selecting (plain replaces, **Ctrl adds, Shift removes**; Ctrl + click on something selected unselects it), with three other patterns in Settings (Shift adds and Ctrl removes; Ctrl flips; Ctrl adds and Alt removes); rings on Alt + double-click; every selection, objects included, is an undo step; double-clicks allow 0.5 s and must land in the same spot (two quick clicks on different shapes were taken as a double-click); points drawn as round dots ([0029](docs/decisions/0029-selection.md)).
- 2026-10-11 — Fourth Phase 3 milestone, move, rotate and scale, with the project lead's controls: `W`/`R`/`S`/`T` gizmos (arrows, plane squares and a free centre; axis rings and a line-of-sight ring; per-axis boxes and a uniform centre), `Q` back to selecting, Shift for ten times finer, Esc or a right click to cancel, `X`/`Y`/`Z` axis locks and `L` world/object axes, snapping on the palette button with Ctrl flipping it per drag (button-only in Settings) to points, edges, polygons or the grid shown, 15° and 10% steps. A drag is one undo step by taking back the previous frame (`Document::abandon`) and doing the whole change again. Objects get `Command::SetTransforms`; points record `MovePoints` or a new `TransformPoints` on top of the base. The Coordinates panel shows and edits world position, size and rotation ([0030](docs/decisions/0030-transform-gizmos.md)). Project files move to format version 2 to store `TransformPoints`; version 1 still opens ([0031](docs/decisions/0031-project-format-version-2.md)).
- 2026-10-11 — At the project lead's request after PR #22: the Scale gizmo gets plane squares like Move; the mouse's direction sets the ratio between the two axes (drag mostly along X and it scales mostly along X). Fifth and last Phase 3 milestone, edits recorded on top of the base: edits also record where their points, edges or polygons were (anchors, relative to the shape's box), and when a settings change renumbers the points they find them again by position (a polygon finds every polygon inside it, an edge every edge along it). The Attributes panel notes when that happened. Project format version 2 (unreleased) stores the anchors ([0032](docs/decisions/0032-edits-find-their-points-again.md)). **Phase 3 is complete and awaits the project lead's gate check**, together with PR #22.
- 2026-10-12 — After a planning talk with the project lead, added to the plan: heavy-mesh robustness as a safeguard (one imported 2–3 million polygon object must stay usable; targets and fixes in §4.8; the *Heavy meshes* milestone opens Phase 4; the heavy-mesh check runs at every phase gate and on request, not on every pull request) ([0033](docs/decisions/0033-heavy-mesh-safeguard.md)); autosave and crash recovery (Phase 4); and what should set the app apart: bake readiness (Phases 5–6), edits that survive changes underneath (every Phase 4 tool records anchors; Phase 7 layers use them), a game bevel for mid-poly work and modular kit helpers (Phase 8), trim sheets (Phase 6), kit export with Godot collision meshes (Phase 5), and an app that explains itself (principle 7). Wishlist W11–W17. Collision meshes and the mesh check before export moved from the backlog into Phase 5.
- 2026-10-12 — At the project lead's request, a **modifiers palette**: a thin column of buttons between the viewport and the right column, next to the Object Manager where modifiers are applied (C4D's placement; our own icons and names). It arrives with the first modifier (Edit Layers, Phase 7) rather than as an empty placeholder, and Phase 8's generators, booleans, weighted normals and game bevel join it. Generators leave the top palette's plan.
- 2026-10-12 — At the project lead's request, UVs become one large phase **after modelling**: Phase 6 is now the workflow tools (edit layers, modifiers palette), Phase 7 advanced hard-surface, Phase 8 UVs. The UV phase aims to be the best UV tools available: the basics at professional standard plus UVs that survive modelling, suggested seams in hidden places, bake-aware texture settings, UV editing on the model, explained fixes, straight-by-default islands and specialist-grade packing (§7). Export's gate no longer includes the bake round trip, which moves to the UV gate ([0034](docs/decisions/0034-uv-phase-after-modelling.md)).
- 2026-10-12 — **Phase 3 gate passed**: the project lead tested PRs #22–#24 ("all the branches are merged and tested"). First Phase 4 milestone, heavy meshes, with no new controls: meshes share their parts between copies (points in chunks, faces in one flat list), connections are flat lists kept with the faces, each mesh gets a picking index (BVH) built in the background and updated around moved points, the GPU keeps each object's mesh in its own space and uploads only what moved, the overlay holds only selected and hovered elements, dense wireframes fade from afar, and the mesh check runs in the background. Help → Add a heavy test shape adds a 3-million-polygon ball. The heavy-mesh check (`tools/heavy/heavy-check.sh`) meets every target on the test machine ([0035](docs/decisions/0035-heavy-meshes.md)).
- 2026-10-12 — Second Phase 4 milestone, **autosave and crash recovery**, with the project lead's choices: the interval changeable in Settings (5 minutes standard, 1–30, can be switched off; only when something changed), copies in the app's own folder (`%LOCALAPPDATA%` on Windows, Local rather than Roaming because copies can be large), the newest 5 kept (1–20 in Settings), and **ask first** after a crash (Restore, Open as saved, Discard). Each run keeps a locked session folder, so the next start tells a crash from a running app; a normal close deletes it. Backups run on their own clock, separate from saving (Ctrl + S doesn't restart the countdown; asked for after the first test). Restored work comes back unsaved under its project's name and is backed up again at once. Fixed along the way: opening or restoring a project mid-session could leave the viewport empty ([0036](docs/decisions/0036-autosave.md)).
- 2026-10-12 — After a talk with the project lead about keys (he had forgotten his old ones; C4D and Max both pair a few direct keys with a menu at the mouse and a search): **`E` opens the tool menu**, everywhere, at the mouse, with the tools for the current mode (object tools in Model and Object mode, move/rotate/scale included; modelling tools only while editing). Tap `E` to step (`Shift + E` back), `Enter`/`Space` to use, click, or hold `E` and flick. A **radial** ring as standard, a **list** in Settings for those who prefer C4D's M menus; more than eight tools allowed with clean icons; every tool keeps its place. Keys chosen for tools to come: `D` extrude (as in C4D), `I` inset, `V` bevel. Icons stay drawn in code in one file. A **command list** defines each command once (name, icon, key, explanation, when it arrives) for the tool menu, menus, palette and keys ([0037](docs/decisions/0037-tool-menu-and-command-list.md)). Built as its own milestone before extrude.
- 2026-10-12 — After trying the tool menu: a bigger middle (where a click closes it), a third look, **Segments** (separate tiles with gaps and no disc, after a picture the project lead shared), and a **Tap E to open** switch (on as standard, for accessibility; off, the menu shows only while E is held) ([0037](docs/decisions/0037-tool-menu-and-command-list.md)).
- 2026-10-13 — Tool menu and icons finished with the project lead in an icon mockup page (six rounds of picks): a new icon for every tool, one line width, the same icon everywhere; Transform (was move, rotate and scale) and Slit (was knife); every ring 14 places at one size, icon only, cancel sign in the middle; four ring looks (Continuous ring standard, Inner edge, Outlined, Solid tiles) plus List, Show keys around the ring and Tap E to open, in a Radial menu section of Settings → Interface ([0037](docs/decisions/0037-tool-menu-and-command-list.md)).
- 2026-10-13 — Fourth Phase 4 milestone, **extrude and inset**, with the project lead's controls: `D` and `I` make Extrude and Inset the tool, a left drag anywhere in a view sets the distance (after the first test: Extrude follows the way the polygons face on screen, right for Inset or when they face the viewer; a drag starting on an unselected polygon selects it first; Shift ten times finer; Esc or a right click cancels; a click still selects), exact values in the Attributes panel only, and a **Keep together** switch for neighbouring polygons (on as standard). Polygons keep their ids, so the selection holds; thickness stays even round corners; hard edges carry over; border edges extrude flat in the Edges mode, with the selection moving to the new edge. The last extrude or inset can be changed in the Attributes panel (`Command::ReplaceOperation`). Project files move to format version 3 ([0038](docs/decisions/0038-extrude-and-inset.md)). Added to Phase 5 at the project lead's request: a **splash screen** while the app starts.
- 2026-10-13 — At the project lead's request, a new guiding principle, **drags follow the pull** (§1, principle 8, [0039](docs/decisions/0039-drags-follow-the-pull.md)): a drag in the viewport is measured from what it changes (its facing, its axis, its middle), with a fixed screen direction only as a fallback. Extrude already followed the polygons' facing; Inset now follows the pull towards the polygons' middle. The gizmos already worked this way; the Scale gizmo's uniform centre keeps "right or up grows", as a drag that starts on the middle has no way to follow.
- 2026-10-13 — At the project lead's request, the README and CONTRIBUTING say plainly that the project isn't accepting outside contributions, as it is too early in development; bug reports and ideas stay welcome through Issues. The project stays open source under the GPL.
- 2026-10-13 — The official name is **Aventail 3D** (the "3D" says what it is). Added to the plan, still to be designed in detail: a built-in **texture painter** and later a **material builder**, after the UV phase, shaped by game work rather than copying Substance Painter, and good enough for hobbyist generalists; **engine export paths** for Godot, Unity and Unreal Engine in Phase 5. Scripting and rigging move later.
- 2026-10-13 — Gap review with the project lead. Decisions:
  - **Materials** come with texturing, unless the engine export needs something earlier (to check: a minimal material slot per polygon for export).
  - **High-poly models** for baking are sculpted elsewhere (Blender, ZBrush) and imported. **Sculpting** is left to Blender, or comes far later.
  - Our own **smoothing modifier** (subdivision, like 3ds Max's TurboSmooth) gets its own name, not copied from another program.
  - **Curves and splines** (sweep, lathe; pipes, cables, railings) are needed.
  - The **official release** comes when the app is finished, well after the material phase. Friends have agreed to test once there are enough tools to build something.
  - Wanted:
    - duplicate and **instances**;
    - splitting polygons off into an object, and joining objects;
    - hide and isolate;
    - **slide** points and edges along their neighbours, and drag them along existing surfaces;
    - pivot editing and scene units (needed by export);
    - shell (thickness).
  - **Pipe joins that keep their size:** turning a pipe's end ring keeps its cross-section from squashing, so pipes connect cleanly. A possible unique selling point, to be designed.
  - A **keyboard shortcut viewer** soon. Custom hotkeys are added along the way.
  - glTF (and OBJ) **import**. No FBX, as it can't be licensed.
  - **Screenshots and turntable GIFs** from the viewport.
  - Wishlist: interface size scaling, a first-start welcome with sample projects.
- 2026-10-13 — Follow-ups to the gap review:
  - **Pipe bends that keep their size**, a possible unique selling point. The project lead showed the problem in Blender: turning a cylinder's end ring pinches it, so a bend narrows.
    - **Keep round:** a switch on extrude, and when turning a ring of a round shape. The ring at the joint sits on the plane halfway between the two directions and stretches by 1 / cos(half the angle), so both pieces keep their full width (a mitre joint).
    - Attributes panel settings: **Angle**, **Bend radius** and **Segments**. With a radius, the bend is a smooth elbow of several rings, each kept round.
    - To design with the curves and splines work, which faces the same problem.
  - **Names to change for safety** (common functional words such as extrude, bevel, inset, bridge, weld and loop cut are fine):
    - Cinema 4D's "Object Manager", "Attribute Manager" and "Live selection" → "Objects panel", "Attributes panel" and "Brush selection";
    - 3ds Max's "Quick Align" → our own name;
    - "Edit Poly" was already renamed "Edit Layers".
    - To do as one small renaming pull request.
  - **Export stays in Phase 5,** kept simple: a minimal material slot (polygons assigned to a named material with a colour) so engines get their parts. Real materials come with texturing. Testers and the project lead's own Godot tests need a way out of the app.
  - The **keyboard shortcut viewer** becomes the last Phase 4 milestone.
- 2026-10-13 — Names decided:
  - Attribute Manager → **Attributes panel**
  - Object Manager → **Objects panel**
  - Live selection → **Paint select**
  - Quick Align → **Snap to**

  Added to the flip normals milestone:
  - **Select flipped polygons**, from the mesh check that already finds them;
  - **Fix facing**, which turns every polygon of a closed shape to face outward in one click;
  - **Select by facing**: polygons facing a direction (up, say) within an angle, or facing like the one clicked.
- 2026-10-13 — Where new tools go: basic modelling tools in Phase 4 (hide and isolate, slide, duplicate and instances, split and join); advanced tools and modifiers in Phase 7 (shell, curves and splines, pipe bends, the smoothing modifier); pivot editing and scene units with export in Phase 5. Testers join around Phase 7. Keys: **Bridge `C`** (free). Slit `X`, Loop cut `F` and `Z` for the camera were proposed but clash with keys in use (X/Y/Z axis locks, F frame selected); to be settled before those tools are built.
- 2026-10-13 — Keys settled for tools to come (per 0028):
  - **`X` Slit**, **`F` Loop cut**, **`C` Bridge**;
  - **`Z` frames the selection** (what `F` does now), and `H` still frames everything;
  - **axis locks move to `Shift + X / Y / Z`** (`Ctrl` would clash with undo, redo and cut).

  The switch happens in one go with the first of Slit or Loop cut, updating the manual, the tool menu and the keyboard viewer at the same time.
