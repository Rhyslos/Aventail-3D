# 0257. Material graphs in the project: the core

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 12's first milestone ([decision 0256](0256-material-graph-plan.md), [the plan](../plans/phase-12.md)) builds the engine every later milestone stands on: what a graph is, how it is changed, checked and worked out, and how it is saved. Fourteen milestones will build on it (the canvas, the boxes, the painter's graph layers, your own boxes, the shape canvas, animated boxes), so its shape needed choosing with care.

## Options

1. **Boxes as one big list of kinds in code** (an enum): simple, but every new box touches the same match statements, and boxes you make yourself (milestone 7) couldn't be added at run time.
2. **Boxes as kinds behind one trait, listed in a registry.** Chosen. Each kind is one file saying everything the interface shows (plain name, group, "What does this do?", plugs, settings, what usually comes next) and how it works.

For saving: storing `matgraph`'s own types with serde, or a file model of the project file's own. Chosen: **the file's own model** (`fileio/src/project/v95.rs`), as for everything else saved ([decision 0024](0024-saving-projects.md)), so the engine's types can change without breaking old files.

For working out: per-pixel functions shared over the cores, or whole-image passes. Chosen: **each box makes its picture in bands of 64 rows over the cores** (`std::thread::scope`; rayon wasn't needed), with row-by-row work for blurs, so results are exactly the same on any number of cores.

## Decision

- A new core crate, **`matgraph`** (layer 1; depends only on `words` and `glam`; no files, GPU or interface).
- **Four value kinds** on wires: shape, height, colour, whole material. A plug takes its own kind; a shape and a height convert into each other, greys into a colour, and a material gives its height or colour, each with a note. Everything else is refused with the reason in plain words ("a colour has no inside or outside").
- **Every refusal is a `Refusal`**: the reason, and ways to make it work as data (`Fix`: put a box between, add a box, connect another output, use the nearest value or size, look at a box, remove a box). The interface offers them (milestone 2); nothing changes until one is picked.
- **A graph** holds boxes (kind id, place, changed settings by key), wires between plug keys (one per input), comment frames (title, colour, rectangle, note; a box is in a frame when it sits inside it, and frames nest the same way), a name and a seed. Ids come from one counter and are never reused. It changes only through a checked **`GraphEdit`**; loops are refused.
- **The scene keeps a project's graphs** (`GraphStore`, by `GraphId`, never reused) and records `Command::AddGraph`, `SetGraph` and `EditGraph` as undo steps, each graph shared so an undo step costs only the graph that changed. Slider drags merge with the usual merge key.
- **Working out:** at a square size, a power of two from 16 to 4096 (others refused, offering the nearest). Every picture wraps round at its edges and every pattern repeats a whole number of times, so everything tiles seamlessly. Each box's result is cached under a hash of its kind, settings, size, seed and its inputs' keys, so only what changed is worked out again; results not on screen are forgotten past a memory budget (512 MB). A box's seed is its Seed setting mixed with the graph's seed; nothing uses a clock.
- **A box that can't run** says why (an empty needed input, a box it waits for, a box from a newer app), and the rest of the graph still runs.
- **Seven first boxes**, enough to test the engine end to end: Noise, Tiles, Levels, Blend, Colour fill, Material from height, Material out. Milestone 3 adds the real set.
- **Saved in the project file, version 95.** Settings out of range in a file are brought into range; boxes of kinds this app doesn't know are kept, and refuse to run with a reason.

## Consequences

- Nothing a user sees changes yet: the Material mode and the canvas come with milestone 2. Projects saved now need this version or newer to open.
- A new box is one file in `matgraph/src/boxes/` and one line in `boxes::standard`. Box, plug and setting ids are saved in files, so they never change once released.
- Your own boxes (milestone 7) can implement the same `BoxType` trait (`Label::Own` carries a person's own words), added to a registry; the scene will then need the project's registry rather than `Registry::standard()`.
- The CPU evaluator takes about 30 ms for a small graph at the preview size (256) on two cores, and about 1.7 s at 2048; Noise is the heaviest box. A later milestone can move heavy boxes onto the GPU without changing the crate's interface.
