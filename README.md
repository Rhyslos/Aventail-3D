# Aventail 3D

**A calm, tool-rich 3D modeller for game assets.**

Aventail 3D is a free, open-source modelling and UV unwrapping tool for **hard-surface game assets**, written in Rust. Model it, unwrap it, and export it to Substance Painter and Godot, all in one app.

> **Status: pre-alpha.** The window opens with its calm, Cinema 4D-style layout and a real 3D view you can orbit, pan and zoom around, from four views at once if you like. You can create primitives (cube, plane, cylinder, sphere, disc, tube, cone) and groups, organise them in the Objects panel, and undo every change. Press B for a list of shapes, and select one to change its size and segments in the Attributes panel, or press Ctrl + E to make it a plain, editable mesh. Press 1, 2 or 3 to see and select a selected shape's points, edges or polygons, and W, R or S to move, rotate or scale what's selected; change a shape's settings afterwards and your edits follow. Save your work as a project and open it again later. Press E anywhere for a menu of the tools for what you're working on, D or I to extrude or inset polygons, V to bevel edges, F to cut a new loop of edges around a shape, and X to cut along a line you draw. Heavy shapes of millions of polygons stay quick to work with, and backup copies saved in the background bring your work back after a crash. More modelling tools follow through Phase 4. Follow the progress tracker below.

