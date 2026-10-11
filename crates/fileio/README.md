# fileio

**Layer 2 · Services**

Everything that touches files: `.avtl` projects, OBJ and glTF import and export, the settings file.

## Purpose

Reads and writes the outside world's files: the versioned `.avtl` project format (DESIGN.md §8, decisions [0007](../../docs/decisions/0007-rhy-project-format.md) and [0010](../../docs/decisions/0010-avtl-file-extension.md)), OBJ and glTF for Substance Painter and Godot (with deterministic triangulation and MikkTSpace tangents), and the settings file described by `config`.

## Must never

- Trust a file: every reader checks sizes and counts before allocating, rejects bad input with a plain-language error, and gets a fuzz test
- Leave a half-written file: write to a temporary file, keep the previous save as a backup, then swap it in
- Write usernames or local paths into exported files
- Know about the GPU or UI

## Depends on

`scene`, `mesh`; `uv` and `config` planned. Outside libraries: `serde` and `postcard` (encoding), `zstd` (compression), `glam`.

## Status

**Projects** (`project`, decisions [0024](../../docs/decisions/0024-saving-projects.md) and [0031](../../docs/decisions/0031-project-format-version-2.md)):

- `to_bytes`/`from_bytes` and `save`/`open`. The file is a 20-byte header (`SIGNATURE` `AVTL`, `FORMAT_VERSION` 2, flags with bit 0 for zstd, the body's length) followed by the body: the file model (plain data separate from the scene's types) encoded with postcard and compressed with zstd. `project/v1.rs` holds the structures, generic over recorded operations (`ProjectFile<Op>`, `FileOperation`); version 1 uses `OperationV1` (moves), version 2 (`project/v2.rs`) `OperationV2` (moves, and points turned or scaled by a matrix). Both versions are read; version 2 is written.
- Reading checks everything, with plain-language `ProjectError`s (`NotAProject`, `TooNew`, `Damaged` naming the object, `TooLarge`, `Io`). Sizes are capped at 4 GB before reading and unpacking. Positions, rotations and moves must be real numbers; meshes go through `Mesh::from_slots`, the tree through `Scene::from_objects`; primitive settings are clamped.
- `save` writes `name.avtl.tmp`, syncs it and reads it back, copies the previous save to `name.avtl.bak` (`backup_path`), then renames the new file into place.

Tests:
- round trips of a scene using every feature (byte-identical on resave);
- the header;
- refusals of other files, newer versions, unknown flags, cut and padded files and bad values;
- clamping;
- fuzzing every cut, 3000 random bit flips and random bodies;
- saving on disk (backup, no leftovers, failure leaving the old file, unwritable folders, missing files);
- `tests/data/sample-v1.avtl` and `sample-v2.avtl`, files of each version that must open in every future version (`WRITE_SAMPLE=1` writes only the current version's).

Export arrives in Phase 5.
