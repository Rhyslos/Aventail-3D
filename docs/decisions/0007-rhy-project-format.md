# 0007. The `.rhy` project format

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

Projects hold more than a mesh exchange format can carry: object stacks, base parameters, settings and optionally undo history. They must survive crashes and open in every future version of the app.

## Options

1. **Save as glTF or OBJ:** universal, but loses stacks, parameters and history.
2. **Our own format:** full fidelity, under the project's control.

## Decision

An own format with the `.rhy` extension (after Rhylib, so it survives any rename of the app):

- A header with the magic bytes `RHY\0`, a format version and a flags field.
- The body serialized with `serde` and compressed with zstd.
- Optional sections for undo history and a small thumbnail.
- Every format change bumps the version, and the loader keeps upgrade steps for older versions.
- Saving writes a temporary file and swaps it in; the previous save is kept as `.rhy.bak`.
- The installer registers `.rhy` so double-clicking a project opens the app.

## Consequences

- Old projects always open, and a crash during saving never corrupts a project.
- Other apps cannot read `.rhy`; exchange happens through OBJ and glTF export.
- If Halberd ever saves its own projects, it should use a different extension, so double-clicking stays unambiguous.
