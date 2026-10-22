# Aventail 3D — Design Document

> Short name in everyday use: **Aventail**. An aventail is the chainmail curtain on a helmet: a mesh of interlocking rings.
>
> **Renaming is designed to be cheap:** the name lives in one place in code (`crates/app/src/branding.rs`), and `RENAMING.md` lists every other place it appears.

A free, open-source 3D modelling and UV-unwrapping tool for **hard-surface game assets**, written in Rust.
It takes inspiration from how Cinema 4D and 3ds Max *feel to use*, and grows into its own tool.

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

### Planned later (in this order, §10)
- Advanced modelling and modifiers (Phase 10).
- Solis, the app's own renderer: real-time first (Phase 11), path tracing much later (Phase 20).
- Texture painting and a material node editor (Phases 12 and 13).
- An alpha release, then a long round of fixes and polish (Phases 14 and 15).
- Scripting in LustScript, with plain Lua too (Phase 16, §4.7).
- Rigging and skinning, then animation, for game assets (Phases 17 and 18).
- Gauntlet, effects: physics, particles and simulations, baked for game engines (Phase 19).
- Other languages, when wanted (English is built as a language package from Phase 9).

### Out of scope (for now)
- Sculpting

### Guiding principles
*Aim:* game development first, but good enough for a hobbyist generalist. If one hobbyist picks it up and enjoys it, the project has won.

1. **Feel first.** The viewport and interaction must feel calm, clean and smooth. A smaller tool that feels great beats a larger one that feels clunky.
2. **Tools are the gateway.** As in 3ds Max, a deep, reliable modelling toolset is the core value.
3. **Never lose work.** Undo is effectively unlimited. Base object properties stay editable no matter how much has been done since.
4. **Familiar first, our own way after.** Start from layouts modellers already know, and change them from real use, not guesses.
5. **Free forever.** Every dependency must allow free use and redistribution (see §9).
6. **Robust with heavy meshes.** Game assets are small, but someone may import a 2–3 million polygon model and build on top of it. The app must stay usable then: slower is acceptable, freezing or crashing is not (§4.8).
7. **Explains itself.** Warnings, checks and tooltips say in plain words what is wrong, why it matters and how to fix it, ideally with a one-click fix. Most 3D programs assume you already know them; this one shouldn't.
8. **Drags follow the pull.** Wherever a drag changes something in the viewport, the direction you pull decides what happens, measured from the thing being changed: its facing, its axis, its middle. Pull a polygon the way it faces and it comes out; pull an inset towards the polygon's middle and it shrinks. A fixed screen direction (right for more) is only the fallback when there is no way to follow. One rule for every tool, so the app speaks one language ([decision 0039](docs/decisions/0039-drags-follow-the-pull.md)).

---

## 2. Feature wishlist (from the user)

| # | Feature | Origin | Priority | Phase |
|---|---------|--------|----------|-------|
| W1 | C4D look and feel: calm, clean viewport, smooth interaction | C4D | Core | 0–1, ongoing |
| W2 | C4D layout: Objects panel + Attributes panel + viewports | C4D | Core | 0, 2 |
| W3 | Near-unlimited undo history | Max | Core | 2 |
| W4 | Base object properties always editable, however much the object has been edited | Max | Core (architecture now) | 2, 6 |
| W5 | **Edit Poly stack**: a layered history of Edit Poly modifiers on top of each other. *"By far the most important tool."* | Max | **Top** (architecture now, UI later) | 2 (foundation), 6 (UI) |
| W6 | Zoom/orbit around the point under the mouse cursor | Max | High | 1 |
| W7 | Snap to | Max | Medium | 6 |
| W8 | Clean, powerful UV unwrapping like C4D, aiming to be **the best UV tools available** (§7) | C4D | Core | 7 |
| W9 | Max-like breadth of modelling tools | Max | Core | 4, 7 |
| W10 | Scripting, so people can code their own functionality | Blender, Max, C4D | Later | 9 |
| W11 | Stays usable with a heavy imported mesh (2–3 million polygons in one object): a safeguard, not the expected use (§4.8) | Own | High (foundation now) | 4, checked at every phase gate |
| W12 | Never lose work to a crash: autosave and recovery | Own | High | 4 |
| W13 | **Bake readiness**: a live check that explains, in plain words, why a bake would go wrong (hard edge without a UV seam, overlapping UVs, missing high/low pair, flipped normals), each with a one-click fix | Own | High (sets the app apart) | 5, 8 |
| W14 | **Edits that survive changes underneath**: every recorded edit finds its elements again when a base's settings or a lower layer change ([0032](docs/decisions/0032-edits-find-their-points-again.md)) | Own, from W4/W5 | High (sets the app apart) | 3 (moves), 4 (every tool), 6 (layers), 8 (UVs) |
| W15 | **Game bevel** for the mid-poly workflow: bevel, weighted normals and hard edges and seams set in one step | Own | Medium (sets the app apart) | 8 |
| W16 | **Trim sheets and modular kits**: UV strips laid onto rows of a shared texture; pieces on a grid with sensible pivots, Godot collision meshes, each piece exported as its own file | Own | Medium (sets the app apart) | 5, 7, 8 |
| W17 | **Explains itself**: plain-language warnings and tooltips, a manual written for someone rusty, help one key away | Own | Ongoing | all |

> **W4 and W5 are the same idea underneath:** an object is a *base* plus a *stack of operations* that can be re-evaluated. This is why the architecture is designed for it from day one (§4), even though the Edit Layers interface comes later. Retrofitting non-destructive editing into a destructive modeller is close to a rewrite, so it must not be postponed at the architecture level.

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
│  ├─ script/      # layer 2 (Phase 16): scripting (LustScript and Lua) through commands
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
- **Phase 2–5 behaviour (C4D-like):** the user sees a normal object. Edits are silently recorded as `Operation`s in an implicit single layer. Primitive parameters (e.g. cube segments) remain editable in the Attributes panel, and the recorded edits replay on top (**W4**).
- **Phase 6 behaviour (Max-like):** the Edit Layers interface exposes layers. Users can add a new edit layer, disable one, reorder, collapse, or go back down and edit an earlier layer (**W5**).
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
- **parameters** shown live in the Attributes panel (C4D style: change "Offset" after extruding and the result updates)
- **commit** → produces an `Operation` + `Command`

Tools never mutate the mesh directly; they produce operations, which keeps undo and the stack consistent.

### 4.5 Keeping the door open for rigging and animation
Nothing animation-related is built before Phase 17, but these rules avoid a rewrite later:
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

### 4.7 Scripting (Phase 16)
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

## 5. UI specification

