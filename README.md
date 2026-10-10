# Aventail 3D

**A calm, tool-rich 3D modeller for game assets.**

Aventail 3D is a free, open-source modelling and UV unwrapping tool for **hard-surface game assets**, written in Rust. Model it, unwrap it, and export it to Substance Painter and Godot, all in one app.

> **Status: pre-alpha.** The window opens with its calm, Cinema 4D-style layout, but there is nothing to model with yet. The 3D viewport is next. Follow the progress tracker below.

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
- [x] Window with the C4D-style layout: viewport, Attribute Manager, Object Manager, palettes, status bar, all dockable
- [x] Theme: calm dark colours, font and spacing

**Gate:** the app launches into the C4D-style layout with movable, dockable panels. *Passed 2026-10-10.*

### Phase 1 · Viewport and navigation — 0 of 5

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] 3D viewport: soft gradient background, fading grid, a test cube
- [ ] Cinema 4D-style navigation: Alt + mouse to orbit, pan and zoom
- [ ] Orbit and zoom around the point under the mouse cursor
- [ ] Frame selected and frame all
- [ ] Perspective, top, front and right views, and the 4-view layout

</details>

**Gate:** navigating feels smooth and calm to the project lead.

### Phase 2 · Objects, history and saving — 0 of 6

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] Object Manager: create, rename, reparent, hide and delete objects
- [ ] Primitives: cube, plane, cylinder, sphere, disc, tube, cone
- [ ] Primitive settings in the Attribute Manager
- [ ] Object stack foundation: base plus recorded operations
- [ ] Undo and redo limited by memory, not steps
- [ ] Save and open `.avtl` projects

</details>

**Gate:** a scene of primitives can be built, edited, undone freely, saved and reopened.

### Phase 3 · Editable meshes and selection — 0 of 5

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] Mesh data structure, fully tested
- [ ] Point, edge and polygon modes
- [ ] Selection: click, rectangle, lasso, live selection, loop, ring, grow and shrink
- [ ] Move, rotate and scale gizmos with axis constraints, snapping and typed values
- [ ] Edits recorded on top of the base, so base settings stay editable

</details>

**Gate:** points, edges and polygons can be selected and moved precisely, and a cube's segments can still be changed afterwards.

### Phase 4 · Core modelling tools — 0 of 7

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] Extrude and inset
- [ ] Bevel and chamfer
- [ ] Loop cut and knife
- [ ] Connect, bridge and weld
- [ ] Dissolve, delete, fill hole and flip normals
- [ ] Hard and soft edges, with normals display
- [ ] Symmetry

</details>

**Gate:** a simple hard-surface prop (a crate, a sci-fi panel) can be modelled from start to finish.

### Phase 5 · Export pipeline — 0 of 4

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] OBJ export and import
- [ ] glTF export
- [ ] Consistent triangulation and MikkTSpace tangents
- [ ] Export presets for Godot and Substance Painter

</details>

**Gate:** a prop goes through Substance Painter into Godot with no fixes in other software.

### Phase 6 · UVs — 0 of 6

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] UV mode and the UV Editor panel
- [ ] Seams, including seams from hard edges
- [ ] Unwrap, relax and straighten
- [ ] Packing with padding
- [ ] Texel density display and normalising
- [ ] Checker texture and stretch view

</details>

**Gate:** that prop is unwrapped and packed in Aventail 3D, with clean bakes in Substance Painter.

### Phase 7 · Workflow tools — 0 of 3

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] Edit Layers: a stack of edit layers per object, inspired by 3ds Max's Edit Poly stack
- [ ] Quick align and align tools
- [ ] Optional quad menu

</details>

**Gate:** stacking, toggling and editing earlier layers feels right to the project lead.

### Phase 8 · Advanced hard-surface — 0 of 4

`░░░░░░░░░░░░░` 0%

<details>
<summary>Milestones</summary>

- [ ] Booleans: union, subtract, intersect
- [ ] Weighted normals
- [ ] Improved bevel
- [ ] Array and mirror generators

</details>

Further ahead: scripting in Lua, so people can add their own tools (Phase 9), then rigging, skinning and animation (Phase 10), once modelling and UVs are mature.

### History

Newest first. One line per finished milestone or passed gate.

| Date | Event |
| --- | --- |
| 2026-10-10 | Phase 0 gate passed: tested by the project lead. Coordinates fitted to its rows, icons coloured, Z set as up |
| 2026-10-10 | Calm dark theme and the Inter font. Phase 0 milestones complete |
| 2026-10-10 | First window: the Cinema 4D-style layout with dockable panels, both palettes and a remembered layout |
| 2026-10-10 | Repository, documentation, quality checks and crate structure in place |
| 2026-10-10 | Project started: design document, roadmap, UI mockups and name |

### Snapshots

Pictures of Aventail 3D at memorable moments, newest first. Kept in [docs/screenshots](docs/screenshots).

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

Ideas, bug reports and feedback are very welcome through Issues. Code contributions aren't accepted at the moment; see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

GNU General Public License v3.0 or later; see [LICENSE](LICENSE) and [NOTICE](NOTICE). You may use, change, share and sell the app, as long as versions you distribute stay open under the same license. Anything you make with it is yours.

Aventail 3D is not affiliated with, endorsed by, or based on code from Maxon or Autodesk. Cinema 4D is a trademark of Maxon; 3ds Max is a trademark of Autodesk.
