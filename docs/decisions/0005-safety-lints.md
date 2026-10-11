# 0005. No unsafe, no panics in library code

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

The app opens files made by other people, and a crash loses the user's work. A memory bug can also be a security hole.

## Options

1. **Guidelines only:** rely on review to catch problems.
2. **Compiler-enforced lints across the workspace:** some convenience lost, but violations fail the build.

## Decision

The workspace `Cargo.toml` sets, for every crate:

- `unsafe_code = "forbid"`: no unsafe Rust. An exception needs its own decision record. *(Since [0026](0026-windows-keyboard-focus.md): `deny`, so a module with its own decision record can allow it; the first is the Windows keyboard-focus fix.)* (Vendored C++ libraries such as xatlas will need a small binding layer; that gets its own record when it arrives.)
- `missing_docs = "deny"`: every public item is documented.
- Clippy denies `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` and `dbg!` outside tests (`clippy.toml` exempts tests).

## Consequences

- Errors must be handled and explained, which makes code a little longer but keeps the app running when something goes wrong.
- Tests may still unwrap, so test code stays short and fails loudly.
