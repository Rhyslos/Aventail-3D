# 0003. Layered single-purpose crates with generic names

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

Large modellers tend to tangle: UI code edits meshes directly, file readers know about rendering, and every change risks breaking something unrelated. The app's name may also change later, and renaming should not ripple through the code.

## Options

1. **One big crate with modules:** simple to start, but nothing stops modules from reaching into each other.
2. **Many crates in fixed layers:** more files, but the build system itself refuses forbidden dependencies.

## Decision

Nine crates in five layers (0 geometry, 1 core, 2 services, 3 front end, 4 app). A crate may only depend on its own layer or below. Each crate's README states its purpose and what it must never do. The `layer-check` tool's tests enforce the layer rule, the workspace lints and the README sections on every change.

Crate names are generic (`mesh`, `scene`, `uv`, `config`, `fileio`, `render`, `tools`, `ui`, `app`), never named after the product; the app's name lives only in `app/src/branding.rs`.

## Consequences

- Each crate can be tested and replaced on its own; `mesh` and `uv` could even be reused by other tools.
- Adding a feature means deciding which crate owns it, which is sometimes slower but keeps responsibilities clear.
- Renaming the app touches `branding.rs` and the places listed in RENAMING.md, not the crate structure.
- Generic names could clash with a future outside library of the same name; if that happens, the outside one is renamed in `Cargo.toml` (Cargo supports this).
