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
- Rigging, skinning and animation for game assets (Phase 9). Modelling and UV work always take precedence unless something urgent comes up.

### Out of scope (for now)
- Rendering beyond a good-looking viewport
- Sculpting, texturing and materials beyond a preview/checker

### Guiding principles
1. **Feel first.** The viewport and interaction must feel calm, clean and smooth, like C4D. A smaller tool that feels great beats a larger one that feels clunky.
2. **Tools are the gateway.** As in 3ds Max, a deep, reliable modelling toolset is the core value.
3. **Never lose work.** Undo is effectively unlimited. Base object properties stay editable no matter how much has been done since.
4. **Copy C4D first, diverge later.** The C4D layout is the starting target. Changes come from real use, not guesses.
5. **Free forever.** Every dependency must allow free use and redistribution (see §9).

---

## 2. Feature wishlist (from the user)

| # | Feature | Origin | Priority | Phase |
|---|---------|--------|----------|-------|
| W1 | C4D look and feel: calm, clean viewport, smooth interaction | C4D | Core | 0–1, ongoing |
| W2 | C4D layout: Object Manager + Attribute Manager + viewports | C4D | Core | 0, 2 |
| W3 | Near-unlimited undo history | Max | Core | 2 |
| W4 | Base object properties always editable, however much the object has been edited | Max | Core (architecture now) | 2, 7 |
| W5 | **Edit Poly stack**: a layered history of Edit Poly modifiers on top of each other. *"By far the most important tool."* | Max | **Top** (architecture now, UI later) | 2 (foundation), 7 (UI) |
| W6 | Zoom/orbit around the point under the mouse cursor | Max | High | 1 |
| W7 | Quick Align | Max | Medium | 7 |
| W8 | Clean, powerful UV unwrapping like C4D | C4D | Core | 6 |
| W9 | Max-like breadth of modelling tools | Max | Core | 4, 8 |

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
│  ├─ fileio/      # layer 2: .rhy projects, OBJ, glTF (FBX later), settings file
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
- **Phase 2–6 behaviour (C4D-like):** the user sees a normal object. Edits are silently recorded as `Operation`s in an implicit single layer. Primitive parameters (e.g. cube segments) remain editable in the Attribute Manager, and the recorded edits replay on top (**W4**).
- **Phase 7 behaviour (Max-like):** the Edit Poly stack UI exposes layers. Users can add a new Edit Poly layer, disable one, reorder, collapse, or go back down and edit an earlier layer (**W5**).
- **Collapse** ("Make Editable" in C4D, "Collapse to" in Max) bakes the stack into an `EditableMesh` base. This is always available as an escape hatch.

**The hard problem — topological naming.** Operations refer to elements (e.g. "extrude faces 12, 15"). If a lower layer or base parameter changes the topology, those references can break.
Strategy:
1. IDs are generated **deterministically** by each operation, so replaying an unchanged history yields identical IDs.
2. When an upstream change makes a reference invalid, the operation applies to whatever still matches and the layer shows a **warning badge**. Nothing is silently destroyed, and collapse/undo still work.
3. This is the same compromise 3ds Max makes; it does not need to be perfect, it needs to be predictable.

### 4.3 Undo / history (**W3**)
- Every user action is a `Command` that can apply and revert itself (storing a compact delta, not a full scene copy).
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
Nothing animation-related is built before Phase 9, but these rules avoid a rewrite later:
- **Generic attribute layers.** Mesh attributes are stored as named, typed layers, so per-vertex **skin weights** (bone index + weight) can be added later as just another layer.
- **Extensible object types.** The scene graph's object kinds are an enum/trait that can gain `Joint`/`Bone` and other node types without restructuring.
- **Clean transforms.** Each object has a local transform relative to its parent, and world transforms are always derived from it. No code may assume world-space transforms are stored.
- **No "static scene" assumption.** Evaluation takes a context parameter (initially empty), so a current time/frame can be added without changing every function signature.
- **glTF as the target.** glTF supports skins and animation, and Godot imports them, so the same export path extends to animated assets.

### 4.6 Rendering (`render`)
- wgpu, MSAA, and a stable 60+ fps target on ordinary hardware for game-asset-sized meshes (up to ~1M triangles).
- Draw passes: shaded (soft studio lighting, matcap optional) → wireframe overlay → selection highlight → gizmos/HUD.
- **GPU picking** (ID buffer) for vertices/edges/faces, and **depth readback** for cursor-point navigation.
- Only meshes that changed are re-uploaded.

---

## 5. UI specification — C4D first