Created by **Rhyslos**, built with AI assistance (Claude by Anthropic). Part of the **Rhylib** family of tools, alongside [Halberd](https://github.com/Rhyslos/halberd).

## Why

Aventail 3D is built for people who loved the modelling workflows of classic 3D packages like Cinema 4D and 3ds Max, but no longer have access to them and never got along with the alternatives. It takes inspiration from how those tools *felt* to use: a clean, calm viewport, deep modelling tools and a powerful UV unwrapper. It starts close to that feel and grows into its own thing.

What it's for:

- **Hard-surface modelling** for game assets: props, weapons, vehicles, environment pieces
- **UV unwrapping**: seams, unwrapping, packing, texel density
- **Clean export** to glTF and OBJ for Godot and Substance Painter
- **Never losing work**: effectively unlimited undo, and base object settings that stay editable after you've modelled on top of them

Rendering, sculpting and texturing are left to the tools that already do them well. Rigging and animation are planned for much later.

## Progress tracker

Aventail 3D is built in phases. Each ends with a gate the project lead checks before the next phase starts. This tracker is updated by the pull request that finishes each milestone. The full plan is in [DESIGN.md](DESIGN.md).

### Phase 0 · Foundations (v0.0, internal) — 7 of 7

`█████████████` 100%

- [x] Repository set up: license, README, security policy, contribution rules
- [x] Automatic builds for Windows, Linux and macOS on every change
- [x] Quality checks in CI: formatting, lint, tests, license and vulnerability scans
- [x] Documentation setup: architecture, contributing guide, changelog, decision records, pull request template, API reference, user manual
- [x] Workspace split into crates
- [x] Window with the C4D-style layout: viewport, Attributes panel, Objects panel, palettes, status bar, all dockable
- [x] Theme: calm dark colours, font and spacing

**Gate:** the app launches into the C4D-style layout with movable, dockable panels. *Passed 2026-10-10.*

### Phase 1 · Viewport and navigation — 5 of 5

`█████████████` 100%

<details>
<summary>Milestones</summary>

- [x] 3D viewport: soft gradient background, fading grid, a test cube
- [x] Navigation: orbit, pan and zoom with the mouse alone (or another choice of mouse controls)
- [x] Orbit and zoom around the point under the mouse cursor
- [x] Frame selected and frame all
- [x] Perspective, top, front and right views, and the 4-view layout

</details>

**Gate:** navigating feels smooth and calm to the project lead. *Passed 2026-10-10.*

### Phase 2 · Objects, history and saving — 6 of 6

`█████████████` 100%

<details>
<summary>Milestones</summary>

- [x] Objects panel: create, rename, reparent, hide and delete objects (with undo and redo)
- [x] Primitives: cube, plane, cylinder, sphere, disc, tube, cone
- [x] Primitive settings in the Attributes panel
- [x] Object stack foundation: base plus recorded operations
- [x] Undo and redo limited by memory, not steps
- [x] Save and open `.avtl` projects

</details>

**Gate:** a scene of primitives can be built, edited, undone freely, saved and reopened. ✅ *Passed 2026-10-11: tested by the project lead, "it all felt great".*

### Phase 3 · Editable meshes and selection — 5 of 5

`█████████████` 100%

<details>
<summary>Milestones</summary>

- [x] Mesh data structure, fully tested
- [x] Point, edge and polygon modes
- [x] Selection: click, rectangle, lasso, paint select, loop, ring, grow and shrink
- [x] Move, rotate and scale gizmos with axis constraints, snapping and typed values
- [x] Edits recorded on top of the base, so base settings stay editable

</details>

**Gate:** points, edges and polygons can be selected and moved precisely, and a cube's segments can still be changed afterwards. ✅ *Passed 2026-10-12: tested by the project lead.*

### Phase 4 · Core modelling tools — 9 of 15

`████████░░░░░` 60%

<details>
<summary>Milestones</summary>

- [x] Heavy meshes: stays usable with a 2–3 million polygon object, checked at every phase gate
- [x] Autosave and crash recovery: backup copies in the background, offered back after a crash
- [x] Tool menu (E): every tool for the current mode at the mouse, radial or list, from one command list
- [x] Extrude and inset (D, I): drag anywhere to push polygons out or in, inset them, or grow border edges; neighbours together or each on its own
- [x] Bevel (V): edges and points, with width, segments and profile (round, flat or hollow); drag away from the edges
- [x] Foundations: saves that always reopen, fast edits however long the history, extra values per point, edge, polygon and corner (ready for UVs), edits that hold when earlier ones change, limits for damaged files; and our own names (Attributes panel, Objects panel, Paint select)
- [x] Loop cut (F): point at an edge, click or drag; Ctrl + wheel for a tool's count; Z frames the selection
- [x] Slit (X): cut new edges along a line of points over the shape, visible only or through
- [x] Extrude with a bend: pipe bends that keep their size, with an angle, direction, bend radius and segments
- [ ] Connect, Bridge (C) and Weld
- [ ] Clean-up: dissolve, delete, fill hole and flip normals, with select flipped, fix facing and select by facing
- [ ] Hard and soft edges, with normals display
- [ ] Symmetry
- [ ] Basics: hide and isolate, slide points and edges, duplicate and instances, split polygons off and join objects
- [ ] Keyboard shortcut viewer: every key in one place

</details>

**Gate:** a simple hard-surface prop (a crate, a sci-fi panel) can be modelled from start to finish, and the heavy-mesh check passes. From here on, every gate includes the heavy-mesh check.

### Phase 5 · Export pipeline — 0 of 10

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] Pivot editing and scene units
- [ ] OBJ export and import
- [ ] glTF export
- [ ] Consistent triangulation and MikkTSpace tangents
- [ ] Simple material slots: polygons assigned to a named material with a colour, so engines get their parts
- [ ] Export presets for Godot and Substance Painter
- [ ] Engine export paths: save straight into a Godot, Unity or Unreal Engine project, exported again on every save
- [ ] Bake readiness check: what would spoil a bake, in plain words, with one-click fixes
- [ ] Kit export: each piece as its own file, and Godot collision meshes
- [ ] Splash screen while the app starts

</details>

**Gate:** a prop exports into Godot and Substance Painter and looks right (shape, hard edges, scale) with no fixes in other software. The full bake round trip comes with UVs (Phase 8).

