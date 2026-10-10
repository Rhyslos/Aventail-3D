# layer-check

A development tool, not part of the app. Its tests run with `cargo test` and fail if any crate in `crates/` breaks the architecture rules from `ARCHITECTURE.md`:

- Every crate declares its layer in `Cargo.toml` under `[package.metadata.architecture]`, as a number from 0 to 4.
- A crate only depends on the app's own crates in its own layer or a lower one.
- Every internal dependency is a real crate in `crates/`.
- Every crate uses the workspace lints (`[lints] workspace = true`), so the safety and documentation rules apply everywhere.
- Every crate has a `README.md` with `## Purpose` and `## Must never` sections.

The app's own crates are the ones listed with a `path` under `crates/` in the root `Cargo.toml`'s `[workspace.dependencies]`. Crate names are generic (no product name), so this tool needs no change when the app is renamed.

## Must never

- Ship inside the app
- Be loosened to make a failing pull request pass: fix the crate instead, or change the rule through a decision record