### 5.1 Layout (default, dockable via `egui_dock`)
```
┌──────────────────────────────────────────────────────────────────┐
│ Menu bar                                                         │
│ Top command palette (primitives, generators, tools)              │
├──┬──────────────────────────────────────────┬────────────────────┤
│M │                                          │ Attribute Manager  │
│o │            Viewport(s)                   │ (selected object / │
│d │   single perspective or 4-up             │  active tool)      │
│e │                                          ├────────────────────┤
│s │                                          │ Object Manager     │
│  │                                          │ (hierarchy tree,   │
│  │                                          │  later Edit Layers)│
├──┴──────────────────────────────────────────┴────────────────────┤
│ Status bar: hints, selection counts, snapping state              │
└──────────────────────────────────────────────────────────────────┘
```
- **Right column order:** Attribute Manager on top, Object Manager below. This deliberately differs from C4D's default (user preference, 2026-10-10).
- **Left palette:** mode switches (Model / Object / Points / Edges / Polygons / UV) and snapping, like C4D.
- **Object Manager:** tree with drag-to-reparent, rename, visibility toggles, and later the Edit Poly stack shown as a sub-list.
- **Attribute Manager:** tabbed properties for the selection or active tool.
- A **Coordinates panel** (C4D's position/scale/rotation fields) can be docked under the viewport.

### 5.2 Visual style ("clean, pleasant, calm")
- Dark neutral greys with low contrast between panels; one accent colour for selection/active state (warm orange, as C4D uses for selected items).
- Viewport: soft vertical gradient background, subtle grid fading with distance, thin anti-aliased wireframe, gentle highlight on hover.
- Typography: Inter, small but legible sizes; generous padding.
- Smooth motion: camera moves are immediate (no laggy smoothing by default), but UI transitions are short and soft.
- **We copy layout and behaviour, not assets.** No Maxon or Autodesk icons, names, logos or screenshots go into the repo.

### 5.3 Navigation
| Action | Default (C4D-style) |
|---|---|
| Orbit | Alt + LMB |
| Pan | Alt + MMB |
| Zoom/dolly | Alt + RMB, mouse wheel |
| Frame selected / all | configurable (e.g. S / H) |
| Views | F1–F5: perspective, top, right, front, 4-up |

**Cursor-point pivot (W6)** — Max-style, on by default and toggleable:
- On orbit start, read the depth under the cursor; if it hits geometry, orbit around that 3D point. If it hits nothing, fall back to the selection centre or the last pivot.
- Wheel zoom dollies towards the 3D point under the cursor.
- Show a small pivot marker while orbiting.

### 5.4 Commands and hotkeys
- Every action is a named command with a configurable hotkey (stored in a settings file).
- **C4D-style modelling menu** (press a key, then a letter for the tool).
- **Max-style quad menu** on right-click as an optional "sprinkle" (Phase 7).
- Searchable **command palette** (Ctrl/Cmd+K or Shift+C, like C4D's Commander).

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

**Tools — Phase 8 (advanced hard-surface):**
- Booleans (union/subtract/intersect) via manifold
- Weighted normals
- Improved bevel (clamping, mitre patterns)
- Array, mirror and other non-destructive generators
- Subdivision preview with creases (optional for game assets)

---

## 7. UV toolset (W8)

- **UV mode** in the left palette, with a dockable **UV Editor** panel (C4D style).
- Seams: mark/clear, and **"seams from hard edges"** (the most common hard-surface workflow).
- Unwrap: angle/conformal (LSCM via `faer`) using marked seams; automatic charting via xatlas.
- Relax, straighten (rectify to grid), rotate/flip/align islands, stitch/weld.
- **Packing** with padding in pixels for a target texture size (xatlas).
- **Texel density** display and normalization across islands/objects.
- Checker texture and seam/stretch visualisation in the viewport.

---

## 8. Import / export

| Format | Direction | Phase | Purpose |
|---|---|---|---|
| Native project (`.rhy`) | in/out | 2 | Full scene, stacks, optional undo history |
| OBJ | in/out | 5 | Simple exchange, Substance Painter |
| glTF / GLB | out (in later) | 5 | Godot, Substance Painter |
| FBX | out | late, optional | Only if needed; own writer (Autodesk's SDK is not redistributable) |

Export options: triangulation (deterministic, so the bake matches the game), MikkTSpace tangents, scale and up-axis presets ("Godot", "Substance Painter"), export selected only.

### The `.rhy` project format
- **Header:** magic bytes `RHY\0`, a format version number, and a flags field. The app can tell a `.rhy` file apart from anything else instantly, and refuses or upgrades old versions cleanly instead of crashing.
- **Body:** the scene, objects, operation stacks and settings, serialized with `serde` and compressed with zstd (BSD).
- **Optional sections:** undo history, and a small thumbnail image so file browsers and a future "recent files" screen can show previews.
- **Versioning:** every format change bumps the version, and the loader keeps upgrade steps for older versions, so old projects always open.
- **Safe saving:** write to a temporary file, then rename over the old one, so a crash mid-save never corrupts a project. Keep the previous save as a `.rhy.bak`.
- **File association:** the installer registers `.rhy` so double-clicking a project opens the app.

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

Each phase ends with something runnable. The user tests the *feel* at the end of each phase before the next starts.

### Phase 0 — Skeleton
- Repository set up in Halberd's style: license, README with progress tracker, security policy, contribution rules, decision records, templates
- Cargo workspace with the crates above and `layer-check`; CI (fmt, clippy, tests and builds on three platforms, docs, cargo-deny, changelog check)
- Window with egui + egui_dock: empty viewport, Attribute Manager (top right), Object Manager (below it), palettes, status bar
- Theme: colours, Inter font, spacing
- **Done when:** the app launches into the C4D-style layout with movable/dockable panels.

### Phase 1 — Viewport and navigation
- wgpu renderer inside the egui viewport panel; gradient background, grid, a test cube
- C4D navigation; cursor-point orbit/zoom (W6); frame selected/all; perspective/ortho views; 4-up layout
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
- Tools from §6 Phase 4, each with live parameters and HUD
- Hard edges and normals display
- **Done when:** you can model a simple hard-surface prop (crate, sci-fi panel) start to finish.

### Phase 5 — Export pipeline
- OBJ and glTF export with triangulation, tangents and presets
- Round-trip test: model → Substance Painter bake → Godot looks correct
- **Done when:** a prop goes through the full pipeline without fixes in other software.

### Phase 6 — UVs
- Everything in §7
- **Done when:** that prop can be unwrapped and packed in Aventail 3D with clean bakes in Substance Painter.

### Phase 7 — 3ds Max sprinkles
- **Edit Poly stack UI (W5):** add/disable/reorder/collapse layers; edit lower layers and see upper ones re-evaluate; warning badges for broken references
- Quick Align and align tools (W7)
- Optional quad menu
- **Done when:** the user's Max workflow of stacking Edit Polys feels right.

### Phase 8 — Advanced hard-surface
- Booleans, weighted normals, better bevel, generators (array, mirror)

### Phase 9 — Rigging, skinning and animation (far future)
Starts only once modelling and UVs are mature. Rough contents, to be designed properly when the time comes:
- Joints/bones in the Object Manager hierarchy; bone display and editing
- Skinning: bind mesh to skeleton, weight painting, weight tools (normalize, mirror, smooth)
- Animation: keyframes on transforms, timeline and dope sheet, curve editor, playback in the viewport
- Export skinned and animated meshes to glTF for Godot

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
- **Later:** in-app help (F1 on a tool opens its manual page, tooltips link to it).

## 12. Ideas backlog

Possible features noted along the way. Not scheduled; nothing here blocks other work.

### Backlog (status: to discuss)
| Item | Why it matters for game assets | Depends on / relates to |
|---|---|---|
| Material slots / polygon selection sets | Each slot becomes a texture set in Substance Painter and a material in Godot | Face attributes (§4.1), export |
| Pivot / axis editing | The object's origin is where Godot places, rotates and snaps the asset; Quick Align needs it too | Object mode, W7 |
| Scene units and export scale | Godot works in metres; a wrong scale means re-scaling every import | Export presets (§8) |
| Reference images in viewports | Blueprints/concept art to model hard-surface parts against | Renderer, ortho views |
| Collision meshes | Godot builds collision from meshes named with suffixes like `-col` / `-colonly` | Object naming, glTF export |
| Mesh check before export | Catches flipped normals, n-gons, non-manifold edges and zero-area faces before they break a bake | mesh queries |
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
- 2026-10-10 — Native project files use the `.rhy` extension (§8).
- 2026-10-10 — Name chosen: **Aventail 3D** (replaces working title "Facet"). Crates renamed to generic names; product name kept in `branding.rs` + `RENAMING.md` so a future rename is quick.
- 2026-10-10 — Project license: GPL-3.0-or-later, so it stays free. No outside code contributions for now (or a CLA if that changes), keeping commercial options open. Repo: github.com/Rhyslos/Aventail-3D (public).
- 2026-10-10 — Rule to flag dependencies and companion features before building was tried and dropped, as it would slow work down. §12 kept as an unscheduled ideas list.
- 2026-10-10 — Same workflow as Halberd: one milestone per branch and pull request, never push to `main`, two sign-offs, CI on three platforms, decision records ([0001](docs/decisions/0001-record-decisions.md)). Crates split into 9 layered crates ([0003](docs/decisions/0003-layered-crates.md)).
- 2026-10-10 — Window built with eframe and egui_dock, palette icons drawn in code, unbuilt commands shown greyed out with their phase ([0008](docs/decisions/0008-window-and-panels.md)).
