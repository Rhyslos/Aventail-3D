# 0028. Ask the project lead about controls before building a feature

- **Status:** Accepted
- **Date:** 2026-10-11
- **Amends:** [0019](0019-only-the-tool-keys-come-from-halberd.md)

## Problem

Decision 0019 said keys other than Halberd's tool keys are chosen by whoever builds the feature, explained in the pull request, and changed afterwards if the project lead asks. In practice the project lead wants to decide controls up front. They know how they like to work (for example, Ctrl rather than Shift to add to a selection), and changing a key after it is built and documented costs a round of testing.

## Options

1. **Keep choosing keys and explaining them afterwards** (0019 as written).
2. **Ask before building**: when a feature needs controls, list them with a recommended choice for each, and let the project lead pick or say what they prefer.

## Decision

Option 2, from 2026-10-11 on, at the project lead's request:

- Before building a feature that adds keys, mouse actions or shortcuts, ask the project lead what they should be.
  - Offer a recommended choice for each (common in 3D programs, no clashes) and one or two alternatives.
  - Explain in plain words anything that might be confused. For example: grow the selection is a different action from resizing the brush.
- Halberd's tool keys (`Q`, `W`, `S`, `R`, `T`, `B`) stay as 0019 set them; they don't need asking again.
- Keys already built stay unless the project lead asks for a change.
- When the project lead isn't there to answer (they said they're away, or the work runs unattended), use the recommended choices. Say so at the top of the pull request, so they can be changed before merging.

## Consequences

- One extra question round per feature, before any code is written.
- The controls are the project lead's own, so later rework is less likely.
- `CLAUDE.md`'s rule about keys is updated to match.
