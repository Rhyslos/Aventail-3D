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

- `to_bytes`/`from_bytes` and `save`/`open`. The file is a 20-byte header (`SIGNATURE` `AVTL`, `FORMAT_VERSION` 3, flags with bit 0 for zstd, the body's length) followed by the body: the file model (plain data separate from the scene's types) encoded with postcard and compressed with zstd. `project/v1.rs` holds the structures, generic over recorded operations (`ProjectFile<Op>`, `FileOperation`); version 1 uses `OperationV1` (moves), version 2 (`project/v2.rs`) `OperationV2` (moves, and points turned or scaled by a matrix, each with its anchors, `AnchorsV2`, every number checked on reading; [decision 0032](../../docs/decisions/0032-edits-find-their-points-again.md)). Version 3 (`project/v3.rs`, [decision 0038](../../docs/decisions/0038-extrude-and-inset.md)) stores version 2's operations as they were (`OperationV3::V2`), then extruded and inset polygons and extruded edges (distance checked, grouping, anchors; an edge must join two different points). All three versions are read; version 3 is written, and a sample of each is kept in `tests/data`.
- Reading checks everything, with plain-language `ProjectError`s (`NotAProject`, `TooNew`, `Damaged` naming the object, `TooLarge`, `Io`). Sizes are capped at 4 GB before reading and unpacking. Positions, rotations and moves must be real numbers; meshes go through `Mesh::from_slots`, the tree through `Scene::from_objects`; primitive settings are clamped.
- `save` writes `name.avtl.tmp`, syncs it and reads it back, copies the previous save to `name.avtl.bak` (`backup_path`), then renames the new file into place.

Tests:
- round trips of a scene using every feature (byte-identical on resave);
- the header;
- refusals of other files, newer versions, unknown flags, cut and padded files and bad values;
- clamping;
- fuzzing every cut, 3000 random bit flips and random bodies;
- saving on disk (backup, no leftovers, failure leaving the old file, unwritable folders, missing files);
- an ignored heavy-mesh check timing saving and opening a project with a 3-million-polygon shape (`tools/heavy/heavy-check.sh`);
- `tests/data/sample-v1.avtl` and `sample-v2.avtl`, files of each version that must open in every future version (`WRITE_SAMPLE=1` writes only the current version's).

**Backup copies** (`autosave`, [decision 0036](../../docs/decisions/0036-autosave.md)):

- `Session::start` makes a `session-<time>-<process>-<n>` folder inside the autosave folder and keeps its `lock` file locked (`File::try_lock`) while the app runs; the system lets go of the lock when the process ends in any way, a crash included.
- `Session::set_project` notes the open project in `project.txt`. `write`/`write_copy_in` write `copy-N.avtl` (written beside its name, synced, then renamed) and delete the oldest beyond the number kept (at least one). `adopt` copies a restored copy into the new session, keeping its time. `end` deletes the folder (a normal close).
- `leftovers` lists the sessions whose lock can be taken (no longer running) with their copies, newest first, and tidies away dead sessions without copies; `discard` deletes one.

Tests: numbering and keeping the newest, a running session versus a crashed one, order and the untitled case, tidying and ignoring other files, adopting (with its time) and two sessions side by side.

Export arrives in Phase 5.
