# The rules in force

Every rule that applies to work on this repository, one line each, with where it comes from. The sources stay the authority: this page is a digest, so when a source changes, change its line here in the same pull request.

Abbreviations: **CLAUDE** is [CLAUDE.md](../CLAUDE.md) in the repository; **workspace CLAUDE** is the general `CLAUDE.md` in the folder above the repository (not in git); **CR n** is ground rule n in [CONTRIBUTING.md](../CONTRIBUTING.md#ground-rules); **0028** and the like are [decision records](decisions/README.md).

## When sources disagree

- A newer decision record replaces an older one where it says so ("Replaces part of"); records are never renumbered ([decisions README](decisions/README.md)).
- Decision records and CLAUDE beat older wording in DESIGN.md, ARCHITECTURE.md and the manual, which may lag behind; fix the lagging text when you touch it.
- Where the workspace CLAUDE differs from the repository's own CLAUDE (it names `docs/CHANGELOG.md`, a `ORG_NAME` constant and "phase order"), follow the repository's CLAUDE and the decisions: it is specific to this project and newer.
- The project lead's explicit request in a session beats all of these; record it as a decision if it changes a rule ([0001](decisions/0001-record-decisions.md)).
- Solis's rules (its limited CI, Windows only) belong to the Solis repository, not this one.

## How work is done

- Start each session by reading README.md, DESIGN.md, ARCHITECTURE.md, CONTRIBUTING.md, the READMEs of the crates you'll touch, the relevant decisions and the phase plan in `docs/plans/` (CLAUDE).
- One milestone (or one fix) per branch and pull request; never push to `main` (CLAUDE; [CONTRIBUTING](../CONTRIBUTING.md#pull-requests)). Phase 9 is the exception the project lead asked for: one branch and one pull request, a commit per milestone ([phase 9 plan](plans/phase-9.md), [0126](decisions/0126-rework-and-first-use.md)).
- Work in roadmap order: Phase 9, then Phase 10, then the rest of Phase 8, then Phase 11 ([0125](decisions/0125-modelling-before-the-rest-of-the-uvs.md), [0121](decisions/0121-rework-before-the-rest-of-phase-8.md)).
- The app must build and launch after every pull request; unfinished work stays on a branch or behind a switch (workspace CLAUDE).
- Two sign-offs before merging: the author's code tests and the project lead's functional test ([CONTRIBUTING](../CONTRIBUTING.md#testing-two-sign-offs-per-pull-request)).
- Fill in the [pull request template](../.github/pull_request_template.md): what changed, test results, documents updated, how to test, and the newcomer check for anything a user would notice.
- "How to test" covers what tests can't (feel, looks, export into Substance Painter and Godot), written for a non-programmer (CLAUDE).
- Explain decisions to the project lead in plain words (CLAUDE).
- A major choice, or an exception to a ground rule, gets a decision record and a line in DESIGN.md §13 ([0001](decisions/0001-record-decisions.md), [DESIGN.md](../DESIGN.md) §13).
- Scope, interface or roadmap changes go into DESIGN.md first ([CONTRIBUTING](../CONTRIBUTING.md#documentation-rule)).
- Each phase ends with a gate the project lead tries by hand; from Phase 4, the gate includes the heavy-mesh check, run also on request, not on every pull request, its numbers in the pull request ([0033](decisions/0033-heavy-mesh-safeguard.md), CLAUDE, DESIGN.md §4.8).
- Commit messages: a short summary in the imperative, details after; AI-written commits carry a `Co-Authored-By` line ([CONTRIBUTING](../CONTRIBUTING.md#ai-assistance)).
- No code contributions from others for now; ideas and bug reports are welcome ([0004](decisions/0004-gpl-license.md), [CONTRIBUTING](../CONTRIBUTING.md#who-can-contribute)).

## Code and architecture

- One crate, one job; propose a new crate before adding one (CR 1).
- Dependencies point down only (mesh → core → services → front end → app); `tools/layer-check` enforces it (CR 2, [0003](decisions/0003-layered-crates.md), [ARCHITECTURE.md](../ARCHITECTURE.md)).
- `mesh` and `scene` never depend on `wgpu`, `egui` or any interface or rendering crate (workspace CLAUDE).
- Small units: files under about 500 lines, functions under about 60 (CR 3). Narrow public surface (CR 4). Reuse before writing (CR 5).
- Read a crate's README and tests before changing it (CR 11). Change the smallest thing that works; refactors get their own pull request (CR 12). Prefer clear code to clever code (workspace CLAUDE).
- No `unsafe` without a decision record; the only exception is `app/src/platform.rs` (CR 6, [0005](decisions/0005-safety-lints.md), [0026](decisions/0026-windows-keyboard-focus.md)).
- No `unwrap`, `expect`, `panic!`, `todo!` or `dbg!` outside tests; every public item documented (CR 7, [0005](decisions/0005-safety-lints.md)).
- Files from outside are hostile: check sizes and counts before allocating, within the limits rulebook, and add a fuzz test (CR 8, [0045](decisions/0045-limits-for-files.md)).
- Only the interface thread changes the scene, through commands; background jobs send messages (CR 9).
- Never lose work: save to a temporary file, check it opens, then swap; every edit undoable; base settings stay editable (CR 10, [0041](decisions/0041-saves-that-always-reopen.md)).
- Tools never change meshes or the scene directly: they produce operations and commands (CLAUDE, [0006](decisions/0006-base-plus-operation-stack.md), DESIGN.md §4.2–4.4).
- Element IDs are stable and made deterministically by operations ([0090](decisions/0090-stable-element-names.md), workspace CLAUDE).
- Automatic tools show all their settings and give the same result for the same model and settings, never a random one ([0110](decisions/0110-controllable-automation.md)).
- Commands are listed once in `ui::commands`; menus, the tool menu, the palette, keys and command search read from that list ([0037](decisions/0037-tool-menu-and-command-list.md)).
- Drags follow the pull: measured from what they change; "right for more" only as a fallback (CR 16, [0039](decisions/0039-drags-follow-the-pull.md)).
- Z is up everywhere; only `fileio` converts on export ([0012](decisions/0012-z-up-axes.md)).
- One development file format (CBOR) until the alpha: new settings get a standard value instead of a new version; older files still open. This replaces DESIGN.md §8's "every change bumps the version" for now ([0126](decisions/0126-rework-and-first-use.md)). A feature an older app would drop without a word on saving gets a new version instead (booleans: version 40, [0135](decisions/0135-live-booleans.md)).
- No "arrives later" placeholders on screen, and menu paths in tooltips must exist; `ui/src/text_checks.rs` tests both ([0126](decisions/0126-rework-and-first-use.md)).
- Exported files hold no usernames or local paths; no telemetry ([ARCHITECTURE.md](../ARCHITECTURE.md#security)).

## Tests

- Every batch gets tests: normal, edge, failure, regression, round trip, fuzz and benchmarks as relevant (CLAUDE, [CONTRIBUTING](../CONTRIBUTING.md#testing-two-sign-offs-per-pull-request)).
- A bug fix adds a test that reproduces the bug; a feature adds tests for its behaviour (CR 13).
- Every mesh operation is tested on degenerate input too: n-gons, zero-length edges, open borders, non-manifold shapes (workspace CLAUDE).
- Anything that reads a file format gets a round trip and a fuzz target (CR 8, CONTRIBUTING).
- Run the whole suite and report the results in the pull request (CLAUDE).
- Before a pull request: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check` (workspace CLAUDE, [CONTRIBUTING](../CONTRIBUTING.md#setting-up)); GPU tests with `REQUIRE_GPU=1` and the window smoke test where possible.
- If rustup can't download the pinned toolchain, use `RUSTUP_TOOLCHAIN=stable` when the versions match (CLAUDE).

## Documentation

- A change isn't done until its documents are: code comments, crate READMEs, ARCHITECTURE.md, CONTRIBUTING.md, DESIGN.md, decisions, manual, CHANGELOG.md, README tracker and history, RENAMING.md, and this page and [SPEC.md](SPEC.md) as the table says (CR 14, [CONTRIBUTING](../CONTRIBUTING.md#documentation-rule)).
- The user manual is required for anything a user would notice, in the same pull request, following [the template](manual/_template.md) and DESIGN.md §11's page shape (CLAUDE).
- The manual describes only what exists in `main`; plans live in DESIGN.md (DESIGN.md §11).
- Write for someone rusty at 3D: explain why as well as how, and link terms to [concepts](manual/concepts.md), adding new ones there (CLAUDE).
- One plain-language line in the root `CHANGELOG.md` per user-facing change; CI fails a code change without one (DESIGN.md §11, `ci.yml`).
- Update the README progress tracker and history when a milestone completes (CLAUDE).
- Use the app's own names (Attributes panel, Objects panel, Paint select, Snap to), one meaning per label, and "pieces", not "islands" ([0046](decisions/0046-our-own-names.md), [0126](decisions/0126-rework-and-first-use.md)).

## Naming and renaming

- Never hardcode the app's name: use `crates/app/src/branding.rs` (`APP_NAME`, `APP_SHORT_NAME`, `APP_ID`, `PROJECT_EXTENSION`, `MANUAL_URL`) (CR 15, CLAUDE).
- Crate and module names stay generic (`mesh`, `scene`, `uv` and so on) ([0003](decisions/0003-layered-crates.md)).
- Any new place the name must appear gets a line in [RENAMING.md](../RENAMING.md) in the same pull request (CLAUDE).
- In documents, write "the app" except where the name reads naturally (workspace CLAUDE).
- The app never shows another program's name in its commands, schemes or settings (DESIGN.md §5.1).

## Licensing and assets

- The app is GPL-3.0-or-later; every crate's `Cargo.toml` says so ([0004](decisions/0004-gpl-license.md), workspace CLAUDE).
- Dependencies stay permissive: MIT, Apache-2.0, BSD, Zlib, ISC, Unicode, BSL-1.0, OFL for fonts ([0004](decisions/0004-gpl-license.md), DESIGN.md §9, [deny.toml](../deny.toml)).
- Never GPL, LGPL, AGPL, non-commercial-only licences or the Autodesk FBX SDK (DESIGN.md §9).
- Check a crate's licence before adding it, and keep `cargo deny` passing (workspace CLAUDE).
- Fonts, icons and images built into the app need their licence text in `licenses/`, listed in [licenses/README.md](../licenses/README.md) ([CONTRIBUTING](../CONTRIBUTING.md#third-party-material)).
- Copy behaviour, never assets: no icons, images, names or code from Cinema 4D, 3ds Max or other proprietary software (CLAUDE, [CONTRIBUTING](../CONTRIBUTING.md#licensing), DESIGN.md §5.2). File formats come from public specifications.

## Controls and keys

- Only the tool keys come from Halberd: Q select, W move, S scale, R rotate, T all gizmos, B new shape ([0019](decisions/0019-only-the-tool-keys-come-from-halberd.md)).
- Ask the project lead before adding keys or mouse actions: a recommended choice (common in 3D programs, no clashes), alternatives, and anything easy to confuse explained ([0028](decisions/0028-ask-about-controls.md)).
- If the project lead can't answer, use the recommended choices and say so at the top of the pull request ([0028](decisions/0028-ask-about-controls.md)).
- Keys already built stay unless the project lead asks; further key changes wait for the key audit document's ticks ([0028](decisions/0028-ask-about-controls.md), [0126](decisions/0126-rework-and-first-use.md)).
- Ctrl adds to the selection and Shift removes, as standard; fixed (DESIGN.md §10, Phase 9).
- A tool with a count gets Ctrl + wheel while in use: add it to `Tool::has_wheel_count` and `Workbench::wheel_count`, and list it on its manual page ([0048](decisions/0048-ctrl-wheel-changes-a-tools-count.md)).
- Viewport navigation keeps its own schemes ([0015](decisions/0015-navigation-schemes.md)).
- A new command can be found: in command search and on a manual page (the [pull request template](../.github/pull_request_template.md)'s newcomer check).

## Builds and CI

- `ci.yml` runs on every pull request and every change to `main`: formatting, clippy, tests on Windows, Linux and macOS, API docs, the window and GPU test, the glTF Validator and Godot import of exported files, `cargo deny`, release builds and the changelog check. All must be green to merge ([ARCHITECTURE.md](../ARCHITECTURE.md#checks), [CONTRIBUTING](../CONTRIBUTING.md#pull-requests)).
- `heavy.yml` runs the heavy-mesh check every night and by hand, never on pull requests ([ARCHITECTURE.md](../ARCHITECTURE.md#checks)).
- `docs.yml` publishes the API reference when `main` changes.
- Each pull request's builds are kept as artifacts for the project lead to try (pull request template).