### Phase 6 · Workflow tools — 0 of 5

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] Stable element names: edits keep their exact points, edges and polygons even when an earlier edit renumbers them
- [ ] Edit Layers: a stack of edit layers per object (inspired by 3ds Max's stack of edit modifiers), whose layers find their parts again when a layer below changes, added from a new modifiers palette between the viewport and the Attributes and Objects panels
- [ ] Edit list: every recorded edit in order, each one changeable, pinned to stay changeable, frozen to keep big models quick, or switched off
- [ ] Snap to and align tools
- [ ] Optional quad menu on right-click

</details>

**Gate:** stacking, toggling and editing earlier layers and edits feels right to the project lead.

### Phase 7 · Advanced hard-surface — 0 of 9

`░░░░░░░░░░░░░` 0%

Friends join as testers around here, once there are enough tools to build something.

<details>
<summary>Milestones</summary>

- [ ] Booleans: union, subtract, intersect
- [ ] Weighted normals
- [ ] Improved bevel: rounded corners where bevels meet, and bevels that never overlap
- [ ] Array and mirror generators
- [ ] Shell: give a surface thickness
- [ ] Curves and splines: sweep and lathe, for pipes, cables and railings
- [ ] Smoothing modifier (subdivision), with creases
- [ ] Game bevel for mid-poly models: bevel, weighted normals, hard edges and seams in one step
- [ ] Modular kit helpers: grid-sized pieces and pivot presets

</details>

### Phase 8 · UVs — 0 of 10

`░░░░░░░░░░░░░` 0%

UVs come after modelling, as one big phase, aiming to be the best UV tools around.

<details>
<summary>Milestones</summary>

- [ ] UV mode, the UV Editor panel and the UV Edit layout
- [ ] Seams: marking, painting, from hard edges, and suggested seams in hidden places
- [ ] Live unwrap, relax and straighten, with hard-surface strips straightened automatically
- [ ] Texture size and texel density as scene settings
- [ ] Packing: real-pixel padding, nesting, groups, texture sets, stacking identical parts
- [ ] Checker texture, and a stretch heatmap with explained fixes
- [ ] UV editing directly on the model
- [ ] UVs that survive modelling: change the model, keep the UVs
- [ ] Bake readiness for UVs: seams, overlaps, padding, texel density
- [ ] Trim sheets

</details>

**Gate:** a prop is unwrapped and packed in Aventail 3D, with clean bakes in Substance Painter, and changing the model afterwards keeps its UVs.

### Further ahead

In this order, each planned in detail when its time comes:

1. **Texturing and a material builder** (to be planned): a built-in texture painter shaped by game work, so no Substance Painter subscription is needed. Hand-painted, stylised art first (painting in the 3D and UV views at once, palettes, a light-and-shadow helper, a toon preview and outlines for cartoon looks), realism after (PBR layers and smart masks for wear and dirt). Then a node-based material builder.
2. **Scripting in Lua** (Phase 9): people add their own tools, as undoable commands.
3. **Rigging, skinning and animation** (Phase 10): for game developers, first and third person, exporting cleanly to engines.

The official release comes when the app is finished, after texturing. Also wanted, not yet scheduled: glTF import, screenshots and turntable GIFs from the viewport, interface size scaling, a first-start welcome with sample projects, and custom hotkeys (added along the way).

### History

Newest first. One line per finished milestone or passed gate.

| Date | Event |
| --- | --- |
| 2026-10-14 | Pipe bends: an extrude turns a corner with Angle, Direction, Bend radius and Segments, as a smooth elbow or a mitre joint, keeping the pipe's full width; project files at format version 8 |
| 2026-10-14 | Slit (X): cut along a line of points clicked over the shape, editable until finished, visible only or through; project files at format version 7. Pipe bends moved into Phase 4 |
| 2026-10-14 | Loop cut (F): a new loop of edges around a shape, shown before you click, slid by dragging; Ctrl + wheel changes a tool's count (Bevel's segments, Loop cut's cuts); Z frames the selection and Shift + X/Y/Z lock axes; project files at format version 6 |
| 2026-10-14 | Our own names: the Attributes panel, the Objects panel and Paint select |
| 2026-10-14 | Foundations, hostile files: damaged or booby-trapped projects are refused with a message instead of crashing or freezing the app; edits that would make a shape impossibly large are skipped; recovery can't get stuck. Foundations complete |
| 2026-10-14 | Foundations, room to grow (part 2): edits keep their ids and switches and hold on to their points when earlier edits change; project files at format version 5 |
| 2026-10-14 | Foundations, room to grow (part 1): shapes can hold extra values per point, edge, polygon and corner, and the tools carry them along (ready for UVs and materials); polygons can be deleted |
| 2026-10-13 | Foundations, fast edits: drags and Last edit changes only work out the one edit, however long the history; bevel and finding edits again fast on big shapes; bevel Profile down to -100% (hollow) |
| 2026-10-13 | Foundations, never lose a project: every save and backup copy is checked to open again before it replaces anything |
| 2026-10-13 | Bevel (V): chamfer or round edges and cut corners, pulling away from the edges; Segments and Profile in the Attributes panel |
| 2026-10-13 | Extrude and inset (D and I): drag anywhere in the view; Keep together; the last one can be changed in the Attributes panel; border edges extrude too |
| 2026-10-12 | The tool menu (E): object or modelling tools at the mouse as a ring or a list; one command list behind every menu |
| 2026-10-12 | Autosave: backup copies of the scene in the background (every 5 minutes, newest 5 kept, both changeable in Settings), offered back after a crash |
| 2026-10-12 | Heavy meshes: a 3-million-polygon shape stays quick to select, hover and edit; Help → Add a heavy test shape to try it |
| 2026-10-12 | Phase 3 gate passed: tested by the project lead |
| 2026-10-11 | Edits find their points again when a shape's settings change, and the Scale gizmo scales along two axes at once. Phase 3 complete |
| 2026-10-11 | Move, rotate and scale (W, R, S, T): gizmos for shapes and for points, edges and polygons, snapping, axis locks, typed values in the Coordinates panel; project files at format version 2 |
| 2026-10-11 | Selecting: click shapes in the viewport; select points, edges and polygons by click, box, lasso or brush (through the shape, or only what's visible), loops, rings, grow and shrink, all with undo |
| 2026-10-11 | Points, Edges and Polygons modes (keys 1, 2, 3): see a selected shape's points and edges, and the one under the mouse lights up |
| 2026-10-11 | Phase 2 gate passed: tested by the project lead, "it all felt great" |
| 2026-10-11 | Phase 3 begins: every mesh's connections (loops, rings, borders, pieces) worked out, and a Mesh section with triangle count and a health check for every shape |
| 2026-10-11 | Saving and opening `.avtl` projects: File menu, safe saving with a backup, a question before unsaved changes are lost. Phase 2 complete |
| 2026-10-11 | Settings window (Edit → Settings…, Ctrl + ,): accent, units, navigation, grid and undo memory in one place |
| 2026-10-11 | Undo limited by memory instead of steps: 2 GB as standard, set in Edit → Undo memory, oldest steps forgotten first |
| 2026-10-11 | Object stack foundation: shapes keep recorded edits apart from their settings, worked out layer by layer and cached; Make editable (then C, now Ctrl + E) bakes a shape into a plain mesh |
| 2026-10-10 | Attributes panel: a shape's settings change as you drag or type, one undo step per drag; copy and paste settings. Also: 1 m standard sizes, step arrows, metres or centimetres, round-end rings and centre points, three grid levels, and the B shape list |
| 2026-10-10 | Primitives: cube, plane, cylinder, sphere, disc, tube and cone, standing on the ground, with hard and soft edges |
| 2026-10-10 | Objects panel: select, rename, group, nest, hide and delete objects, with undo and redo |
| 2026-10-10 | Phase 1 gate passed: tested by the project lead, "worked flawlessly and felt good" |
| 2026-10-10 | Perspective, top, front and right views and the four-view layout (F1 to F5, middle click). Phase 1 milestones complete |
| 2026-10-10 | Frame all (H) and frame selected (then F, now Z), gliding the view until everything fits |
| 2026-10-10 | Orbit and zoom around the point under the mouse, with a marker while orbiting |
| 2026-10-10 | A darker studio look from the approved mockup: horizon, finite grid, menu lines inside panels, a red accent with five others to choose from, and a choice of navigation controls |
| 2026-10-10 | Cinema 4D-style navigation (Alt + drag to orbit, pan and zoom, wheel to zoom) and a grid detail switch, with a calmer 1 m grid by default |
| 2026-10-10 | First 3D viewport: gradient, fading grid on the ground, X and Y axes, and a softly lit test cube |
| 2026-10-10 | Phase 0 gate passed: tested by the project lead. Coordinates fitted to its rows and styled as in the mockup, shape icons in blue, Z set as up |
| 2026-10-10 | Calm dark theme and the Inter font. Phase 0 milestones complete |
| 2026-10-10 | First window: the Cinema 4D-style layout with dockable panels, both palettes and a remembered layout |
| 2026-10-10 | Repository, documentation, quality checks and crate structure in place |
| 2026-10-10 | Project started: design document, roadmap, UI mockups and name |

### Snapshots

Pictures of Aventail 3D at memorable moments, newest first. Kept in [docs/screenshots](docs/screenshots).

**2026-10-10: the first primitives.** All seven shapes side by side (the selected cube outlined in the red accent): smooth round sides, crisp rims, and ends made of single faces.

![The seven primitives](docs/screenshots/2026-10-10-primitive-gallery.png)

**2026-10-10: the Objects panel.** The test cube dragged into a group (its name orange while selected), and a second group hidden with its eye.

![The Objects panel](docs/screenshots/2026-10-10-object-manager.png)

**2026-10-10: four views.** Perspective, top, front and right side by side, the flat views each looking at their own grid face on; the active view is outlined in red.

![The four-view layout](docs/screenshots/2026-10-10-four-views.png)

**2026-10-10: orbiting around the mouse.** The view turning around the spot grabbed on the cube's side, marked with a small ring.

![Orbiting around the point under the mouse](docs/screenshots/2026-10-10-orbit-around-the-mouse.png)

**2026-10-10: the studio look.** Darker panels, the viewport's own menu line, a grey sky over the ground with a horizon, a grid fading out around the cube, the green Z axis, the red accent on the active mode, and the navigation controls button in the bottom-right corner.

![The darker studio look](docs/screenshots/2026-10-10-studio-look.png)

**2026-10-10: moving around.** The view orbited with Alt + drag, over the calmer 1 m grid. The grid detail switch sits at the bottom of the left palette.

![The 3D view orbited, with the calm grid](docs/screenshots/2026-10-10-navigation.png)

**2026-10-10: the first 3D viewport.** The grid on the ground fading into the distance, the red X and blue Y axes, and a 2 m test cube in soft studio light, drawn on the GPU.

![Aventail 3D's first 3D viewport](docs/screenshots/2026-10-10-first-3d-viewport.png)

**2026-10-10: the calm theme.** The mockups' gentle greys, the orange accent on the active mode, Inter for text, and the viewport filling its panel edge to edge.

![Aventail 3D with its calm dark theme](docs/screenshots/2026-10-10-theme.png)

**2026-10-10: the first window.** The menu bar and command palette along the top, the mode palette down the left, the viewport's calm grey, Attributes and Objects on the right, Coordinates underneath. Taken by the automatic window test on a virtual screen.

![Aventail 3D's first window](docs/screenshots/2026-10-10-first-window.png)

## Building from source

You need [Rust](https://rustup.rs) installed. Then, in this folder:

```
cargo run -p app              # build and start the app
cargo test --workspace        # run every test
```

The first build downloads dependencies and takes a few minutes. Ready-made builds for Windows, Linux and macOS are attached to every pull request and change on GitHub, under the Actions tab.

## Documentation

- [User manual](docs/manual/SUMMARY.md): how to use the app. New to 3D, or rusty? Start with [Concepts](docs/manual/concepts.md).
- [DESIGN.md](DESIGN.md): what the app is, the interface plan and the full roadmap
- [ARCHITECTURE.md](ARCHITECTURE.md): how the app is put together
- [CONTRIBUTING.md](CONTRIBUTING.md): how it is built and tested, and the ground rules
- [CHANGELOG.md](CHANGELOG.md): what changed
- [docs/decisions](docs/decisions): why major choices were made
- [SECURITY.md](SECURITY.md): how to report a vulnerability

## Contributing

**Aventail 3D isn't accepting outside contributions: it is too early in development.** The design is still moving quickly, and the project is built by one person with AI assistance, so pull requests from others won't be merged.

Bug reports and ideas are welcome through [Issues](https://github.com/Rhyslos/Aventail-3D/issues). See [CONTRIBUTING.md](CONTRIBUTING.md) for how the project is developed.

## License

GNU General Public License v3.0 or later; see [LICENSE](LICENSE) and [NOTICE](NOTICE). You may use, change, share and sell the app, as long as versions you distribute stay open under the same license. Anything you make with it is yours.

Aventail 3D is not affiliated with, endorsed by, or based on code from Maxon or Autodesk. Cinema 4D is a trademark of Maxon; 3ds Max is a trademark of Autodesk.
