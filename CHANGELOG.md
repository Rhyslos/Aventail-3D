# Changelog

Every change a user would notice, newest first, in plain words.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- Axis colours are now X red, Y blue, Z green (most programs use Y green and Z blue), in the viewport, the Coordinates panel and the palette's axis locks.
- The viewport looks softer: gentler light with a slight gradient across each side, a little shade where shapes meet the ground, a quieter grid and fainter edges.
- The Coordinates panel is now just tall enough for its rows, and every panel can be dragged much smaller than before.
- Palette icons follow the mockups: soft white, with the shapes you can create in blue. Commands that aren't built yet are only slightly dimmed.
- The Coordinates panel follows the mockup: coloured X, Y and Z letters, wide dark value fields, and Object (Rel) and Apply on the right.
- Z is up: X and Y lie on the ground and Z is height, as in 3ds Max and Blender. Shown in the manual; the viewport is built this way from Phase 1.
- Project files will be saved as `.avtl` (short for Aventail) instead of `.rhy`. Nothing has been saved yet, so no files are affected.

### Added

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
