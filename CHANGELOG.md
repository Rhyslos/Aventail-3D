# Changelog

Every change a user would notice, newest first, in plain words.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- A darker, calmer look, closer to the approved mockup: darker panels with plain text tabs, small menu lines inside the panels, and a viewport with a grey sky over a slightly darker ground, a horizon, and a grid that fades out around the point you're looking at (at least 18 m around it, reaching farther as you zoom out). A green line rises from the origin to show Z, and the test cube is a mid grey.
- The viewport has its own menu line (View, Cameras, Display, Options, Panel) in place of its tab, with buttons you can drag to pan, zoom and orbit, and labels showing the view, the camera, which way the axes point, and the grid spacing.
- The active mode is now marked by a filled red square (the new accent colour) instead of orange; the opening view is a little lower, so the horizon shows.
- Axis colours are now X red, Y blue, Z green (most programs use Y green and Z blue), in the viewport, the Coordinates panel and the palette's axis locks.
- The viewport looks softer: gentler light with a slight gradient across each side, a little shade where shapes meet the ground, a quieter grid and fainter edges.
- The Coordinates panel is now just tall enough for its rows, and every panel can be dragged much smaller than before.
- Palette icons follow the mockups: soft white, with the shapes you can create in blue. Commands that aren't built yet are only slightly dimmed.
- The Coordinates panel follows the mockup: coloured X, Y and Z letters, wide dark value fields, and Object (Rel) and Apply on the right.
- Z is up: X and Y lie on the ground and Z is height, as in 3ds Max and Blender. Shown in the manual; the viewport is built this way from Phase 1.
- Project files will be saved as `.avtl` (short for Aventail) instead of `.rhy`. Nothing has been saved yet, so no files are affected.

### Added

- Navigation controls to choose from, with the button in the bottom-right corner of the viewport (or Options → Navigation controls…): **Mouse only** is now the standard (right drag orbits, middle drag pans, the wheel zooms), and Alt + mouse (as before), Mouse only with middle zooming, Middle pans with Alt orbiting, and Middle orbits can be picked instead. Alt + mouse can keep working in every scheme, and orbiting can be inverted. The status bar shows the controls in use, and the choice is remembered.
- Accent colour: red as standard, or Blue, Marine green, Plum, Olive or Petrol blue, from Window → Accent colour. Remembered between sessions. Each colour has a small swatch next to its name.
- Layout tabs on the right of the menu bar: Standard, with Modelling and UV Edit to come.
- Move around the 3D view as in Cinema 4D: hold Alt and drag with the left mouse button to orbit, the middle button to pan, the right button to zoom; the mouse wheel zooms too. The status bar shows the controls, and **View → Reset view** brings back the opening view. New manual page: Navigating the view.
- A grid detail switch at the bottom of the left palette (also **View → Fine grid lines**): the grid now shows only 1 m lines by default, for a calmer ground, and the 10 cm lines come back when you switch it on. The choice is remembered.
- The 3D viewport: a calm grey background, a grid on the ground (10 cm and 1 m lines, fading softly into the distance), the red X and blue Y axes, and a softly lit 2 m test cube. Without a usable graphics card, the viewport says why instead.
- The calm dark look from the mockups: gentle greys, a warm orange for the active mode, the Inter font, and a viewport that fills its panel edge to edge. The app stays dark whatever your system prefers.
- The app window, laid out like Cinema 4D: menu bar, command palette, mode palette, viewport, Attributes (top right), Objects (below it), Coordinates (under the viewport) and a status bar.
- Panels can be dragged by their tabs, stacked and resized; **View → Reset panel layout** brings the standard layout back. The layout and window size are remembered between sessions.
- Pick a mode (Model, Object, Points, Edges, Polygons, UV) from the left palette or the Modes menu.
- Commands that aren't built yet are listed greyed out, with the phase that brings them.
- File → Quit (Ctrl+Q) and Help → About.
- Command-line options `--version` and `--help`.
- User manual: Interface tour page.
- Project foundations: Cargo workspace with 9 single-purpose crates in five layers, with generic names so renaming the app never touches them.
- `layer-check` tool whose tests enforce the architecture rules on every change.
- Workspace-wide safety lints: no `unsafe` code, no `unwrap`/`expect`/`panic!` in library code, documentation required on every public item.
- Continuous integration: formatting, lint, tests and docs on Windows, Linux and macOS; license and vulnerability audit; downloadable builds for all three platforms; changelog check on pull requests.
- API reference published to GitHub Pages from `main`.
- Documentation: README with progress tracker, architecture overview, contributing guide, security policy, decision records, pull request and issue templates, user manual structure with a concepts page.
- Program skeleton that reports its name and version, with the app's name kept in one place (`branding.rs`).
