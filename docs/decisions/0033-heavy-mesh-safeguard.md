# 0033. Heavy meshes: a safeguard, checked at phase gates

- **Status:** Accepted
- **Date:** 2026-10-12

## Problem

The project lead asked whether what we build now can handle scenes of 2–3 million polygons. The app is for game assets, which are far smaller. But someone may import a high-poly model (a scan, a sculpt) and model on top of it, so one object can be that heavy. That case must not make the app freeze or crash.

A measurement on a 2-million-polygon object (2026-10-12) showed where things stand:
- holding, copying and moving the mesh is fast (about 0.1 s, 115 MB), and so is finding what's under the mouse (35 ms);
- working out the mesh's connections takes 3 s, and building the picking map 1.9 s. Both are rebuilt whenever the shape changes, which includes every frame of a drag with snapping on.

So today, a heavy object would stutter badly while dragging. Game-sized shapes are unaffected.

Two questions follow: when to fix it, and how to keep it fixed.

## Options

When:
1. **When someone complains.** Cheapest now, but Phase 4's seven tools will all build on the slow parts, and changing them under seven tools costs more.
2. **A milestone at the start of Phase 4**, before the tools.

How to keep it fixed:
1. **A 3-million-polygon test in every pull request.** It catches a slowdown at once, but it is slow and heavy for every small change. The project lead judged it overkill.
2. **A heavy-mesh check at the end of every phase**, as part of the gate, **and whenever the project lead asks**. Ordinary pull requests keep their small benchmarks for code on a performance path.

## Decision

Option 2 for both:
- **Robustness with heavy meshes is a safeguard** (principle 6, wishlist W11), not the expected use. The targets for one 2–3 million polygon object are in DESIGN.md §4.8:
  - smooth orbiting;
  - selecting in under 50 ms;
  - dragging without visible stutter;
  - rebuilds of a few seconds at most, never a silent freeze.
- **The *Heavy meshes* milestone opens Phase 4.** It brings:
  - connections in flat lists;
  - a spatial index for picking and snapping;
  - updating only what changed;
  - small undo steps;
  - long jobs kept off the interface thread;
  - the heavy-mesh check itself.
- **The heavy-mesh check runs at every phase gate from Phase 4 on, and on request.** Its numbers go in the pull request. It is not part of the normal CI run.

## Consequences

- Phase 4 starts one milestone later, and the tools build on fast foundations.
- Between phase gates, a slowdown with heavy meshes can go unnoticed for a while. The small benchmarks on performance paths, and the gate check, limit how far it can go.
- Measured numbers depend on the machine. The check reports them against the targets with some margin, and the project lead's own feel test on his machine counts most.
