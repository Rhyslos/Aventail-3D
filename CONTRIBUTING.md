# Contributing to Aventail 3D

Thanks for your interest. Aventail 3D is a hobby project with high standards for stability and feel; these rules keep it that way.

## Who can contribute

**Ideas, bug reports and feedback are very welcome** through Issues.

**Code contributions are not accepted at the moment.** Keeping the copyright in one place keeps every future option open for the project (see [decision 0004](docs/decisions/0004-gpl-license.md)). If that changes, contributors will be asked to sign a Contributor License Agreement first, and this section will say so.

The rest of this guide describes how the project itself is developed.

## Setting up

1. Install Rust with [rustup](https://rustup.rs). The exact Rust version is pinned in `rust-toolchain.toml` and installs automatically.
2. Clone the repository and run:

```
cargo run -p app                # build and start the app
cargo test --workspace          # run all tests
cargo clippy --workspace --all-targets -- -D warnings   # lint
cargo fmt --all                 # format
cargo doc --workspace --no-deps --open                  # read the API docs
```

Optional, matching CI: `cargo install cargo-deny` and run `cargo deny check`.

GPU rendering tests (`cargo test -p render --test gpu`) draw real frames and check their pixels. Without a GPU they skip with a note; set `REQUIRE_GPU=1` to make a missing GPU a failure instead, as CI does.

The window smoke test (Linux only) opens the real window on a virtual screen, as CI does: it checks the 3D view (background and the test cube), picks a mode with a click, quits with File > Quit (answering the unsaved-changes question), checks the panel layout was saved, and opens a project given on the command line. Pass the app's name (`APP_NAME` in `crates/app/src/branding.rs`) as the window title:

```
sudo apt-get install xvfb xdotool imagemagick mesa-vulkan-drivers libxkbcommon-x11-0
cargo build -p app
xvfb-run -s "-screen 0 1440x900x24" tools/smoke/window-smoke.sh target/debug/aventail "Aventail 3D" smoke-output
```

## Third-party material

Anything embedded in the program itself (fonts, icons, images) needs its license text in `licenses/`, listed in `licenses/README.md`; that folder ships with every build. Code libraries are covered by `cargo deny`, and must use a permissive license even though the app itself is GPL.

Never add material copied from Cinema 4D, 3ds Max or any other proprietary software. Recreating layout and behaviour is fine; copying icons, images, names or code is not.

## Ground rules

A pull request that breaks one of these is sent back, however good the feature.

**Structure**

1. **One crate, one job.** New code goes in the crate whose job it is (see each crate's README). If none fits, propose a new crate first.
2. **Dependencies point down only.** The mesh knows nothing of the scene; the core knows nothing of the GPU or UI; tools and the UI change the scene only through commands. `tools/layer-check` enforces this.
3. **Small units.** As a guide: files under about 500 lines, functions under about 60.
4. **Narrow public surface.** Expose as little as possible from each crate.
5. **Reuse before writing.** Check whether a crate already does it.

**Safety**

6. **No `unsafe` code** without a decision record. The workspace denies it; the only exception so far is the Windows keyboard-focus fix in `app/src/platform.rs` ([decision 0026](docs/decisions/0026-windows-keyboard-focus.md)), which allows it on that one module.
7. **No crashes on bad input.** Library code never uses `unwrap`, `expect` or `panic!` outside tests (enforced by clippy). Return errors that explain the problem in plain words.
8. **Untrusted files are hostile.** Every reader checks sizes and counts before allocating, and gets a fuzz test.
9. **One owner for the scene.** Only the interface thread changes it, through commands; background jobs send messages.
10. **Never lose work.** Saving writes to a temporary file and swaps it in; edits are undoable; base object settings stay editable.

**Working with existing code**

11. **Read before changing:** the crate's README and tests first.
12. **Change the smallest thing that works.** Refactors get their own pull request.
13. **Tests move with the code.** Bug fixes add a test that reproduces the bug; features add tests for their behaviour.
14. **Documentation changes with the code.** See below.
15. **The app's name lives in `branding.rs`.** Never hardcode it; see `RENAMING.md`.

## Documentation rule

A change is not done until its documentation is.

| Document | Update when |
| --- | --- |
| Code comments (`///` and `//!`) | The item changes. Missing docs fail the build. |
| Crate `README.md` | The crate's purpose or limits change |
| `ARCHITECTURE.md` | A crate is added, removed or moved, or a data flow changes |
| `CONTRIBUTING.md` | Tooling or rules change |
| `DESIGN.md` | Scope, the UI spec or the roadmap changes |
| `docs/decisions/` | A major choice is made or reversed |
| User manual (`docs/manual/`) | Anything a user would notice. Follow `docs/manual/_template.md`. |
| `CHANGELOG.md` | Any change a user would notice |
| README progress tracker | A milestone or gate is completed |
| `RENAMING.md` | The app's name appears somewhere new |

## Testing: two sign-offs per pull request

1. **Code tests** by the author: normal cases, edge cases, failure cases (bad input gives an error, never a crash), a regression test for every bug fixed, round trips for anything that reads and writes a format, fuzz targets for new file readers, benchmarks for code on a performance path. The heavy-mesh check (`tools/heavy/heavy-check.sh`, a 3-million-polygon object, DESIGN.md §4.8) is not run on every pull request: it runs at the end of each phase as part of the gate, and whenever the project lead asks ([decision 0033](docs/decisions/0033-heavy-mesh-safeguard.md)). Run the full suite and report the results in the pull request.
2. **Functional test** by the project lead: the "how to test" list in the pull request, tried in the app (and in Substance Painter or Godot when export is involved).

Both must pass before merging.

## Pull requests

- One milestone or one fix per pull request, on its own branch (for example `phase1/viewport-camera`).
- Fill in the pull request template: what changed, test results, docs updated, how to test.
- All CI checks must be green.
- Commit messages: a short summary line in the imperative ("Add extrude tool"), then details if needed.

## AI assistance

Much of the code is written with AI assistance (Claude by Anthropic), and commits say so with a `Co-Authored-By` line. AI-written changes follow exactly the same rules and reviews as anyone else's.

## Licensing

The project is licensed under the GNU General Public License v3.0 or later. Write your own code: nothing copied from proprietary 3D software. File formats are implemented from public specifications.
