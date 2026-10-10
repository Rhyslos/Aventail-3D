# Changelog

Every change a user would notice, newest first, in plain words.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

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
