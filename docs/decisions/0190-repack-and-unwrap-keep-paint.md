# 0190. Re-pack and re-unwrap keep the paint

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's twelfth milestone ([0177](0177-texture-painter-plan.md)): re-packing or re-unwrapping UVs after painting must not lose the paint. Paint following the model ([0189](0189-paint-follows-the-model.md)) already moves paint with its polygons after any change, packing and unwrapping included. Two things were left:
- Packing often makes pieces smaller on the texture. Reading each new texel from one place of the old texture then picks fine detail at random (stripes come out as stripes or as gaps, not as their average).
- Nothing told the user the paint had moved.

## Decision

- **Shrunk polygons are averaged.** When a polygon's triangle covers less of the texture than before, each new texel is averaged from samples spread over the old texels it now stands for: as many across each way as it shrank (rounded up), at most 4 by 4, each sample read smoothly and weighted by its coverage. A polygon the same size or bigger keeps the single smooth read, so a piece moved by whole texels stays exact.
- **The status bar says so:** "The paint moved with its polygons (one undo takes both back)."
- Re-unwrap, pack, straighten, texel density, trim placement, layout snapshots and moving UVs by hand all go through the same path: they are recorded UV edits, so the paint follows them in their own undo step.

## Consequences

- Packing tighter keeps the look of fine paint, a little softer, as a smaller picture would.
- Packing looser (pieces made bigger) can't add detail that wasn't painted: the paint is smoothly enlarged. Repainting, or texture resizing with strokes replayed (milestone 13), gives sharper results.
