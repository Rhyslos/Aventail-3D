# 0031. Project file format version 2

- **Status:** Accepted
- **Date:** 2026-10-11
- **Follows:** [0024](0024-saving-projects.md)

## Problem

Rotating and scaling points records a new kind of edit, `TransformPoints` ([0030](0030-transform-gizmos.md)). Version 1 of the project file can only store `MovePoints`, and its writer skipped operations it didn't know. Saving would have lost every turn and scale of points. Decision 0024 requires a new format version for any change to what a project holds.

## Options

1. **Add the variant to version 1's model.** Old files would still read, but the encoding of "version 1" would change meaning, which 0024 rules out.
2. **A full copy of version 1's model as version 2.** About 400 lines repeated for one new variant.
3. **Make version 1's structures generic over the operation type**, and define version 2 as the same structures with a larger operation enum.

## Decision

Option 3.
- `v1.rs` keeps the structures, now `ProjectFile<Op>`, `ObjectFile<Op>` and `LayerFile<Op>`, and a `FileOperation` trait that converts an operation to and from the file.
  - Generic parameters don't change postcard's encoding, so version 1 files read exactly as before.
  - The committed `sample-v1.avtl` still opens in the tests, unchanged.
- `v2.rs` defines `OperationV2` (`MovePoints`, then `TransformPoints` with the matrix's 12 affine numbers) and `ProjectV2 = ProjectFile<OperationV2>`.
- The header's version picks the model: 1 is read with version 1's operations, 2 with version 2's. A version 1 file needs no upgrade step, as the scene it describes is the same.
- The app always writes version 2 (`FORMAT_VERSION = 2`), and a new `sample-v2.avtl` is committed with a test. Every matrix value is checked on reading, as everything else is.
- Before release, version 2 also gained each edit's anchors (where its elements were), for [0032](0032-edits-find-their-points-again.md). Version 2 had not been released, so it was changed in place and its sample rewritten.

## Consequences

- Projects saved from now on can't be opened by older builds of the app. They refuse them clearly ("made by a newer version"), as 0024 designed.
- The next change to operations adds a variant at the end of a new `OperationV3` the same way. A change to anything else (objects, layers) will need its own generic step or a new model.