### 5.1 Layout (default, dockable via `egui_dock`)
```
┌──────────────────────────────────────────────────────────────────┐
│ Menu bar                                                         │
│ Top command palette (primitives, tools)                          │
├──┬───────────────────────────────────────┬──┬────────────────────┤
│M │                                       │M │ Attributes panel   │
│o │            Viewport(s)                │o │ (selected object / │
│d │   single perspective or 4-up          │d │  active tool)      │
│e │                                       │i ├────────────────────┤
│s │                                       │f │ Objects panel      │
│  │                                       │s │ (hierarchy tree,   │
│  │                                       │  │  later Edit Layers)│
├──┴───────────────────────────────────────┴──┴────────────────────┤
│ Status bar: hints, selection counts, snapping state              │
└──────────────────────────────────────────────────────────────────┘
```
- **Right column order:** Attributes panel on top, Objects panel below. This deliberately differs from C4D's default (user preference, 2026-10-10).
- **Left palette:** mode switches (Model / Object / Points / Edges / Polygons / UV) and snapping, like C4D, with viewport display switches at the bottom (grid detail: fine 10 cm lines on or off).
- **Modifiers palette** (the project lead's choice, 2026-10-12): a thin column of icon buttons between the viewport and the right column, holding everything that changes a shape non-destructively: edit layers (Phase 6), mirror and array (Phase 7), booleans, subdivision, weighted normals and the game bevel. It sits right next to the Objects panel because that is where modifiers are applied and shown. C4D keeps its generators and deformers in a similar spot; we copy only the placement, with our own icons (drawn in code) and names. It appears with the first modifier (Edit Layers, Phase 6), not as an empty placeholder: a strip of buttons that do nothing would only confuse. How a click applies a modifier (to the selection, as a child or a parent, as a layer) is asked about before it is built ([0028](docs/decisions/0028-ask-about-controls.md)).
- **Objects panel:** tree with drag-to-reparent, rename, visibility toggles, and later the Edit Layers stack shown as a sub-list. Built: click, Ctrl and Shift selection with names in the accent colour, double-click to rename, drag onto a row to nest or onto its edge to reorder, an eye per row, fold arrows, a right-click menu, and Edit, View and Object menus ([0018](docs/decisions/0018-scene-document-and-undo.md)).
- **Attributes panel:** tabbed properties for the selection or active tool.
- A **Coordinates panel** (C4D's position/scale/rotation fields) can be docked under the viewport.
- **Menu lines inside panels** ([decision 0014](docs/decisions/0014-studio-look-and-accent.md)): the viewport has its own (View, Cameras, Display, Options, Panel, and buttons to drag for pan, zoom and orbit) in place of a tab; Attributes and Objects have small ones too.
- **Overlays on the viewport:** view name top left, camera name top centre, axis marker top right, grid spacing bottom left, navigation controls picker bottom right.
- **Layout tabs** on the right of the menu bar: Standard now; Modelling and UV Edit arrive with their phases.
- The app never shows another program's name: commands, schemes and settings are named by what they do.

### 5.2 Visual style ("clean, pleasant, calm")
- Dark neutral greys with low contrast between panels; one accent colour for the active mode or tool and chosen options. **Red** is the standard; Blue, Marine green, Plum, Olive and Petrol blue can be chosen (Window → Accent colour). Selected objects (their names in the Objects panel and their outline in the viewport) use the accent colour, so selection follows the theme (2026-10-10). See [decision 0014](docs/decisions/0014-studio-look-and-accent.md).
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
| Frame selected / all | `Z` / `H` (§5.4; `Z` was `F` until Loop cut took it, [decision 0047](docs/decisions/0047-loop-cut.md)). The view keeps its direction and glides there in 0.2 s; `Z` frames everything while nothing is selected |
| Views | F1–F5: perspective, top, right, front, 4-up; a middle *click* (no drag) switches between one view and four. Flat views pan and zoom but never orbit |

**Navigation schemes** ([decision 0015](docs/decisions/0015-navigation-schemes.md)): picked from a button in the viewport's bottom-right corner. *Mouse only* (the standard: right orbits, middle pans), *Alt + mouse*, *Mouse only, middle zooms* (Shift + middle pans), *Middle pans, Alt orbits*, and *Middle orbits* (Shift + middle pans). Every scheme zooms with the wheel and leaves a plain left drag to the tools. Switches: *Alt + mouse works in every scheme* (on) and *Invert orbit direction* (off). The project lead chose *Mouse only* as the standard after trying them, because it is the easiest to start with; the choice of schemes stays, as easy customisation is part of making the app welcoming.

**Cursor-point pivot (W6)** — Max-style, on by default and toggleable (built: a one-pixel depth render under the mouse, read back without waiting; [decision 0017](docs/decisions/0017-picking-the-point-under-the-mouse.md)):
- On orbit start, read the depth under the cursor; if it hits geometry, orbit around that 3D point. If it hits nothing, fall back to the selection centre or the last pivot.
- Wheel zoom (and drag zoom) dollies towards the 3D point under the cursor, which stays under the cursor; over empty background, towards the ground under it, or along the line of sight.
- Show a small pivot marker while orbiting.
- **Switched off, it is exactly the original rotation:** orbit and zoom around the middle of the view (the project lead's explicit wish, 2026-10-10).

### 5.4 Commands and hotkeys
- **The tool keys come from Halberd** ([decision 0019](docs/decisions/0019-only-the-tool-keys-come-from-halberd.md), narrowing 0016): `Q` select, `W` move, `S` scale, `R` rotate, `T` all gizmos, `B` a new shape. Other keys are chosen by the project lead before each feature is built ([0028](docs/decisions/0028-ask-about-controls.md)); selection's are in [0029](docs/decisions/0029-selection.md). Others built earlier (common in 3D programs, no clashes with these): built so far `Ctrl+Z`/`Ctrl+Y` undo and redo, `Delete` delete, `Esc` deselect, `H`/`Z` frame (`Z` was `F`), `Shift+X`/`Y`/`Z` axis locks, `Ctrl` + wheel for the count of the tool in use (Bevel's segments, Loop cut's cuts, Extrude's bend segments; [0048](docs/decisions/0048-ctrl-wheel-changes-a-tools-count.md)), `F1`–`F5` views, `Alt+G`/`Shift+G` group and ungroup; `Ctrl+S` save to come. Nothing else is taken from Halberd unless the project lead asks for it. Viewport navigation uses the schemes in §5.3.
- Every action is a named command with a configurable hotkey (stored in a settings file).
- **A modelling menu** (press a key, then a letter for the tool).
- **Max-style quad menu** on right-click as an optional "sprinkle" (Phase 6).
- Searchable **command palette** (Ctrl/Cmd+K or Shift+C, like C4D's Commander), reading from the command list ([0037](docs/decisions/0037-tool-menu-and-command-list.md)).
- **Tool menu** (`E`): the current mode's tools at the mouse, as a ring or a list ([0037](docs/decisions/0037-tool-menu-and-command-list.md)).

---

## 6. Modelling toolset (hard-surface focus)

**Selection:** click, rectangle, lasso, paint select (a brush), loop, ring, grow/shrink, select by hard edges/seams, invert, select connected; "visible only" toggle.

**Transform:** move/rotate/scale gizmos, axis/plane constraints, numeric input, grid/vertex/edge/face snapping, symmetry (live mirror).

**Tools — Phase 4 (core):**
- Extrude, Inset (built)
- Bevel (edges and points; width, segments, profile; built)
- Loop cut (built), Slit (cutting along a drawn line; built)
- Extrude with a bend: pipe bends that keep their size (moved from Phase 7 at the project lead's request; built)
- Connect (edges and points) (built)
- Bridge, Weld (to the middle, or onto a target point) (built)
- Dissolve, Delete, Fill hole, Flip normals, with select flipped, fix normals and select same facing (built)
- Hard and soft edges, auto-smooth by angle (built: Hard edges by angle)
- Symmetry (built: a live mirror while moving, rotating and scaling points, and every modelling edit mirrored, [0059](docs/decisions/0059-symmetry-for-every-edit.md); the mirror through the pivot, the middle or the world centre)
- Basics: hide and isolate, slide points and edges, duplicate and instances, split polygons off and join objects (built)
- A keyboard shortcut viewer (built: Keys on the menu bar)
- Wireframe (built: Shaded, Shaded with wires, Wireframe), cavity (built, [0065](docs/decisions/0065-cavity.md)), clickable corner axes (built, [0066](docs/decisions/0066-clickable-axes.md)), automatic hard edges (built, [0064](docs/decisions/0064-automatic-hard-edges.md))

**Tools — Phase 10 (advanced hard-surface):**
- Booleans (union/subtract/intersect) via manifold
- Shell (thickness), and curves and splines (sweep, lathe)
- Our own smoothing modifier (subdivision, with creases)
- Weighted normals
- Improved bevel (mitre patterns, rounded corners; clamping, even width, skew and welding come earlier, with Groove, [0083](docs/decisions/0083-groove-and-bevel-control.md))
- **Game bevel** (**W15**): for the mid-poly workflow, where a model is bevelled and shaded well instead of baked. One step bevels the chosen edges, sets weighted normals, and marks the hard edges and seams that go with them.
- **Modular kit helpers** (**W16**): pieces sized to the grid, pivot presets (base centre, a corner, a snapping edge), and checking that pieces line up.
- Array, mirror and other non-destructive generators. The mirror generator brings a **live weld** for symmetry: the halves joined only while their points are within the weld distance, coming apart again when moved away, replacing Weld the middle now ([0059](docs/decisions/0059-symmetry-for-every-edit.md), the project lead's choice) *The mirror, with its live weld, came early as the live mirror ([0088](docs/decisions/0088-live-mirror.md)).*
- Subdivision preview with creases (optional for game assets)

---

## 7. UV toolset (W8, Phase 7)

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
- Window with egui + egui_dock: empty viewport, Attributes panel (top right), Objects panel (below it), palettes, status bar
- Theme: colours, Inter font, spacing
- **Done when:** the app launches into the main layout with movable/dockable panels.

### Phase 1 — Viewport and navigation
- wgpu renderer inside the egui viewport panel; gradient background, grid, a test cube
- C4D navigation, with a choice of navigation schemes; cursor-point orbit/zoom (W6); frame selected/all; perspective/ortho views; 4-up layout
- **Done when:** navigating feels smooth and calm to the user.

### Phase 2 — Document, objects, history
- Scene graph and Objects panel (create, rename, reparent, hide, delete)
- Parametric primitives: cube, plane, cylinder, sphere, disc, tube, cone — parameters live in the Attributes panel
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
- **Same keys everywhere, and E in every area** (the project lead's rule, 2026-10-16, [0095](docs/decisions/0095-uvs-before-advanced-modelling.md)): a key does the same kind of job in every part of the app (the viewport, the UV Editor, later areas), and E opens the tool menu with the tools for where the mouse is and what is being worked on.
- **The tool menu** (`E`): the tools of the current mode at the mouse, as a ring (standard) or a list (Settings), every tool in a fixed place, from one **command list** that every menu, the palette and the keys read from. *Built* ([0037](docs/decisions/0037-tool-menu-and-command-list.md)). Keys for tools to come: `D` extrude, `I` inset, `V` bevel
- **Foundations** (between bevel and loop cut, from the whole-app review): saves that always reopen ([0041](docs/decisions/0041-saves-that-always-reopen.md)), fast edits ([0042](docs/decisions/0042-fast-edits.md)), attribute layers and the builder ([0043](docs/decisions/0043-attribute-layers-and-the-builder.md)), edits that hold ([0044](docs/decisions/0044-edits-that-hold.md)), limits for files ([0045](docs/decisions/0045-limits-for-files.md)), and our own names ([0046](docs/decisions/0046-our-own-names.md)). *Built.*
- Tools from §6 Phase 4, in this order after loop cut: Slit (`X`); extrude with a bend (pipe bends); Connect, Bridge (`C`) and Weld; the clean-up tools; hard and soft edges with normals display; symmetry; the basics (hide and isolate, slide, duplicate and instances, split and join); and the keyboard shortcut viewer last. Each with live parameters and HUD, each recording anchors so it survives base changes (**W14**). *Extrude and inset built* ([0038](docs/decisions/0038-extrude-and-inset.md)): `D` and `I`, dragged anywhere in the view; polygons together or each on its own (Keep together); border edges extrude too; the last one changeable in the Attributes panel. *Bevel built* ([0040](docs/decisions/0040-bevel.md)): `V`, dragged away from the edges or points; width, segments and profile. *Loop cut built* ([0047](docs/decisions/0047-loop-cut.md)): `F`, point at an edge to see the loop, click to cut or drag to slide; Cuts and Position in the Attributes panel. *Slit built* ([0049](docs/decisions/0049-slit.md)): `X`, click points over the shape, cut as seen (or through), editable until Esc or Enter. *Pipe bends built* ([0050](docs/decisions/0050-pipe-bends.md)): an extrude turns a corner first, with Angle, Direction, Bend radius and Segments, a smooth elbow or a mitre joint, keeping its full width. *Connect, Bridge and Weld built* ([0051](docs/decisions/0051-connect-bridge-weld.md)): `J` (now `A`, [0057](docs/decisions/0057-after-phase-4-testing.md)), `C` (or drag edge to edge), `M` and `Ctrl + M`.
- Hard edges and normals display
- Added after the shortcut viewer at the project lead's request ([0063](docs/decisions/0063-wireframe.md)): **wireframe** (Shaded, Shaded with wires, Wireframe, from a button beside the corner axes, which move to the bottom left; *built*), **cavity** (*built*, [0065](docs/decisions/0065-cavity.md)), **clickable corner axes** (look along an axis; *built*, [0066](docs/decisions/0066-clickable-axes.md)), and **automatic hard edges** (a switch in the Mesh menu, on as standard: shapes shaded by angle after every change; *built*, [0064](docs/decisions/0064-automatic-hard-edges.md))
- **Done when:** you can model a simple hard-surface prop (crate, sci-fi panel) start to finish.

### Phase 5 — Export pipeline
- Pivot editing and scene units, which export needs. *Built* ([0068](docs/decisions/0068-pivot-editing-and-scene-units.md)): the Object mode moves and turns pivots, with Pivot to centre, Pivot to world origin, Pivot to object and Object to world origin; each project saves its units
- OBJ and glTF export with triangulation, tangents and presets. *OBJ export and import built* ([0071](docs/decisions/0071-obj-export-and-import.md)): File → Export…, Export again and Import…; *glTF export built* ([0072](docs/decisions/0072-gltf-export.md)): binary `.glb`, each object at its pivot; *triangulation and tangents built* ([0073](docs/decisions/0073-mikktspace-tangents.md)): the viewport's cut, MikkTSpace tangents with UVs; *presets built* ([0075](docs/decisions/0075-export-presets.md)): Made for Godot (glTF) or Substance Painter (OBJ, triangles)
- Simple material slots: polygons assigned to a named material with a colour, so engines get their parts (real materials come with texturing). *Built* ([0074](docs/decisions/0074-material-slots.md)): a Materials list in the Attributes panel with Assign, Select by material, colours in the viewport, OBJ `.mtl` and glTF materials
- Round-trip test: model → Godot and Substance Painter, shape, hard edges, normals and scale correct (the full bake round trip is the UV phase's gate)
- Bake readiness, model part, and kit export (§8; **W13**, **W16**). *Bake check built* ([0077](docs/decisions/0077-bake-check.md)): File → Check for baking…, with Select and one-click fixes; *kit export built* ([0078](docs/decisions/0078-kit-export.md)): File → Export kit…, a Collision setting Godot reads
- **Engine export paths**: presets for Godot, Unity and Unreal Engine that save straight into the engine's project folder and export again on every save. *Built* ([0076](docs/decisions/0076-engine-export-paths.md)): a game project folder per project, the engine found from it, Export on every save, File → Export to game project. Godot and Unreal re-import by themselves. Unity reads glTF through its glTFast package (our own FBX writer would be extra work, and the Autodesk FBX SDK is ruled out). A live-link plugin per engine may come later
- A **splash screen** while the app starts (the app's name and version, and what it is loading), at the project lead's request. *Built* ([0079](docs/decisions/0079-splash-screen.md))
- **Done when:** a prop exports into Godot and Substance Painter and looks right (shape, hard edges, scale) with no fixes in other software.

### Next — Bevel control and Groove (the project lead's request, before Phase 6)
Planned in [0083](docs/decisions/0083-groove-and-bevel-control.md); to be built once the open pull requests are merged. Two pull requests:
- **Bevel control:** Weld overlapping points (on by default), Skew (−100% to +100%, with Flip side), Even width (measured straight across the edge) and Stop at overlaps. *Built* ([0085](docs/decisions/0085-bevel-control.md))
- **Groove** (K): edges turned into a thin strip and pushed in (a groove, a panel line) or out (a ridge) in one step; Width, Depth, straight or sloped Sides, neat ends. *Built* ([0086](docs/decisions/0086-groove.md))
- **Done when:** the project lead can bevel a prop's edges evenly and cut its panel lines without a separate bevel and extrude.

### Still to check by hand
Merged without the project lead trying them on his computer (2026-10-16); to recheck when he asks what is left, not before:
- The export pipeline end to end: OBJ and glTF export and OBJ import (#65, #66), triangulation and tangents in a Substance Painter bake (#67), materials in Godot and Substance Painter (#69), the presets (#70), exporting into a game project on save (#71), Check for baking (#72), kit export and Godot collision (#73).

### Next — Feedback from testing the Phase 5 stack (the project lead's choices, 2026-10-15)
- **View dice** (replaces the clickable X/Y/Z arrows, bottom-left of each view; behaviour like 3ds Max's ViewCube, nothing of its look): click a face, edge or corner to look from there (26 views); **Ctrl + click** looks from the opposite side; **Display → Switch to orthographic when clicking the dice**, and with it on, orbiting away goes back to perspective; **hold Ctrl while orbiting** to snap to the nearest dice view. *Built* ([0084](docs/decisions/0084-view-dice.md)).
- **Live mirror replaces Symmetry** (pulls Phase 7's mirror generator forward): the mirror stands at the **world centre** unless moved, so moving an object to one side moves its mirror image the other way; the mirrored half is **part of the same object**, live, welded where the halves meet, and exports as one mesh; today's "every edit made twice" Symmetry goes (old projects keep their mirrored edits); the mirror is shown as a **faint see-through red plane** with a clearer red line where it crosses the ground, a line in Top, Front and Right. Its own milestone and decision record when built. *Built* ([0088](docs/decisions/0088-live-mirror.md)).
- **One calm look for windows and menus** (the project lead: the Export window works but is hard to look at). A shared set of parts in `ui` that every window uses, starting with Export, then Export kit, Check for baking, Settings, Navigation controls and the smaller dialogs:
  - a header (title, one short line saying what the window is for);
  - in Export, **Made for** is one drop-down (the project lead's choice over big tiles): Godot, Unity, Unreal Engine and Substance Painter, each with its file type beside it, then **Custom settings** for any other program, which opens the file settings;
  - Export is **one window with a short side list** (the project lead's choice over folded boxes): Made for in the title bar; General first (objects, an **Includes** row of tick buttons for Mesh, Materials, Skeleton, Animations, the preset's file settings as one locked line, a plain explanation of the game project export with where it goes, and what goes out); then Materials, Mesh, Rig and Animations, each listed only once its feature exists and showing its state ("2 of 3", "off"); Game project last. Presets fill every section in; Custom settings unlocks them;
  - settings grouped in cards, each with a small heading;
  - two columns, the name on the left at a fixed width and its control on the right, as in the Attributes panel;
  - **segmented buttons** for two to four choices (Metres | Centimetres) instead of rows of radio buttons, a drop-down list for longer ones;
  - explanations short and in one place: one line under a card, the rest on hover;
  - settings a preset fixes shown as a quiet summary line ("glTF: metres, Y up, triangles") instead of greyed-out controls, with your own settings under an **Advanced** fold;
  - a footer: Cancel and the main button on the right, the main one filled in the accent colour; Enter presses it, Esc cancels;
  - spacing, widths and text sizes from `theme`, so every window matches. Menus get the same pass: even padding, keys right-aligned, dividers between groups.
  - *Built* ([0087](docs/decisions/0087-one-look-for-every-window.md)): the parts are `ui::dialog`; Export, Settings, Check for baking, About and the questions use them, and the menus are roomier. A preset's settings show as a locked line with Custom settings to change them (no Advanced fold); the Keys window keeps its own resizable frame.

### Phase 6 — 3ds Max sprinkles
- **Apply transforms** first (the project lead's request, 2026-10-16): an object's rotation and scale (and position, if chosen) back to none while it looks the same, as Blender's Apply. *Built* ([0089](docs/decisions/0089-apply-transforms.md)).
- **Stable element names** first (review A3, [0044](docs/decisions/0044-edits-that-hold.md)): edits keep their exact elements even when an earlier edit renumbers them, which the edit list makes common. *Built* ([0090](docs/decisions/0090-stable-element-names.md)): every element named after where it came from, names saved; used for now to tell apart points on the same spot, finding by name first waiting for the edit list.
- **The modifiers palette** (§5.1) between the viewport and the right column, starting with Edit Layers. *Built* ([0091](docs/decisions/0091-edit-layers.md)): a column on the viewport panel's right edge, with Add edit layer.
- **Edit Layers interface (W5):** add/disable/reorder/collapse layers (*built*, [0091](docs/decisions/0091-edit-layers.md): an Edit layers panel beside Objects; collapse is Merge down); the edit list *built* ([0092](docs/decisions/0092-the-edit-list.md)): edits opened in the Attributes panel, eyes, Keep editable, Freeze up to here, Delete edit, kept / parts missing / skipped marks; an **edit list** inside each layer, where any recorded edit can be opened and changed, pinned to keep it editable, frozen, or switched off (see the 2026-10-13 log entry); edit lower layers and see upper ones re-evaluate, each layer finding its elements again by their anchors (**W14**); warning badges only for what truly can't be found
- Snap to and align tools (W7). *Built* ([0093](docs/decisions/0093-snap-to-and-align.md)): Tools → Snap to… (pivot onto a clicked object's pivot) and Align… (a live window: world axes, Lowest / Middle / Pivot / Highest, turn to match).
- Optional quad menu. *Built* ([0094](docs/decisions/0094-quad-menu.md)): off as standard; a right-click in the viewport opens Show, Modes, Edit and Tools around the mouse.
- **Done when:** the user's Max workflow of stacking edit layers feels right.

### Phase 7 — UVs
Everything in §7, as one phase after the core modelling and workflow phases ([decision 0034](docs/decisions/0034-uv-phase-after-modelling.md)), brought before advanced hard-surface at the project lead's request ([decision 0095](docs/decisions/0095-uvs-before-advanced-modelling.md)):
- Primitives come with ready-made UVs and seams. *Built* ([0095](docs/decisions/0095-uvs-before-advanced-modelling.md)).
- The UV Editor panel, UV mode and UV Edit layout. *Built* ([0096](docs/decisions/0096-uv-editor.md)): layouts switched from the menu bar's tabs, each keeping its own arrangement; the editor draws the selected objects' UVs and selects with the viewport's selection; the UV mode selects whole islands; seams in orange.
- Numbered checker (asked for by the project lead). *Built* ([0107](docs/decisions/0107-numbered-checker.md)): Display → Surface → Numbered checker, cells A1 to H8 in the accent colour's gradient, shown on the model and behind the UVs in the UV Editor.
- Trim sheets. *Built* ([0106](docs/decisions/0106-trim-sheets.md)): one per project, full-width rows saved with it (format 33), made in the UV Editor's Trim menu and drawn there; UV → Put on trim row lays strips straight along a row, filling its height, from its left end; Pack and the bake check leave trim pieces alone.
- Bake readiness for UVs. *Built* ([0105](docs/decisions/0105-bake-readiness-for-uvs.md)): in the Check for baking window (and UV → Check for baking…): no UVs, hard edges that aren't seams, folded, outside, overlapping, stacked and too-close pieces per texture, uneven texel density, each explained with a one-click fix where there is one.
- UVs that survive modelling. *Built* ([0104](docs/decisions/0104-uvs-that-survive-modelling.md)): after each edit that adds or rewires polygons, the broken ones it made are solved around the kept polygons (or laid out as pieces of their own), seams keep pieces apart, and recorded UV work replays on a changed shape by its anchors.
- UV editing directly on the model. *Built* ([0103](docs/decisions/0103-uvs-on-the-model.md)): in the UV mode, W, R, S and T then a drag in the viewport move, turn and scale the selected islands' texture, which follows the mouse; the piece under the mouse lights up in both views.
- Checker texture, and a stretch heatmap with explained fixes. *Built* ([0102](docs/decisions/0102-checker-and-stretch.md)): Display → Surface (Colours, Checker, Stretch), worked out per pixel from the UVs; the UV Editor coloured the same, explaining the polygon under the mouse.
- Packing: pixel padding aware of mipmaps, rotation, nesting, groups, texture sets per material, stacking identical parts. *Built* ([0101](docs/decisions/0101-packing.md)): shapes packed on a grid of cells, padding in texture pixels, a texture per material or object, stacking identical pieces.
- Texture settings: texture size and texel density for the scene, density display and normalising. *Built* ([0100](docs/decisions/0100-texture-size-and-texel-density.md)): saved with the project, the Texture menu, Set and Match texel density, the pixel grid.
- Live unwrap, relax and straighten; automatic straightening of strips and axis alignment. *Built* ([0099](docs/decisions/0099-unwrap-relax-straighten.md)): LSCM with our own solver, pieces at the object's texel density where they were, Unwrap as seams change. Mirrored halves come with packing (stacking).
- Seams: marking and clearing (around selected polygons too), from hard edges, suggested seams in hidden places, selecting them. *Built* ([0098](docs/decisions/0098-seams.md)); painting is Paint select on edges, then Mark seams.
- Editing UVs by hand. *Built* ([0097](docs/decisions/0097-editing-uvs-by-hand.md)): the viewport's move, rotate and scale gizmo laid flat in the UV Editor, shared-location moves that keep islands whole, quarter turns and flips, and the UV ring of the E menu over the editor.
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

### Phase 8 — UV powerhouse and baking
At the project lead's request (2026-10-16; [decision 0108](docs/decisions/0108-uv-powerhouse-and-baking.md)), before advanced hard-surface. It covers everything the other 3D programs do well for UVs, full control over every automatic step, four ideas no other app has, and a baker inside the app.

**UV tools and controls:** what the other programs do well (recreated in behaviour only), and full control over every automatic step (the project lead: the best, most controllable UV tool there is, never random):
- Unwrapping: pins (E menu and UV menu, no key: the project lead's choice), unfolding one way only, and angle-based and size-keeping methods beside LSCM, the best picked for each piece. *Built* ([0109](docs/decisions/0109-pins-and-unwrap-methods.md)): Pin and Unpin (the `uv_pin` corner layer, recorded edits) held by Unwrap, Relax and Straighten; UV → Unfold across only / up only; UV → Unwrap method (Keep shapes, Keep shapes smoother by linear ABF, Keep sizes by ARAP, Automatic by the lowest distortion); the E ring's turns and flips grouped as Turn & flip, with Pin, Unpin and a Stitch & cut group (X and C, coming with milestone 8); file format 34.
- Fully controllable automation: every automatic tool (suggest seams, unwrap, straighten, pack, mending) shows all its settings, and the same model and settings always give the same result, never a random one. *Built* ([0110](docs/decisions/0110-controllable-automation.md)): the UV menu's settings section (Seam suggestions: corner angle, hard edges, where cuts hide; Unwrapping: method, strips and how square, live; Relaxing: strength; Packing: with Fit; Mending: on, and its two limits, saved with the project), every choice recorded with its edit (file format 35), determinism tested.
- Seam preferences ("prefer a seam here", "never a seam here") and a stretch budget per piece, obeyed by every automatic tool. *Built* ([0111](docs/decisions/0111-seam-preferences-and-stretch-budget.md)): UV → Prefer seams here / Never seams here / Clear seam preference (an edge layer), UV → Stretch budget for pieces (a polygon layer) and a budget for every piece in Seam suggestions; Suggest seams, Unwrap and Seams from hard edges obey; over-budget pieces slit from their most squeezed point to the border, the most hidden way; file format 36.
- Lock a piece (nothing automatic changes it), and per-piece settings (method, straightening, density) kept for every re-unwrap. *Built* ([0112](docs/decisions/0112-locked-pieces-and-piece-settings.md)): UV → Lock pieces / Unlock pieces (a polygon layer; Unwrap, Relax, Straighten, Pack, density, trim, Suggest seams and mending leave locked pieces alone) and UV → Piece settings (method, straightening, texel density per piece, used by every unwrap); file format 37.
- Straighten any quad piece by its edge flow, and the texture locked in place while moving points
- Seams: a slider from fewer seams to less stretch, a seam brush, symmetric seams, seams along loops and rings
- Projections: planar, box, cylindrical and spherical with an on-model gizmo, project from view, and world-aligned UVs for modular kits
- Editing pieces: stitch and cut in the UV Editor, align and distribute, orient by an edge, lattice warp
- Precise placement: UV snapping (to texture pixels, other pieces' edges and corners, trim rows, a grid of your own), exact U, V, turn and size in the Coordinates panel, and transform pivots (the selection's middle, each piece's own, a corner, a point you place)
- UV brushes in the UV Editor: grab, relax and pinch
- Selection by overlap, mirror, stretch and similarity, grow and shrink in UV, shortest path; isolate the selected pieces; the UV Editor following the viewport's selection or keeping its own; copying UVs between similar pieces and objects
- Packing: packing groups (kept together, in order, with their own padding or region), locked pieces, packing into a region, keeping the grain's direction, texel density overrides per object, UDIM tiles
- Several UV sets per object: add, name, copy and switch them, each with its own texture, for high-resolution texturing of big models
- Lightmap UVs: a UV set made for Godot's baked lighting
- Seeing and checking: always-on warnings for overlapping and mirrored pieces in the UV Editor, stretch, pixel size and texel density shown on each piece, and measuring in texture pixels and real centimetres. *Built* ([0115](docs/decisions/0115-seeing-and-checking.md)): overlapping pieces (and folded polygons) tinted red and mirrored ones purple, counted top right, per texture as packing groups them; View → Numbers on pieces; View → Measure, then drag, in pixels and centimetres on the surface (Esc stops).
- The numbered checker in each object's own colours (some red, some blue), to tell objects' UVs apart at a glance
- Seeing textures: a real texture behind the UVs and on the model, UV layout export (PNG, SVG), separate angle and size stretch views, a mipmap preview
- Layout snapshots: named versions of a layout to switch between, compared before and after side by side. *Built* ([0114](docs/decisions/0114-layout-snapshots.md)): the UV Editor's Snapshots menu takes (the shown objects' UVs and seams), renames, restores (one recorded edit per object, one undo step), compares (drawn faintly under the current UVs), updates and removes them; kept in the project; file format 38.
- Explained auto-unwrap: one button for seams, unwrap, straighten, density, pack and lightmap UVs, then every step listed to tweak or undo on its own
- Unwrap recipes: unwrap a whole kit at once with saved steps

**New ideas, in no other app:**
- Player-view UVs: seams hidden and texture detail placed by how the player sees the asset, and a texels-per-screen-pixel view
- UV intents: pieces keep what they were told to be (a strip, on a trim row, a mirror, a density, this way up) after any change
- "Why is this here?": every automatic seam, size and packing choice explains itself and can be changed *Built* ([0116](docs/decisions/0116-why-is-this-here.md)): only while the UV Editor's View → Explain choices is ticked (the project lead's choice); pointing at a seam gives Suggest seams' reason worked out with the current settings (`uv::seams::suggest_explained`) or says it was made otherwise, with the menu commands that change it; pointing at a piece lists what decides it (locked, trim row, pins, own settings, density against the aim, place, warnings, stretch) with fixes.
- Paint-aware detail and seam preview: more texture on edges and corners where wear gets painted, and bake seams shown before baking

**Baking in the app** (it moves here from the texturing plan; the baked maps feed texturing's smart masks later):
- The baker: normal (matching Godot's tangent space), ambient occlusion, curvature, thickness, colour ID, position and height
- High to low: _low / _high pairing, exploded bakes, floating details, an automatic cage shown on the model with a push slider and cage painting
- Live bake preview, and bake problems shown on the model (missed rays, wrong parts, skew), each explained with a fix
- Bakes for the pipeline: to vertex colours, a whole kit at once, stacked and mirrored pieces moved aside for the bake, and straight into a Godot material (normal map and an ORM texture)
- **Done when:** a modular kit is unwrapped, packed and baked in Aventail 3D and looks right in Godot, and changing a model afterwards keeps its UVs and their intents.

### Phase 9 — First use, and English as a language package
A brief phase between the UVs and advanced modelling ([0119](docs/decisions/0119-roadmap-after-the-uvs.md)), from the first testers' feedback (2026-10-18): the app felt overwhelming to people who weren't there from the first cube.
- **A side palette that slides out** to show each button's name, not only its icon.
- **High contrast:** a setting that makes edges, points and the selection stand out much more.
- **The selection mode button shows it has choices:** a small dropdown arrow, so people see there are several kinds.
- **Labels on everything:** every button and field says what it is (on hover at least, and in words where there's room).
- **Tidier menus:** commands moved to where people look for them; nothing important hidden deep.
- **The UV menu in the UV Editor too** ([issue #115](https://github.com/Rhyslos/Aventail-3D/issues/115)): the same grouped menu beside Texture, Trim, Snapshots and View, so UV commands are where the work is; and **the UV settings in a panel of their own**, a tab beside Attributes, instead of UV → Settings.
- **Hard edges explain themselves** ([issue #114](https://github.com/Rhyslos/Aventail-3D/issues/114)): marking or clearing hard edges while automatic hard edges by angle is on warns that the angle decides instead, and says where to turn it off.
- **English as the one language package:** every piece of text the app shows comes from one English package through one translation layer (Project Fluent's `fluent` crates are the likely choice: Apache-2.0/MIT), and a check fails CI when new text skips it. Only English ships, but adding a language later means adding a package, not reworking the app. From then on, new text goes into the English package. (The rest of [0117](docs/decisions/0117-language-support.md)'s plan, a language setting, local numbers, fonts and translations, waits until other languages are wanted.)
- **Done when:** a tester new to the app finds the main tools without help, sees what is selected on any screen, and no text in the app skips the English package.

### Phase 10 — Advanced hard-surface
- Every tool here carries UVs and seams along from the start (the builder, decision 0043)
- Booleans, weighted normals, better bevel, generators (array, mirror)
- **Mirror as a modifier** that can be turned on per object (the project lead, 2026-10-18), separate from the live mirror (symmetry), which stays as it is: the modifier mirrors one object's shape; the live mirror makes edits work on both halves
- Shell, curves and splines, and our own smoothing modifier
- Friends join as testers around this phase
- Game bevel and modular kit helpers (§6; **W15**, **W16**)
- Generators, booleans, weighted normals and the game bevel are added to the modifiers palette (§5.1)

### Phase 11 — Solis v1: real-time rendering
**Solis** is the app's own renderer ([0119](docs/decisions/0119-roadmap-after-the-uvs.md)). It will have two modes, with a switch between them: a light, fast **real-time** renderer, and later a slow, accurate **path tracer** (Phase 20). Version 1 is the real-time one, and it lights the viewport too. To be planned in detail; the starting ideas:
- **What makes it worth using:** it shows a game asset the way the game engine will. The same lighting model and tone mapping as Godot (and later Unity and Unreal), so what you see in the app is what you get in the game, and the textures painted in the app (Phases 12–13) look right before export. A stylised mode with toon shading and outlines (the texturing plan's "almost a priority") comes with it.
- **Built on the GPU through `wgpu`**, as the viewport is, so it runs on Vulkan, Metal and DirectX 12.
- **Its own crate**, `render` or similar, knowing nothing about the app's interface, so it could later be used elsewhere (see Phase 20 on a Blender add-on).
- Turntables and stills straight from it, for showing assets off.

### Phases 12 and 13 — Texture painting, then materials
Texture painting (Phase 12), then materials with a node editor (Phase 13): see "Texturing and a material builder" below, which still needs its planning talk.

### Phase 14 — Alpha release
The app released for alpha testing on the project lead's website, and maybe on Steam: installers for Windows, macOS and Linux, updates, a way for testers to report problems, and the manual online.

### Phase 15 — Bug fixes and polish
A long phase of fixing what alpha testers find, and polishing what they use most.

### Phase 16 — Scripting
Scripting so people can add their own tools and automate work. **LustScript** (the project lead's own language, free and open source) is the default scripting language, and plain Lua works too, in LustScript files ([0119](docs/decisions/0119-roadmap-after-the-uvs.md)). The design in §4.7 (written for Luau, [0011](docs/decisions/0011-lua-scripting.md)) is revisited for LustScript when this phase is planned; LustScript's licence must be permissive (MIT or Apache-2.0, say) for the app to use it (§9).
- Script runner with the sandbox, time limit and permissions
- The scripting API: scene, selection, operations and commands (undoable like any edit)
- Scripts as commands: in menus, the command search and hotkeys, with parameters in the Attributes panel
- Script manager (install, enable, disable) and a reload button for script authors
- API reference and a "your first script" page in the manual; example scripts
- **Done when:** a user can write a short script (say, "bevel every hard edge of the selection by 2 mm") that shows up as a command, can be undone, and can't harm files outside what it was allowed.

### Phases 17 and 18 — Rigging and skinning, then animation (far future)
Rigging and skinning first (Phase 17), animation after (Phase 18). Rough contents, to be designed properly when the time comes:
- **Aim (project lead, 2026-10-13):** every animation tool is for game developers, helping them animate for games in both first and third person: first-person arms and weapons, third-person characters, and loops and clips that export cleanly to engines.
- Joints/bones in the Objects panel hierarchy; bone display and editing
- Skinning: bind mesh to skeleton, weight painting, weight tools (normalize, mirror, smooth)
- Animation: keyframes on transforms, timeline and dope sheet, curve editor, playback in the viewport
- Export skinned and animated meshes to glTF for Godot

### Phase 19 — Gauntlet: effects
**Gauntlet** is the app's effects system: physics, particles and simulations, to be planned when its time comes (the project lead is new to effects, so it starts with research into what game effects artists use). Two ideas from the start:
- **Scripting at its heart:** effects driven and extended with LustScript.
- **Baked for game engines:** effects made in the app export baked (as flipbook textures, vertex animation or animated meshes), so they cost little in the game.

### Phase 20 — Solis, full: path tracing
The second Solis mode: a path tracer for slow, accurate, high-quality images, using the GPU to the full (compute shaders; hardware ray tracing where `wgpu` supports it). Whether Solis also becomes a **Blender add-on** is decided here, once the real-time Solis has proved itself: Blender already has a strong path tracer (Cycles) and real-time renderer (EEVEE), so a Solis add-on is only worth making for what Solis does differently (the engine-matching look, the stylised mode), not as a general renderer.

### Later: other languages
Translations, a language setting, local numbers and fonts for other alphabets ([0117](docs/decisions/0117-language-support.md)), when other languages are wanted; Phase 9's English package makes this an addition, not a rework.

### To be designed: Texturing and a material builder (Phases 12 and 13)
At the project lead's request (2026-10-13): a built-in texture painter, so no Substance Painter subscription is needed, and a material builder after it. Both still need a planning talk before they get phases and milestones. Rough parts:
- the maps baked in Phase 8 (normal, AO, curvature, thickness, position, ID) as its starting point;
- a layer stack with fill and paint layers and masks;
- smart masks driven by the baked maps;
- 3D painting;
- export to each engine's texture layout;
- later, a node-based material builder.

**Game-first, not a Painter copy.** The tools should be shaped by game work (texel density, trim sheets, texture sets, engine channel packing, mid-poly shading) and may well differ from Painter's. They still need to be good enough for a hobbyist generalist.

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
  - **Fix normals** (first named Fix facing), which turns every polygon of a closed shape to face outward in one click;
  - **Select by facing**: polygons facing a direction (up, say) within an angle, or facing like the one clicked.
- 2026-10-13 — Where new tools go: basic modelling tools in Phase 4 (hide and isolate, slide, duplicate and instances, split and join); advanced tools and modifiers in Phase 7 (shell, curves and splines, pipe bends, the smoothing modifier); pivot editing and scene units with export in Phase 5. Testers join around Phase 7. Keys: **Bridge `C`** (free). Slit `X`, Loop cut `F` and `Z` for the camera were proposed but clash with keys in use (X/Y/Z axis locks, F frame selected); to be settled before those tools are built.
- 2026-10-13 — Keys settled for tools to come (per 0028):
  - **`X` Slit**, **`F` Loop cut**, **`C` Bridge**;
  - **`Z` frames the selection** (what `F` does now), and `H` still frames everything;
  - **axis locks move to `Shift + X / Y / Z`** (`Ctrl` would clash with undo, redo and cut).

  The switch happens in one go with the first of Slit or Loop cut, updating the manual, the tool menu and the keyboard viewer at the same time.
- 2026-10-13 — Fifth Phase 4 milestone, **bevel**, with the controls agreed earlier: `V`, then drag away from the selected edges (or points) to widen, back to narrow; Segments and Profile in the Attributes panel only; Width, Segments and Profile changeable as the Last edit. Edges get a strip `width` into each polygon, with rows along a profile from flat to round; corners slide along plain edges or meet both offsets; where strips meet, they share their end rows or a corner polygon closes the gap; with a single bevelled edge, the end polygon takes the strip's end. Points are cut off with one polygon. Border and non-manifold edges are skipped. Project files move to format version 4 ([0040](docs/decisions/0040-bevel.md)). Fixed along the way: a mesh's topology could stay stale after polygons were added to it.
- 2026-10-13 — At the project lead's request, planned for Phase 6 with Edit Layers: an **edit list per object** showing every recorded edit in order. Clicking one shows its settings in the Attributes panel, editable, and everything after it replays. A **keep (pin) toggle** keeps chosen edits editable, and unpinned old ones can be frozen (baked) to keep big models fast. Edits can be switched off to see the model without them, and edits that no longer fit after an earlier change say so plainly. Until then, only the last edit stays changeable.
- 2026-10-13 — Whole-app review at the project lead's request ([docs/reviews/2026-10-13-whole-app-review.md](docs/reviews/2026-10-13-whole-app-review.md)): two ways a session can save a project that won't reopen, whole-history replay on every drag frame, no mesh attribute layers, weak re-finding when earlier edits change, no limits on hostile files, and saving that follows symbolic links. A Foundations step before Loop cut and Slit is proposed, awaiting the project lead's decision.
- 2026-10-13 — The project lead approved the Foundations step before Loop cut and Slit. First part, **never lose a project** ([0041](docs/decisions/0041-saves-that-always-reopen.md)): every save and backup copy is checked to open again before it replaces anything; one safe-write helper that never writes through links; the two ways to save an unopenable project closed at the source; harmless damage (bad hard edges) left out when opening; huge polygons checked quickly; object id and copy number overflow prevented.
- 2026-10-13 — Foundations, second part, **fast edits** ([0042](docs/decisions/0042-fast-edits.md)): the cache keeps the shape before each layer's last edit, so adding, dragging or changing the last edit applies only that edit, whatever the history holds; bevel no longer slows with the square of the selection; edits are found again through a grid, with a tolerance that fits each element; undo memory counts mesh connections; nothing waits as a drag ends. An edit-speed check joins the heavy-mesh check.
- 2026-10-13 — At the project lead's request, bevel **Profile goes from -100% to 100%**: 100% round, 0% flat, -100% curved inward (hollow, like a cove). Older projects keep their profiles; file format unchanged (version 4 stored the number already, only its limit widened).
- 2026-10-13 — At the project lead's request, planned for Foundations 3: **edits stretch new rows along**. When a shape's settings add points (more segments) after an edit, a new point on an edge between a moved point and one that stayed put moves part of the way (halfway along, half the move), so a lifted cube's new rows spread over its whole height. Applies to move, rotate and scale edits; extrude walls are new polygons and keep their own rows. Edits record where the edges at the edge of what they moved ran (about 50 bytes for a lifted cube top, a few KB for large patches), only on shapes that still have settings; only does work when settings change. Needs a file format version bump; older edits keep today's behaviour.
- 2026-10-14 — Foundations 3 split in two. First part, **attribute layers and the builder** ([0043](docs/decisions/0043-attribute-layers-and-the-builder.md)): named values per point, edge, polygon or corner, with hard edges as the first; a mesh builder through which tools say where new elements come from, so every layer is carried over (extrude, inset and bevel moved onto it); deleting polygons. No change users see. The second part brings edit ids and switches, better re-finding, stretching new rows, and saving attribute layers in file format version 5.
- 2026-10-14 — Foundations 3, second part, **edits that hold** ([0044](docs/decisions/0044-edits-that-hold.md)): every edit gets an id and on/off and pinned switches (no interface until the Phase 6 edit list); edits trust their ids while the shape's structure below them is unchanged, so changing an earlier edit no longer loses later ones; file format version 5 saves all of it and plain meshes' attribute layers. Stable element names (review A3) are left for a smaller step before the Phase 6 edit list.
- 2026-10-14 — **Stretching new rows along an edit, dropped** at the project lead's request after testing: blending a move along the edges, then across the polygons it touched, fixed boxes with segments on every axis, but round shapes given more sides still broke (new points on the curve lie on none of the old polygons), and a full fix would be a deep problem for little reward at this stage. New rows follow the shape's settings only; Loop cut adds rows where they're needed. The attempt is kept in the history of pull request #37.
- 2026-10-14 — Foundations 4, **hostile files** ([0045](docs/decisions/0045-limits-for-files.md)): one limits rulebook applied while files are read (512 MB unpacked, a 2 GB memory budget every list is paid from, at most 100,000 objects and edits and about 30 million polygons), a size guard skipping edits that would grow a shape past 30 million polygons, recovery that doesn't offer a copy whose restore closed the app, private backup folders, and fuzz tests. Foundations complete; next, the renames, then Loop cut and Slit.
- 2026-10-14 — The agreed renames done ([0046](docs/decisions/0046-our-own-names.md)): Attributes panel, Objects panel and Paint select in the app, the manual and the code; Snap to in the plans. Decision records and dated log lines keep the names as they were.
- 2026-10-14 — Sixth Phase 4 milestone, **loop cut** ([0047](docs/decisions/0047-loop-cut.md)), with the project lead's controls: `F`, point at an edge to see the loop, click to cut or press and drag along the edge to slide it; Cuts in the Attributes panel (the mouse wheel was asked for only if Cinema 4D or 3ds Max use it, and neither does: Cinema 4D uses the middle button, a heads-up display and its attribute panel; 3ds Max's SwiftLoop has no wheel control); Cuts and Position changeable as the Last edit. The agreed key switch is done with it: `Z` frames the selection, axis locks are `Shift + X / Y / Z`, plain `X` waits for Slit. The ring runs through four-sided polygons, cutting every edge at the same share from the same side; polygons where it ends take the new points. Project files move to format version 6. The milestone "Loop cut and knife" is split: Slit (knife) is next, as its own milestone.
- 2026-10-14 — After the first build, the project lead added **Ctrl + wheel for Loop cut's number of cuts**, for those who want it. It doesn't clash: Ctrl + wheel grows the selection, but while Loop cut is in use nothing is selected, so it changes the cuts instead (as it changes the brush's size with the brush). Works while pointing and while dragging a cut.
- 2026-10-14 — At the project lead's request, **Ctrl + wheel changes the count of the tool in use, as the standard for every tool from now on** ([0048](docs/decisions/0048-ctrl-wheel-changes-a-tools-count.md)): Bevel's segments and Loop cut's cuts now, every later tool's count when it has one. It does what the arrows beside the count in the Attributes panel do: the next use before, the edit being dragged during, the last edit after (one undo step per notch). With tools without a count, Ctrl + wheel still grows the selection.
- 2026-10-14 — At the project lead's request, the README's progress tracker brought up to date with this plan: the Foundations step and the basics milestone added to Phase 4 in the agreed order, Slit, Bridge and the clean-up tools under their current names; pivot editing and scene units, simple material slots and engine export paths in Phase 5; stable element names and the edit list in Phase 6; shell, curves and splines, pipe bends and the smoothing modifier in Phase 7; texturing before scripting and rigging; the old panel names replaced. This document's tool list (§6) and roadmap (§10) now say the same, with our own names (Edit Layers, Slit, Weld).
- 2026-10-14 — Seventh Phase 4 milestone, **Slit** ([0049](docs/decisions/0049-slit.md)), with the controls chosen earlier (as in Cinema 4D, visible only with a switch): `X`, click or drag points over the shape, each snapping to a point, edge or polygon; the shape is cut as the line grows; drag a point to move it, click the line to add one, Ctrl + click to remove one, Shift for 15° steps; Esc or Enter finishes; **Cut through** in the Attributes panel. Each stretch of the line sweeps a fan from the eye and cuts the polygons it crosses; a point inside a polygon turns the cut there; a line ending inside a polygon doesn't cut it. Stored as the line in the object's own space, so it follows changes to the shape. The builder can blend a polygon's corners at a spot inside it. Project files move to format version 7.
- 2026-10-14 — At the project lead's request, **pipe bends that keep their size move from Phase 7 into Phase 4**, as the milestone after Slit: needed for the simple prop that tests the Phase 4 gate. Controls chosen: **extrude with a bend**: press `D` and drag the length as usual, then Angle, Direction, Bend radius and Segments in the Attributes panel make a smooth elbow that keeps its full width; Ctrl + wheel changes Segments ([0048](docs/decisions/0048-ctrl-wheel-changes-a-tools-count.md)). Rotating a ring with a Keep round switch was offered and not chosen.
- 2026-10-14 — Eighth Phase 4 milestone, **pipe bends** ([0050](docs/decisions/0050-pipe-bends.md)), with the controls chosen: extrude with a bend, set in the Attributes panel. The extrude turns first, then goes on straight for its Distance. With a Bend radius, Segments rings turn rigidly around a pivot, so every ring keeps its full size. With none, the old end ring moves onto the halfway plane, stretched by 1 / cos(half the angle): a mitre joint that keeps the pipe round. Angle up to 180° (150° as a mitre); Direction 0 turns up. Extrude now has a count, so Ctrl + wheel changes the bend's Segments while it is in use. Project files move to format version 8.
- 2026-10-14 — Ninth Phase 4 milestone, **Connect, Bridge and Weld** ([0051](docs/decisions/0051-connect-bridge-weld.md)), with the project lead's controls: `J` connects points or edges (Cuts as the Last edit); `C` bridges two selected borders or two polygons at once, or with nothing to bridge selected makes Bridge a drag-from-edge-to-edge tool (Segments with Ctrl + wheel, and Twist); `M` welds at the middle and `Ctrl + M` drags a point onto a target (D, first asked for, is Extrude; Ctrl + D is kept for Duplicate). Project files move to format version 9.
- 2026-10-14 — Tenth Phase 4 milestone, **clean-up** ([0052](docs/decisions/0052-clean-up.md)), with the project lead's controls: in the element modes `Delete` deletes (leaving a hole) and `Backspace` dissolves (the surface stays closed); in the Model mode both still delete objects. Fill hole and Flip normals in the tool menu and Mesh menu, no keys; Fix facing in the Mesh menu; Select flipped and Select similar facing (30°, from the selected polygons, instead of a typed direction) in the Select menu. Project files move to format version 10.
- 2026-10-14 — Eleventh Phase 4 milestone, **hard and soft edges** ([0053](docs/decisions/0053-hard-and-soft-edges.md)), with the project lead's choices: no keys (tool menu and Mesh menu), and the normals switch in the viewport's Display menu, beside a Hard edges switch (on as standard, light cyan, while editing elements). In the Polygons mode the commands use the selected polygons' edges. Hard edges by angle (30° to start, as the Last edit) and Select hard edges added. Guides are drawn for shapes of up to 50,000 polygons. Project files move to format version 11.
- 2026-10-14 — Twelfth Phase 4 milestone, **symmetry** ([0054](docs/decisions/0054-symmetry.md)), with the project lead's choice: a live mirror switch on the palette (and Tools menu), with X, Y and Z in the Attributes panel (X as standard). It mirrors moving, rotating and scaling points across the shape's own planes, matched by position (0.01 cm); points on a plane stay on it; the lower-numbered of a chosen pair leads. The modelling tools are not mirrored yet. No file format change.
- 2026-10-14 — Thirteenth Phase 4 milestone, **the basics**, in two pull requests ([0055](docs/decisions/0055-basics.md)). First part, with the project lead's keys: Shift + H hides, Ctrl + H isolates, Alt + H shows everything (objects in the Model mode through their Visible switch; polygons in the element modes as a view setting, not saved or undone); Ctrl + C / Ctrl + V copies, Alt + C / Alt + V instances (a new object kind that shows another object's shape), with Duplicate and Instance in the Model mode's tool menu. Deleting an original bakes its instances. Project files move to format version 12, with instances listed beside the object list.
- 2026-10-14 — The basics, second part ([0055](docs/decisions/0055-basics.md)): **slide** with the project lead's control, Alt + drag on the Move gizmo (Alt + drag elsewhere still orbits), each point along the edge to the neighbour that best follows the drag, as `SlidePoints`; **Split off** (Mesh menu, moves the polygons into a new object) and **Join objects** (the Model mode's tool menu and the Mesh menu), no keys. Project files move to format version 13.
- 2026-10-14 — Last Phase 4 milestone, the **keyboard shortcut viewer** ([0056](docs/decisions/0056-shortcut-viewer.md)), placed as the project lead asked: a **Keys** button on the menu bar, after Help, opening a window of every key and mouse action, grouped, with a search field; read from the app's own key constants, with a test that every command's key is listed. Every Phase 4 milestone is built; the gate (a simple hard-surface prop modelled start to finish, and the heavy-mesh check) is for the project lead to try.
- 2026-10-14 — **Fixes and keys after Phase 4 testing** ([0057](docs/decisions/0057-after-phase-4-testing.md)), the first follow-up pull request, with the project lead's choices: **Tab** steps straight back to the Model mode (finishing a slit, putting a modelling tool away); **Connect moves to A**, **Fill hole gets J**, C always bridges; in Weld and Bridge a click selects and a drag from a highlighted point or edge is the tool's, and the line snaps to its target; the bend Angle resets to 0 after each extrude and on a tool change, and the bend rows are number fields; Dissolve across a corner sharper than 30° asks first, with an "I know what I'm doing" tickbox and "Don't warn me again" (back on in Settings). Fixes: gone edges aren't drawn, dissolved edges lose their hard marks, Bridge and Fill hole mark new sharp edges hard, and polygons are cut into triangles properly (brought forward from Phase 5), so bent and dented ones draw and pick right; and a double-click that lands on the gizmo (moved there by the first click) still selects the loop.
- 2026-10-14 — **Closed slits** ([0058](docs/decisions/0058-closed-slit.md)), the second follow-up from testing, asked for by the project lead: with Slit, clicking the first point (three or more points) closes the line into a loop and finishes it. A loop wholly inside one polygon cuts its shape in as a polygon of its own, the ring round it cut in two by edges from the polygon's corners; a loop across polygons cuts like a line, the closing stretch included. Saved as the first point again at the end, so the file format stays at 13.
- 2026-10-14 — **Symmetry for every edit and the Symmetry menu** ([0059](docs/decisions/0059-symmetry-for-every-edit.md)), required by the project lead after testing: with Symmetry on, every modelling edit is recorded as one mirrored edit (`Operation::Mirrored`), made on the selection and its mirror together (extrude, inset, bevel, delete, dissolve, fill hole, flip, hard and soft edges) or as a mirrored copy (loop cut, slit, connect, bridge, weld, bent extrude; left out when it would be the edit itself); the Last edit changes both sides. The Symmetry menu (an arrow beside the palette button, placed as he asked) holds the X, Y and Z switches (moved from the Attributes panel), the middle (object's pivot, object's middle, world centre), offsets, **Weld the middle now** (a button that joins the halves once, as an undoable edit, within the weld distance, 0.1 cm as standard) and Don't cross the middle (off), the last two asked for after the first look. An automatic weld was tried first and dropped: once points joined, moving them back couldn't part them. A live weld that parts again comes with the mirror generator in Phase 7. Project files move to format version 14.
- 2026-10-14 — **Mode keys put a tool away** ([0060](docs/decisions/0060-mode-keys-put-tools-away.md)), the project lead's choice: with a modelling tool in use, 1, 2 or 3 (or a mode button) puts it away and selects in that mode, staying in it; without one, the keys work as before.
- 2026-10-14 — **The Keys window draws a keyboard** ([0061](docs/decisions/0061-drawn-keyboard.md)), the project lead's request: used keys lit with a coloured mark per situation, point at or click a key for its jobs, chips to hide situations, search, mouse actions underneath; US, UK, Norwegian, Danish, Swedish / Finnish, German and French layouts, guessed from the system language and remembered.
- 2026-10-14 — **Spirals** ([0062](docs/decisions/0062-spirals.md)), the project lead's choice of automatic climbing plus a number field: with a bend radius the Angle goes up to 3600°; past one turn each loop climbs just clear of the one before (the pipe's width and a 5% gap), plus **Rise per turn**; under one turn nothing changes. Segments count each turn's rows (256 at most for a whole spiral). Mirrored copies wind the other way. File format version 15.
- 2026-10-14 — **Four more Phase 4 milestones** at the project lead's request ([0063](docs/decisions/0063-wireframe.md)): wireframe, cavity, clickable corner axes and automatic hard edges by angle. Their choices: the corner axes move to the bottom left with a small row of display buttons beside them; clicking an axis swings the view to look along it (again for the other side); no key for the looks; automatic hard edges per object, on by default at 30°, saved with the project. **Wireframe built**: Shaded, Shaded with wires and Wireframe in the Display menu and the new row; the renderer can leave faces out; remembered for every view. The hard edge highlight they asked for already exists (Display → Hard edges).
- 2026-10-14 — **Automatic hard edges** ([0064](docs/decisions/0064-automatic-hard-edges.md)), as the project lead clarified (replacing the per-object choice in 0063): a Display menu switch right below Hard edges, on as standard, that shades every shape by angle (30°) after every change, whatever its edges are marked, so bends and extrudes stay smooth. Shading only: the shape and its edits are untouched, edges set by hand are kept for when it's off, and the cyan guides follow it. What export writes is to be settled in Phase 5.
- 2026-10-14 — **Cavity** ([0065](docs/decisions/0065-cavity.md)): ridges brightened and creases darkened on the surfaces, worked out on screen from their facing (as Blender's screen-space cavity), before edges and the grid are drawn; on as standard, from a Cavity button beside Shading and the Display menu. At the project lead's request, **Automatic hard edges moves to the main Mesh menu**, just above Hard edges by angle.
- 2026-10-14 — **Clickable corner axes** ([0066](docs/decisions/0066-clickable-axes.md)), with the project lead's choice from 0063: in a perspective view, clicking X, Y or Z on the corner marker swings the view round its pivot, at the same distance, to look along that axis from its side (Z from above); again, from the other side. Straight down or up lines up like the Top view. The marker grows a little, with round letters to click. At the project lead's request, Cavity loses its own button and sits with Hard edges and Normals in the Display menu. Every Phase 4 milestone is built again. The heavy-mesh check was run: all targets met except turning a new 3-million-polygon shape into GPU geometry, 3.1–3.5 s against 3 s on the 2-core test machine (by marks it was 2.7–3.2 s there), a background step. To make it cheaper, shading groups and face facings are now worked out on every core, and a heavy shape whose edges turn sharp or soft while dragging follows the move at once and is reshaded in the background.
- 2026-10-14 — **Clearer names** ([0067](docs/decisions/0067-clearer-names.md)), as agreed with the project lead: Fix facing → Fix normals, Show only the selection → Isolate, Instance → Duplicate as instance, Select similar facing → Select same facing (and `Knife` → `Slit` in code). The README's introduction is rewritten for game developers: "A calm, easy-to-learn 3D modeller for game assets", inspired by Cinema 4D and 3ds Max but not a copy.
- 2026-10-14 — At the project lead's request, this document's own wording follows the README ([0067](docs/decisions/0067-clearer-names.md)): the app takes inspiration from Cinema 4D and 3ds Max and grows into its own tool; principle 4 is now *Familiar first, our own way after*; "like C4D" and "C4D-style" dropped from principle 1, the UI specification's heading, the Phase 0 gate and the modelling menu. Comparisons that say where an idea came from stay.
- 2026-10-15 — **Pivot editing and scene units** ([0068](docs/decisions/0068-pivot-editing-and-scene-units.md)), the first Phase 5 milestone, with the project lead's choices: the **Object mode moves the pivot** (W and R move and turn it, the shape stays; scaling does nothing; no key for the mode yet), stored as an offset between each object's pivot and its shape so primitives and recorded edits are untouched; **Pivot to centre**, **Pivot to world origin**, **Pivot to object…** (click the target) and **Object to world origin** in the Object mode's E menu and the Tools menu; **units are saved with each project** (metres as standard, or centimetres; an undo step to change), with the units for new projects kept in Settings, and an export scale to come with export. Project files move to format version 16.
- 2026-10-15 — **Extrude freedom** ([0070](docs/decisions/0070-extrude-freedom.md)), after the project lead's sloped-patch extrude sheared: extrudes go **straight out** as standard (the piece's average facing), with **Direction** (Straight out, Along normals, Along X/Y/Z), **End size**, **Steps** (Ctrl + wheel) and **Flat end** in the Attributes panel, before and after; older extrudes keep the first way (Along normals) so old models keep their shape. The **Scale gizmo's squares** now scale both axes equally (his request). Project files move to format version 17.
- 2026-10-15 — **OBJ export and import** ([0071](docs/decisions/0071-obj-export-and-import.md)), the second Phase 5 milestone, with the project lead's choices: File → Export…, Export again and Import…, with no keys yet; shading exported as seen (by angle while Automatic hard edges is on). Exports are prepared once (`fileio::export`: which objects, polygons or triangles, normals and UVs, in the file's units and up axis, points kept shared) so every format writes the same thing. Import makes each OBJ object a plain mesh in one undo step, hard where the file's normals break, and reads back the units and up axis this app writes into its files.
- 2026-10-15 — **glTF export** ([0072](docs/decisions/0072-gltf-export.md)): binary glTF (`.glb`) becomes the Export window's standard kind of file; each object is a node at its pivot holding its shape around the pivot, always metres, Y up and triangles; vertices split where shading (or UVs) break; one grey material for now. Checked with the Khronos glTF validator: no errors or warnings. New dependency: `serde_json` (MIT or Apache-2.0).
- 2026-10-15 — **Consistent triangulation and MikkTSpace tangents** ([0073](docs/decisions/0073-mikktspace-tangents.md)): exports cut polygons with the viewport's own cut (a dented polygon's triangles are tested to match), and glTF exports of shapes with UVs carry MikkTSpace tangents, worked out with `bevy_mikktspace` (byte-identical to the reference, no dependencies, Zlib AND (MIT OR Apache-2.0)) on the UVs as the file holds them; vertices split where tangents differ. Checked with the Khronos validator: no errors or warnings.
- 2026-10-15 — **Material slots** ([0074](docs/decisions/0074-material-slots.md)), with the project lead's choices: a **Materials** list in the Attributes panel (colour, name, polygon count, **Assign**; Add material, Remove last), **Select → Select by material**, no new keys. Each polygon holds a slot number (a polygon attribute, carried by every edit), each object a list of named, coloured materials (an instance uses its original's); same-named materials are one material in an export; Join merges lists by name. Assigning is a recorded edit (`SetMaterial`), so undo, anchors and symmetry work. The viewport draws each material's colour (a per-object palette); OBJ exports write `usemtl` and a `.mtl` file, glTF one material per name with its colour; OBJ import reads `usemtl` and the `.mtl` colours. The standard material is the viewport's grey, so nothing changes until materials are used. Project files move to format version 18.
- 2026-10-15 — **Export presets** ([0075](docs/decisions/0075-export-presets.md)): a **Made for** row at the top of the Export window (the project lead chose the File menu for exporting, so presets live in its window): **Godot** (the standard: glTF, each object at its pivot, metres, Y up, triangles) and **Substance Painter** (OBJ, metres, Y up, cut into the viewport's triangles, materials as texture sets). Changing a setting by hand switches to *Your own settings*. The units are the export scale for now; Unreal Engine's centimetres come with engine export paths.
- 2026-10-15 — **Engine export paths** ([0076](docs/decisions/0076-engine-export-paths.md)), with the project lead's choices: a **game project folder per project**, saved with it, and **Export on every save** switched on once a folder is chosen. The engine is found from the folder (`project.godot`; `Assets` and `ProjectSettings`; a `.uproject`), and its preset used: glTF for Godot, Unity (through glTFast) and Unreal Engine 5, so pivots and materials survive. File → Export to game project exports now. For safety the app never creates folders there and only replaces files it wrote itself (named after the project). Project files move to format version 19.
- 2026-10-15 — **The bake check, model part** ([0077](docs/decisions/0077-bake-check.md)): File → Check for baking… (and a button in the Export window under the Substance Painter preset; no keys) lists in plain words what would spoil a bake: polygons facing the wrong way, edges shared by three polygons, polygons with no area, points on the same spot, n-gons bent out of flat (over 5% of their size) or cut into slivers (under 2°), loose points, and `_low` / `_high` names without a partner. Each has Select; where the fix is certain (fix normals, weld points on the same spot, delete loose points) a one-click Fix, as one undo step. A fix that makes something else worse says so. Quads aren't checked for bends or slivers.
- 2026-10-15 — **Kit export and Godot collision** ([0078](docs/decisions/0078-kit-export.md)): **File → Export kit…** exports each selected object (with what's inside it) as its own file, named after it, its pivot at the file's origin (turned and sized as in the scene), with the Export window's settings, into the game project folder or a chosen one (never creating folders, only replacing the app's own files). Each object has a **Collision** setting (None, The shape itself, Wrapped, A box) that exports made for Godot turn into Godot's name endings (`-col`, `-convcol`, and a `-convcolonly` box beside it). Project files move to format version 20.
- 2026-10-15 — **Splash screen** ([0079](docs/decisions/0079-splash-screen.md)), the last Phase 5 milestone: the main window's first frames show the app's cube icon in the accent colour, its name and version, a progress bar and the step under way (starting the graphics, getting the workbench ready, opening the project given at start, checking for unsaved work), one step done between frames while its name shows; at least 0.6 s so a quick start doesn't flash. Every Phase 5 milestone is built; the gate (a prop into Godot and Substance Painter looking right) waits for the project lead's test.
- 2026-10-15 — **Bends keep their shape on stretched objects** ([0081](docs/decisions/0081-bends-keep-their-shape-on-stretched-objects.md)), after the project lead found a bend widening a pipe on a prop: a bent extrude records how the object is stretched (its scale on each axis over their average) when it is made or its settings change, and is worked out as seen, so the pipe stays round. Polygons bent each on their own now keep their size. File format 21.
- 2026-10-15 — **Extrudes, insets and bevels keep their shape on stretched objects** ([0082](docs/decisions/0082-edits-keep-their-shape-on-stretched-objects.md)), after the project lead saw a bevel come out uneven: any edit that measures across the shape records the object's stretch and is worked out as seen (`mesh::stretch::as_seen`), as bends are since 0081. File format 22.
- 2026-10-15 — **Groove and bevel control, planned** ([0083](docs/decisions/0083-groove-and-bevel-control.md)), with the project lead's choices: a new tool **Groove** (not "Crease", which means a smoothing crease in other programs) on **K**, turning edges into a thin strip pushed in or out in one step; Bevel gains **Weld overlapping points** (on by default), **Skew** with Flip side, Even width and Stop at overlaps. Built next, as its own milestone, once the open pull requests are merged.
- 2026-10-15 — **Orthographic view** ([0069](docs/decisions/0069-orthographic-view.md)), asked for urgently by the project lead: **P** (his choice; Ctrl + O, his first idea, is Open project) switches the active view between perspective and orthographic, keeping its direction, pivot and the size of things at the pivot. The free orthographic view orbits, pans and zooms; orbiting Top, Front or Right turns them into it, as in Blender (his choice). The Cameras menu lists Orthographic beside the others.
- 2026-10-15 — **Navigation feel** ([0080](docs/decisions/0080-navigation-feel.md)), after the project lead's test (orbits swinging around nothing, panning too fast, zooming close fighting back): an orbit or pan started over empty space turns around something on screen (the surface point last pointed at, the selection's middle, everything's middle, or the view's middle); a pan grabs the surface under the mouse; zooming over empty ground never moves the pivot farther away; within 10 cm zooming moves in steady steps through surfaces, the pivot kept 10 cm ahead; no movement is lost at the start of a drag; the turning-point ring shows whenever you orbit and can be switched off (his choice).
- 2026-10-16 — **The view dice** ([0084](docs/decisions/0084-view-dice.md)), with the project lead's choices: a cube in each view's bottom-left corner replaces the X/Y/Z arrows; click a face, edge or corner (26 views, as 3ds Max's view cube) or Ctrl + click for the opposite side, with arrows to the hidden faces when looking straight at one; hold Ctrl while orbiting to snap to its views; **Display → Switch to orthographic when clicking the dice**, the view going back to perspective once orbited away.
- 2026-10-16 — **Bevel control** ([0085](docs/decisions/0085-bevel-control.md)), first half of 0083: Even width (each edge sized by its angle so its new polygon is a right-angled edge's width), Skew with Flip side (the side following each chain of edges round), Stop at overlaps (the whole bevel narrowed until it fits) and Weld points (new points within 1 mm welded); new bevels have all but skew on, older ones keep none. File format 23.
- 2026-10-16 — **Groove** ([0086](docs/decisions/0086-groove.md)), second half of 0083: **K** in the Edges mode cuts a thin line along the selected edges (drag in) or raises a ridge (drag out), made as a bevel at half the width then the strip pushed in or out; Groove width (standard 1 cm) and straight or sloped Sides, all changeable under Last edit; ends inside a surface close with a clean wall; the tool menu grows to 15 places, Groove last so Delete keeps its place; file format 24.
- 2026-10-16 — **One look for every window** ([0087](docs/decisions/0087-one-look-for-every-window.md)), with the project lead's choices from the mockups: shared parts in `ui::dialog` (title bar with ×, a side list of sections with their state, rows with the name on the left, segmented choices for two to four options, drop-downs for more, a footer with the main button in the accent colour; Enter presses it, Esc cancels and no longer selects nothing while a window is open). The Export window is rebuilt: **Made for** (a drop-down: Godot, Unity, Unreal Engine, Substance Painter, Custom settings) in the title bar; **General** (Objects, Includes, the preset's settings as one locked line, where every save exports to, What goes out) and **Game project** sections; Materials, Mesh, Rig and Animations join the list with their features. Settings, Check for baking, About and the questions follow; menus get roomier rows and keys lined up on the right.
- 2026-10-16 — **The live mirror** ([0088](docs/decisions/0088-live-mirror.md)), with the project lead's choices, replaces Symmetry's mirrored edits: the Symmetry button gives the selected objects a mirror across the world's X, Y and/or Z planes through the world centre (movable in its menu); the mirrored twin is part of the object (drawn, counted and exported with it), follows every edit and move, and is welded to it where points meet at the mirror (0.1 cm as standard, live). The tools make each edit once; Weld the middle now, Mirror through and Don't cross the middle are gone; old projects keep their mirrored edits. A faint red see-through sheet with a clearer line on the ground shows each mirror. File format 25.
- 2026-10-16 — **Apply transforms** ([0089](docs/decisions/0089-apply-transforms.md)), first in Phase 6 at the project lead's request: Mesh → Apply transforms (and the Objects panel's Object menu) applies rotation and scale (the usual choice, also the last place of the Model tool menu), position, rotation, scale or all three; the object looks the same, its shape placed by a recorded edit (`PlaceShape`, so primitives keep their settings and it undoes in one step), objects inside it stay put, instances are left alone. No key yet (Ctrl + A selects all; he can pick one). File format 26.
- 2026-10-16 — **Stable element names** ([0090](docs/decisions/0090-stable-element-names.md)), review A3: each point and polygon an edit makes is named after the edit and where it came from (the builder's `PointFrom`, by the sources' names), a primitive's base after where its points sit in its box, a polygon after its corners; edits keep their anchored points' names, and finding points again by place tells apart points on the same spot by them. Finding by name first was built and dropped for now: it changed what existing chains of edits make; it waits for the edit list. Names are saved: file format 27.
- 2026-10-16 — **Edit layers and the modifiers palette** ([0091](docs/decisions/0091-edit-layers.md)), Phase 6: the modifiers palette arrives as a thin column on the viewport panel's right edge with **Add edit layer** (also in the Mesh menu), which adds an empty layer on top of each selected shape. A new **Edit layers** panel, a tab beside Objects (not a sub-list in the Objects tree, which would crowd it), lists the selected object's layers top first with their edits; eye to switch off, double-click to rename, right-click or the Layer menu to move up or down, merge down (Max's collapse; both layers on) or delete. New edits go into the top layer that is on (a new layer if none is). The project lead was asleep, so the button's behaviour was the recommended choice; no new keys. Saved layouts get the panel added beside Objects. No file format change.
- 2026-10-16 — **The edit list** ([0092](docs/decisions/0092-the-edit-list.md)), Phase 6, as asked on 2026-10-13: any edit in the Edit layers panel opens in the Attributes panel in the Last edit's place (On, Keep editable, its settings, Freeze up to here, Delete edit, Close; a new edit closes it). Freeze bakes the base and every edit up to the chosen one into a plain mesh, refused past a kept or switched-off edit or layer, so the shape stays the same and the edits after it still apply. Edit rows have eyes and marks only for real problems (parts missing, skipped). Finding by name first stays held back for the project lead's look. No new keys (he was asleep); no file format change.
- 2026-10-16 — **Snap to and Align** ([0093](docs/decisions/0093-snap-to-and-align.md)), Phase 6 (W7): Tools → Snap to… moves the selected objects' pivots onto a clicked object's pivot (3ds Max's Quick Align, our name); Tools → Align… opens a window at the bottom left of the viewport (world axes; this object's and the target's Lowest, Middle, Pivot or Highest; Also turn them to match), shown live, one undo step, Cancel takes it back. Pivot to object, Snap to and Align share one way of picking the target. No keys (the project lead was asleep): the Tools menu only.
- 2026-10-16 — **An optional quad menu** ([0094](docs/decisions/0094-quad-menu.md)), the last Phase 6 milestone: switched on in Settings → Interface (off as standard), a right-click in the viewport (not a right drag, and never one that ends something under way) opens four lists around the mouse, 3ds Max's layout with our contents: Show, Modes, Edit (undo, edit layers, Snap to, Align) and the current mode's Tools from the command list. The recommended controls were used (the project lead was asleep).
- 2026-10-16 — **UVs before advanced hard-surface** ([0095](docs/decisions/0095-uvs-before-advanced-modelling.md)), at the project lead's request: the UV phase becomes Phase 7 and advanced hard-surface Phase 8 (it needs nothing from it; its tools will carry UVs from the start). Controls chosen by the project lead: the UV Editor navigates and uses keys exactly as the viewport (Q/W/R/S/T, 1/2/3), seams from the E menu and UV menu with live unwrapping. **Rule from now on: keys stay the same everywhere, and the E menu is in every area of the app with the tools for where the mouse is.** First milestone: every primitive comes unwrapped (box, flat, round with cones as fans, ball), cut along seams, pieces at their real relative sizes; the new `seam` edge layer.
- 2026-10-16 — **The UV Editor** ([0096](docs/decisions/0096-uv-editor.md)), second UV milestone: Panel::UvEditor (optional in a layout; Window → UV Editor shows or puts it away); the menu bar's **UV Edit** tab switches to its own layout (viewport and editor side by side) and the UV mode, **Standard** back, each keeping its arrangement (saved apart). The editor draws the selected objects' UVs (island borders brighter) and selects with the one element selection; the UV mode is a polygon mode whose selections take whole islands, in the editor and the viewport, with its 3D gizmo off until the UV tools come. Navigation and H/Z as the viewport's (H/Z go to whichever the mouse is over). Seams drawn in orange in the UV mode and with Display → Seams.
- 2026-10-16 — **Editing UVs by hand** ([0097](docs/decisions/0097-editing-uvs-by-hand.md)), third UV milestone, with the controls chosen in 0095 (the viewport's keys): W/R/S/T show the gizmo laid flat in the UV Editor (red arrow across, blue up, ring, boxes, middle square), live and one undo step, Shift finer, snapping to the square's tenths, the snap angle and scale steps, Esc or a right click cancels. Moves use shared location (corners of the same point at the same spot move together, so islands never tear). Recorded as `Operation::TransformUvs` with anchors; file format version 28. E over the editor (and in the UV mode) shows the UV ring: gizmos and Frame selected in the object ring's places, Turn left/right 90° and Flip left to right/upside down (no keys, none asked for), and seams, unwrap, relax, straighten and pack greyed until built. Fix: a put-away UV Editor no longer takes H and Z where it used to be.
- 2026-10-16 — **Seams** ([0098](docs/decisions/0098-seams.md)), fourth UV milestone, from menus as the project lead chose (no keys): Mark and Clear seams on the selected edges (or around the selected polygons) from the UV ring and the UV menu; Seams from hard edges (as shown); Suggest seams (`uv::seams::suggest`: hard edges and corners over 66°, then each piece cut to a disk along its least visible edges: facing down or back, concave, on corners); Select seams. Marking changes only the marks; the UVs follow with live unwrapping (next). Painting is Paint select on edges. `Operation::SetSeams`, file format version 29; seams orange in the UV Editor.
- 2026-10-16 — **Unwrap, Relax and Straighten** ([0099](docs/decisions/0099-unwrap-relax-straighten.md)), fifth UV milestone: LSCM unwrapping in the `uv` crate (own sparse solver; pieces that can't lie flat cut open in a hidden place and the cut marked as a seam; strips of square quads laid out as straight grids; each piece turned to its smallest box, a quarter turn to match how it lay, sized to the object's texel density, put where it was). Live unwrapping as the project lead chose: seam changes record an Unwrap of the pieces either side in the same step (UV menu switch, on). Relax (springs) and Straighten. `Operation::Unwrap`, `StraightenUvs`, `RelaxUvs`; file format version 30; `scene` depends on `uv`.
- 2026-10-16 — **Texture size and texel density** ([0100](docs/decisions/0100-texture-size-and-texel-density.md)), sixth UV milestone: `scene::TextureSettings` (square size 16–16384 px, 2048 standard; density in px/m, 512 standard) saved with the project and changed by `Command::SetTexture` (the UV Editor's Texture menu, UV → Texture); the editor shows the selection's density against the aim and the pixel grid when zoomed in; Set and Match texel density (`Operation::SetTexelDensity`, density in texture widths per scene unit). File format version 31.
- 2026-10-16 — **Packing** ([0101](docs/decisions/0101-packing.md)), seventh UV milestone: `uv::pack` places pieces by shape on a grid of cells (half the padding each, 128–512 across), lowest then leftmost, each turned to its smallest box and tried a quarter turn round; the largest common scale found by halving; padding in texture pixels (8 standard), half to the edges; a texture per material (on) or per object (off); identical pieces stacked when asked (off). Recorded per object as `Operation::PackUvs` (each island's polygons and move); file format version 32. Every tool in the UV ring is now built.
- 2026-10-16 — **Checker and Stretch** ([0102](docs/decisions/0102-checker-and-stretch.md)), eighth UV milestone: a Surface choice in the viewport's Display menu (Colours, Checker, Stretch; remembered, for every view). Mesh vertices on the GPU carry UVs (split where UVs split); the shader draws the checker (one square per 128 texture pixels, tinted red across and green up) and the stretch per pixel from screen derivatives (uneven = ratio of the two stretches, room = their product against the object's density; green, yellow at 1.5, red at 2, purple for no room). `uv::stretch` measures the same per polygon for the UV Editor, which colours polygons and explains the one under the mouse with its fix.
- 2026-10-16 — **Editing UVs on the model** ([0103](docs/decisions/0103-uvs-on-the-model.md)), ninth UV milestone: in the UV mode W, R, S and T then a drag anywhere in the viewport (the recommended controls, used because the project lead couldn't be asked) change the selected islands' UVs so the texture follows the mouse: a least-squares fit from the islands' corners on screen to the texture turns the drag into a UV move (applied the other way), rotate by 0.01 rad per point and scale by e^(0.005 per point), both snapping; one `TransformUvs` per object, one undo step. The polygon under the mouse in either the viewport or the UV Editor lights up in the other. No file format change.
- 2026-10-16 — **UVs that survive modelling** ([0104](docs/decisions/0104-uvs-that-survive-modelling.md)), tenth UV milestone. `uv::mend` runs after every operation that adds, removes or rewires polygons (`Operation::mends_uvs`). It judges the polygons the edit changed and marks as broken those that are squashed, mirrored, stretched 3 times one way, or 3 times off in size against those around. It marks seams where changed, unbroken polygons now meet with different UVs. Each cluster of broken polygons is solved by LSCM with the kept corners next to it held, joining the kept part with the longest shared border (seams part it from others). Clusters with nothing kept next to them, or still bad after solving, are laid out like Unwrap and slid inside the square. All steps are local (15 ms for one extrude on 40,000 polygons). Bevel keeps a bevelled seam on its strip's far border, and a point bevel blends its neighbours' UVs. Moving points never mends. No file format change.
- 2026-10-16 — **Bake readiness for UVs** ([0105](docs/decisions/0105-bake-readiness-for-uvs.md)), eleventh UV milestone. The Check for baking window gains the UV part: `uv::check::UvReport` (hard edges as shaded where UVs don't split; polygons folded or squashed against their piece; pieces outside the square; pieces twice or half the shape's area-weighted usual density) and `check_sheet`. `check_sheet` takes the pieces sharing a texture, grouped as Pack groups them, and draws them into a grid of up to 1,024 cells: middles in two pieces mean an overlap, or stacked when the pieces are identical; spreads that meet within the padding (one cell's slack) mean too close. Fixes are one step each: no UVs → suggested seams, unwrap and pack; hard edges → seams and unwrap; folded → unwrap; outside, overlapping, too close and accidental stacking → pack the texture's objects; uneven → set to the usual density. The list is found again when the scene, UV settings or hard-edge angle change. No new keys, no file format change.
- 2026-10-16 — **Trim sheets** ([0106](docs/decisions/0106-trim-sheets.md)), twelfth and last UV milestone. `scene::TrimSheet` holds full-width rows (name; bottom and top as shares of the height; at most 64; no overlaps), changed by `Command::SetTrim` and saved in file format 33. The UV Editor's Trim menu and UV → Trim sheet edit the rows: name, from top and height in pixels; Add a row; 2, 4, 8 or 16 even rows. The editor draws them as faint bands with names, fading past the right edge. UV → Put on trim row records `Operation::PlaceOnTrim` (the row kept as its band, so later sheet changes don't move pieces). `uv::trim::place` lays each piece: a straight grid for square-quad strips, the right way round, long side along the row, height filling it, end to end from the left. Pieces whose top and bottom lie on a row's edges (`uv::trim::on_row`) are left alone by Pack and by the per-texture bake checks. The bake check lists trim pieces sharing a texture with other pieces. Every UV milestone is now built; the phase gate waits for the project lead's bake test.
- 2026-10-16 — **Numbered checker** ([0107](docs/decisions/0107-numbered-checker.md)), at the project lead's request (a UV checker map with coloured, labelled cells, as in Cinema 4D, in the app's own colours). It is a fourth Surface choice: `uv::grid::numbered_checker` makes 8 × 8 cells labelled A1 to H8 with the app's own pixel font, each a 4 × 4 two-shade checker, blended from the accent's fill to a colour 120° back round the wheel. The image is 1,024 px with mipmaps, read by the UVs in the shader (`SurfaceDraw::NumberedChecker`, `set_grid_image`) and drawn dimmed behind the UVs in the UV Editor. It is made again when the accent changes. No file format change.
- 2026-10-16 — **Phase 8: UV powerhouse and baking** ([0108](docs/decisions/0108-uv-powerhouse-and-baking.md)), at the project lead's request. Every UV tool idea from the other 3D programs (recreated in behaviour only) and the project lead's control requests (seam preferences, stretch budgets, locked pieces, per-piece settings, snapping, exact numbers, pivots, packing groups, always-on warnings, numbers on pieces, snapshots, isolate and sync, several UV sets per object, measuring, explained auto-unwrap, the numbered checker per object, and no automatic step ever random), four new ideas (player-view UVs, UV intents, "why is this here?", paint-aware detail with a seam preview) and an in-app baker, all before advanced hard-surface, which becomes Phase 9 (scripting Phase 10, rigging Phase 11). Baking moves from the texturing plan to this phase. Each milestone asks about its controls first, as usual.
- 2026-10-16 — **Pins and unwrap methods** ([0109](docs/decisions/0109-pins-and-unwrap-methods.md)), Phase 8's first milestone. Controls asked first: Pin and Unpin in the E menu and UV menu, no key; the E ring's four turns and flips grouped into **Turn & flip** (a group's tools open on the same ring around its place), making room for Pin, Unpin and a **Stitch & cut** group whose tools come with milestone 8 (X cuts and C stitches in the UV Editor, the project lead's choice). Pins are a corner flag (`uv_pin`) set by `Operation::SetUvPins`; with two or more pins Unwrap solves around them and keeps the piece's place (mirrored pieces stay mirrored), with one it moves the piece back to it; Relax never moves them; Straighten keeps the first. UV → Unfold across only / up only fits the new layout over the old and keeps the other coordinate. UV → Unwrap method: Keep shapes (LSCM, standard), Keep shapes smoother (linear angle-based flattening, then LSCM), Keep sizes (as rigid as possible from LSCM, 12 rounds), Automatic (all three, the lowest area-weighted distortion, a later one must be 1% better). `Operation::Unwrap` gains `method` and `only`; file format 34.
- 2026-10-16 — **Fully controllable automation** ([0110](docs/decisions/0110-controllable-automation.md)), Phase 8's second milestone. Every rule the automatic UV tools followed is a setting in the UV menu's last section: Seam suggestions (cut corners sharper than 66°, cut along hard edges, hide cuts underneath and at the back / underneath / at the back / anywhere), Unwrapping (method, straighten strips, strip corners within 25°, unwrap as seams change), Relaxing (strength, 60 steps), Packing (with Fit: fast, standard, best) and Mending (on, mend when stretched 3×, when sized off 3×). The app remembers the first four, and each edit keeps the ones it used: `Operation::Unwrap` carries `uv::UnwrapOptions` whole, `RelaxUvs` its steps. Mending is the project's (`Command::SetMending`, saved in file format 35, carried to evaluation in `EvalContext`). Nothing uses chance; hash maps are only looked up; tests run every tool repeatedly and compare bit for bit. No keys.
- 2026-10-16 — **Seam preferences and stretch budgets** ([0111](docs/decisions/0111-seam-preferences-and-stretch-budget.md)), Phase 8's third milestone. Menu commands only (no keys): UV → Prefer seams here, Never seams here, Clear seam preference, and a Stretch budget submenu; a budget for every piece in UV → Seam suggestions. "Prefer" is where needed cuts go first; "never" keeps every tool's cuts off an edge unless a piece couldn't lie flat otherwise. A budget is how far off nearly all of a piece (all but its worst 5% by area) may be, as laid flat by Keep shapes; pieces over it are slit, one at a time, from their most squeezed or one-way-stretched point to their border along the most hidden way, a slit kept only if it helps, at most 24. Preferences are an edge layer, piece budgets a polygon layer; `SetSeamPreference` and `SetStretchBudget` record them; unwraps record the budget for every piece; file format 36. The distortion measure stops pricing a layout mirrored as a whole.
- 2026-10-16 — **Locked pieces and piece settings** ([0112](docs/decisions/0112-locked-pieces-and-piece-settings.md)), Phase 8's fourth milestone. Menu commands only: UV → Lock pieces, Unlock pieces and a Piece settings submenu, on whole pieces. Locks are a polygon layer (`uv_lock`) carried to polygons modelling makes from locked ones; every many-piece tool skips locked pieces (Unwrap, Relax, Straighten, density, trim, Pack, Suggest seams' changes, mending), hand edits still work. Piece settings (method, straightening, texel density in texture widths per unit) are polygon layers read by every unwrap through `uv::PieceSettings`. `SetUvLock`, `SetPieceSettings`; file format 37. The rest of Phase 8 is planned milestone by milestone in [docs/plans/phase-8.md](docs/plans/phase-8.md), at the project lead's request, so later sessions can pick it up.
- 2026-10-16 — **The UV menu in groups** ([0113](docs/decisions/0113-uv-menu-in-groups.md)), before milestone 5, because the UV menu only just fit a 900-pixel window. The project lead chose grouped submenus from three mock-ups: Seams, Turn and flip, Lay out, Pieces, then Texture, Trim sheet and Check for baking, then Settings. No commands or keys changed; later UV tools go into the group they fit.
- 2026-10-16 — **Layout snapshots** ([0114](docs/decisions/0114-layout-snapshots.md)), Phase 8's milestone 18, the first of the six the project lead asked for in least-work order (lightmaps and recipes left for later in the phase, still planned). In the UV Editor's own Snapshots menu, no keys (his choice). A snapshot is a name and, per object, every polygon's corner UVs and the seams (`mesh::uvs::UvLayout`), kept in the scene (`Command::SetSnapshots`, an undoable step); Restore records `Operation::RestoreUvs` on each object as one undo step, so it replays like any UV edit: polygons made since, or with other corners, keep their UVs, and only seams between restored polygons change. Compare draws one faintly under the current layout. File format 38.
- 2026-10-17 — **Seeing and checking** ([0115](docs/decisions/0115-seeing-and-checking.md)), Phase 8's milestone 15. Warnings always on: overlapping pieces and folded polygons tinted red, mirrored pieces purple, counted top right (`uv::warnings`, the bake check's overlap test on a 256-cell grid, per texture as packing groups pieces; stacked and trim pieces left out). View → Numbers on pieces (pixel size, px/m, worst stretch). View → Measure then drag (`uv::measure`: the line walked in 256 steps over the shapes' triangles, world distance added within pieces); Esc stops. Nothing new is saved.
- 2026-10-17 — **"Why is this here?"** ([0116](docs/decisions/0116-why-is-this-here.md)), Phase 8's milestone 23. Only while View → Explain choices is ticked (his choice). Reasons are worked out on demand rather than kept from the last run, so they explain seams in any project, opened or not: `uv::seams::suggest_explained` gives each suggested seam's `SeamReason` (hard edge, sharp corner with its angle, to lie flat, preferred, stretch budget), and a seam it wouldn't make is explained as made by hand or another tool. Fixes are named menu commands, not buttons in the box (no new clicks). Pieces are explained from their state. Nothing saved.
- 2026-10-17 — **Language support** ([0117](docs/decisions/0117-language-support.md)) added as Phase 9, the first phase after the UVs, at the project lead's request ("huge thing we need to fix"). Advanced hard-surface becomes Phase 10, scripting Phase 11, rigging Phase 12. Every text through one translation layer, a language setting, local numbers, fonts for other alphabets, then translations. Which languages, and whether the manual is translated, are asked before it starts.
- 2026-10-18 — **Select connected and the outside warning** ([0118](docs/decisions/0118-select-connected-and-outside-warnings.md)), from the project lead's testing. **Alt + A** (his choice) selects everything joined to the selection on the model, across seams, in any element mode; also Select → Select connected and UV → Pieces. The UV Editor's always-on warnings now also tint pieces outside the texture square amber (trim pieces left out).
- 2026-10-18 — **The roadmap after the UVs** ([0119](docs/decisions/0119-roadmap-after-the-uvs.md)), from the first testers' feedback and the project lead's new ideas, in his order. After the whole UV phase: a brief first-use phase (a sliding palette with names, high contrast, a dropdown on the selection mode, labels on everything, tidier menus) that also builds English as the one language package, so other languages can be added later without a rework (language support itself waits, replacing most of 0117); advanced modelling, with Mirror as a per-object modifier beside the live mirror; Solis v1, the app's own real-time renderer, made to show assets as the game engine will, in its own crate; texture painting; materials; an alpha release; polish; scripting with LustScript as the default and plain Lua too (its licence must be permissive); rigging; animation; Gauntlet (effects, baked for engines); Solis's path tracer, with a Blender add-on decided then.
- 2026-10-20 — **The Last edit ends when the selection changes** ([0120](docs/decisions/0120-last-edit-ends-when-the-selection-changes.md)), from the project lead's testing: setting up the next extrude kept changing the last one. Once something else is selected, the Attributes panel and Ctrl + wheel set the tool for its next use; a Change it button opens the earlier edit.
- 2026-10-21 — Issues #114 (a warning when marking hard edges while automatic hard edges by angle is on) and #115 (the UV menu in the UV Editor too, the UV settings in a panel of their own) added to Phase 9, the first-use phase, at the project lead's request.
