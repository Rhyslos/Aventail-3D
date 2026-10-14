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

- `to_bytes`/`from_bytes` and `save`/`open`. The file is a 20-byte header (`SIGNATURE` `AVTL`, `FORMAT_VERSION` 14, flags with bit 0 for zstd, the body's length) followed by the body: the file model (plain data separate from the scene's types) encoded with postcard and compressed with zstd. `project/v1.rs` holds the structures, generic over recorded operations (`ProjectFile<Op>`, `FileOperation`); version 1 uses `OperationV1` (moves), version 2 (`project/v2.rs`) `OperationV2` (moves, and points turned or scaled by a matrix, each with its anchors, `AnchorsV2`, every number checked on reading; [decision 0032](../../docs/decisions/0032-edits-find-their-points-again.md)). Version 3 (`project/v3.rs`, [decision 0038](../../docs/decisions/0038-extrude-and-inset.md)) stores version 2's operations as they were (`OperationV3::V2`), then extruded and inset polygons and extruded edges (distance checked, grouping, anchors; an edge must join two different points). Version 4 (`project/v4.rs`, [decision 0040](../../docs/decisions/0040-bevel.md)) wraps version 3's operations (`OperationV4::V3`) and adds bevelled edges and points (width and profile checked, rows brought within 1–64, profile within -1–1). Version 5 (`project/v5.rs`, [decision 0044](../../docs/decisions/0044-edits-that-hold.md)) keeps each recorded edit as an `EntryV5` (id, on/off, pinned, version 4's operation as it was, and `AnchorsMoreV5`: the structure fingerprint), through `FileOperation::entry_to_file`/`into_entry` (older versions number their edits 1, 2, 3… in order); and plain meshes' attribute layers beside the objects (`MeshLayersV5`, every value checked by `Mesh::set_attribute_values`; hard edges stay with the mesh). Version 6 (`project/v6.rs`, [decision 0047](../../docs/decisions/0047-loop-cut.md)) is version 5's file with `OperationV6`: version 4's operations as they were (`OperationV6::V4`) and loop cuts (the edge's two points must differ, the position must be a real number; cuts brought within 1–64, position within 0–1). Version 5's structures are generic over the operation (`EntryWith<Op>`, `ProjectWith<E>`), so a later version adds only its new kinds. Version 7 (`project/v7.rs`, [decision 0049](../../docs/decisions/0049-slit.md)) adds slits to version 6's operations (`OperationV7::V6`): 2 to 256 points, every number real, an eye whose flat-view direction isn't zero. Version 8 (`project/v8.rs`, [decision 0050](../../docs/decisions/0050-pipe-bends.md)) adds bent extrudes (`OperationV8::V7` for the rest): every number real, the bend brought into range. Version 9 (`project/v9.rs`, [decision 0051](../../docs/decisions/0051-connect-bridge-weld.md)) adds Connect, Bridge and Weld. Version 10 (`project/v10.rs`, [decision 0052](../../docs/decisions/0052-clean-up.md)) adds the clean-up edits: deleted and dissolved elements (`ElementsV10`), filled holes, flipped polygons and Fix facing. Version 11 (`project/v11.rs`, [decision 0053](../../docs/decisions/0053-hard-and-soft-edges.md)) adds hard and soft edges (an edge must join two different points) and hard edges by angle (a real angle, brought within 0–180°). Version 12 (`project/v12.rs`, [decision 0055](../../docs/decisions/0055-basics.md)) is `ProjectWithInstances<E>`: version 5's model plus `InstanceV12` (object, source), each instance written in the object list as a group; `OperationV12::V11` holds version 11's edits. Version 13 (`project/v13.rs`) adds slid points (`OperationV13::SlidePoints`: a real amount, never towards itself) to version 12's file. Version 14 (`project/v14.rs`, [decision 0059](../../docs/decisions/0059-symmetry-for-every-edit.md)) adds mirrored edits (`OperationV14::Mirrored`: a version 13 edit that can be mirrored, the axes, at least one on, and a real middle); they don't nest. All fourteen versions are read; version 14 is written, and a sample of each is kept in `tests/data`. **Saves that always reopen** ([decision 0041](../../docs/decisions/0041-saves-that-always-reopen.md)): `to_bytes` reads its own bytes back with `from_bytes` and fails with `ProjectError::WouldNotReopen` (also when an edit can't be stored), so neither `save` nor autosave copies ever hand out a project that won't open; `save` then keeps the old file and writes the bytes to `name.avtl.unreadable`. `safe_write` (a new uniquely named temporary file, never following links, synced, read back, renamed over the target, folder synced) is used for the project, its `.bak` and autosave copies.
- Reading checks everything, with plain-language `ProjectError`s (`NotAProject`, `TooNew`, `Damaged` naming the object, `TooLarge`, `Io`). **The limits rulebook** (`limits`, [decision 0045](../../docs/decisions/0045-limits-for-files.md)): the body is capped at 512 MB packed and unpacked (`MAX_UNPACKED`; a zstd body that states a larger size is refused before unpacking); every list in the file model is a `Bounded` list, saved like a plain one, whose items are paid for from a 2 GB budget (`MAX_MEMORY`) while the file is read, so a list can't claim more memory than allowed (`within`, `within_budget`, `limit_hit`, and the plain message `TOO_MUCH`); and `ProjectFile::check_limits` refuses more than `MAX_OBJECTS` objects, `MAX_EDITS` edits or about `MAX_POLYGONS` polygons (primitives estimated with `Primitive::polygon_estimate`) before anything is built. Positions, rotations and moves must be real numbers; meshes go through `Mesh::from_slots`, the tree through `Scene::from_objects`; primitive settings are clamped.
- `save` writes `name.avtl.tmp`, syncs it and reads it back, copies the previous save to `name.avtl.bak` (`backup_path`), then renames the new file into place.

Tests:
- round trips of a scene using every feature (byte-identical on resave);
- the header;
- refusals of other files, newer versions, unknown flags, cut and padded files and bad values;
- clamping;
- fuzzing every cut, 3000 random bit flips and random bodies, and 150 well-formed files with random nonsense inside (every shape that opens also worked out, each within seconds);
- the limits: too many objects, edits or polygons refused with the limit named and at once, bodies unpacking to too much (with and without their size stated) refused, and a read stopping when its memory budget runs out; bounded lists saving like plain ones and refusing lists that claim too much;
- saving on disk (backup, no leftovers, failure leaving the old file, unwritable folders, missing files);
- an ignored heavy-mesh check timing saving and opening a project with a 3-million-polygon shape (`tools/heavy/heavy-check.sh`);
- `tests/data/sample-v1.avtl` and `sample-v2.avtl`, files of each version that must open in every future version (`WRITE_SAMPLE=1` writes only the current version's).

**Backup copies** (`autosave`, [decision 0036](../../docs/decisions/0036-autosave.md)):

- `Session::start` makes a `session-<time>-<process>-<n>` folder inside the autosave folder and keeps its `lock` file locked (`File::try_lock`) while the app runs; the system lets go of the lock when the process ends in any way, a crash included.
- `Session::set_project` notes the open project in `project.txt`. `write`/`write_copy_in` write `copy-N.avtl` (written beside its name, synced, then renamed) and delete the oldest beyond the number kept (at least one). `adopt` copies a restored copy into the new session, keeping its time. `end` deletes the folder (a normal close).
- `leftovers` lists the sessions whose lock can be taken (no longer running) with their copies, newest first, and tidies away dead sessions without copies; `discard` deletes one. A session folder without its lock file is left alone for its first minute (an app may be starting). Backup folders are made readable only by their owner on Linux and macOS.
- `mark_restoring` notes in a leftover's folder which copy is being restored, and `clear_restoring` removes the note once the restore finishes; a note still there at the next start leaves that copy out (`Leftover::left_out`), so a copy whose restore closed the app isn't offered again (decision 0045).

Tests: numbering and keeping the newest, a running session versus a crashed one, order and the untitled case, tidying and ignoring other files, adopting (with its time) and two sessions side by side.

Export arrives in Phase 5.
