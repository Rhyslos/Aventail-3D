# Notes for Claude sessions

Aventail 3D is developed largely by Claude, steered by the project lead (Rhyslos), who is not a programmer and is relearning 3D modelling along the way. Read this before any work.

## Start of every session

1. Read `README.md` (progress tracker: what is next), `DESIGN.md` (scope, UI spec, roadmap), `ARCHITECTURE.md` and `CONTRIBUTING.md`.
2. Read the README of every crate you will touch.
3. Check `docs/decisions/` before changing anything a decision covers.

## Rules that are easy to forget

- One milestone per branch and pull request. Never push to `main`.
- Follow every ground rule in `CONTRIBUTING.md`; CI enforces many of them, review enforces the rest.
- Write tests for every batch (normal, edge, failure, regression, round trip, fuzz, benchmarks as relevant), run the whole suite, and report results in the pull request.
- Update documentation in the same pull request: code comments, crate READMEs, `ARCHITECTURE.md`, `CHANGELOG.md`, the **user manual** (`docs/manual/`), and the README progress tracker and history when a milestone completes.
- The pull request's "How to test" list covers what automated tests cannot: feel, visuals, the export pipeline into Substance Painter and Godot. Write it for a non-programmer.
- Explain decisions to the project lead in plain words.
- At the end of each phase (from Phase 4) and whenever the project lead asks, run the heavy-mesh check and put its numbers in the pull request (DESIGN.md §4.8, [decision 0033](docs/decisions/0033-heavy-mesh-safeguard.md)). Not on every pull request.

## Specific to this project

- **The user manual is required.** Any change a user would notice updates `docs/manual/` in the same pull request, following the page shape in DESIGN.md §11 and `docs/manual/_template.md`. Write for someone rusty at 3D: explain *why* as well as *how*, and link terms to `docs/manual/concepts.md` (add new terms there). The manual describes only what exists in `main`.
- **Keep renaming cheap.** Never hardcode the app's name in code: use `crates/app/src/branding.rs`. Crate and module names stay generic. Any new place the name must appear (Cargo metadata, CI artifacts, installer, icons, docs) gets added to `RENAMING.md` in the same pull request.
- **Tools never change meshes or the scene directly.** They produce operations and commands (DESIGN.md §4.2–4.4, [decision 0006](docs/decisions/0006-base-plus-operation-stack.md)), which keeps undo and the future Edit Layers stack working.
- **Only the tool keys come from Halberd** ([decision 0019](docs/decisions/0019-only-the-tool-keys-come-from-halberd.md)): `Q` select, `W` move, `S` scale, `R` rotate, `T` all gizmos, `B` new shape. **Ask the project lead what the controls should be before building a feature that adds keys or mouse actions** ([decision 0028](docs/decisions/0028-ask-about-controls.md)): offer a recommended choice (common in 3D programs, no clashes) and alternatives, and explain anything easy to confuse. When they can't answer, use the recommended choices and say so at the top of the pull request. Don't base anything else on Halberd (keys or features) unless the project lead names that specific thing. Viewport navigation keeps its own schemes.
- **Dependencies stay permissive** even though the app is GPL ([decision 0004](docs/decisions/0004-gpl-license.md)).
- **Copy behaviour, never assets:** nothing from Cinema 4D, 3ds Max or other proprietary software (icons, images, names) goes into the repository.

## Local environment notes

- If rustup cannot download the pinned toolchain, run commands with `RUSTUP_TOOLCHAIN=stable` when the installed stable version matches `rust-toolchain.toml`.
