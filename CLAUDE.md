# Working on Aventail 3D

Read `DESIGN.md` before starting any task. It is the source of truth for scope, architecture and the roadmap.

## How we work
- Work **one roadmap task per PR**, in phase order, unless the user asks otherwise.
- The app must build and launch after every PR. No half-finished features on `main`; put unfinished work behind a feature flag or in a branch.
- The user tests the *feel* of the UI and viewport. When a change affects how something looks or behaves, say in the PR description exactly what to try and what to look for.
- When a decision is made or changed, add a line to the decision log in `DESIGN.md` (§13).

## Documentation (required)
- Any PR that adds or changes user-facing behaviour **updates `docs/manual/` in the same PR**, following the page shape in DESIGN §11. A PR without its docs update is not done.
- Add one plain-language line to `docs/CHANGELOG.md` per user-facing change.
- The manual describes only what exists in `main`. Write for someone who knows roughly what 3D modelling is but is rusty: explain *why* as well as *how*, and link terms to `docs/manual/concepts.md` (add new terms there when needed).

## Architecture rules
- `mesh` and `scene` must not depend on `wgpu`, `egui` or any UI/render crate.
- Tools never mutate meshes directly. They produce `Operation`s and `Command`s (DESIGN §4.2–4.4). This keeps undo and the future Edit Poly stack working.
- Element IDs are stable and generated deterministically by operations.
- Every mesh operation gets unit tests, including degenerate cases (n-gons, zero-length edges, open boundaries, non-manifold input).
- Prefer clear code over clever code; this project will be maintained across many sessions.

## The app's name (keep renaming cheap)
- Never hardcode the product name in code. Use the constants in `crates/app/src/branding.rs` (`APP_NAME`, `APP_SHORT_NAME`, `APP_ID`, `ORG_NAME`, `PROJECT_EXTENSION`), including window titles, settings paths, About box and installer metadata.
- Crate and module names stay generic (`mesh`, `scene`, `uv`, ...), never named after the product.
- In docs, write the name only where it reads naturally; prefer "the app" elsewhere.
- Any new place the name must appear outside `branding.rs` (Cargo metadata, installer config, icons, README) gets added to `RENAMING.md` in the same PR.

## Licensing (strict)
- The project is GPL-3.0-or-later. Every crate's `Cargo.toml` sets `license = "GPL-3.0-or-later"`.
- Dependencies must still be permissive (this keeps future commercial options open): MIT, Apache-2.0, BSD, Zlib, ISC, Unicode, BSL-1.0; OFL for fonts.
- No GPL/LGPL/AGPL, no non-commercial-only licenses, no Autodesk FBX SDK.
- Check the license before adding a crate, and keep `cargo-deny` passing.
- Never add assets copied from Cinema 4D, 3ds Max or other proprietary software (icons, images, names). Recreate layout and behaviour only.

## Checks before opening a PR
- Manual and changelog updated if anything user-facing changed.
```
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```
