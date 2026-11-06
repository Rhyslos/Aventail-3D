# matgraph

**Layer 1 · Core**

Material graphs: boxes joined by wires, worked out into seamless textures.

## Purpose

The engine behind the Material mode (Phase 12, [decision 0256](../../docs/decisions/0256-material-graph-plan.md), [decision 0257](../../docs/decisions/0257-material-graphs-core.md), the plan in [docs/plans/phase-12.md](../../docs/plans/phase-12.md)): what a graph is, every change to it, and working it out into pictures. Everything here can be tested without a window or a GPU.

## Must never

- Know about scenes, undo, files, the GPU or UI (the scene keeps graphs and records their edits; `fileio` saves them; the interface draws them)
- Change a graph except through a checked `GraphEdit`, or refuse one without a reason in plain words
- Make a texture that doesn't tile, or that changes with the number of cores
- Allocate a picture outside `Size::MIN` to `Size::MAX`, or panic on a damaged graph

## Depends on

`words` (the language layer: every reason, name and note goes through it), `glam`.

## Status

Phase 12, milestone 1 (the core):

- **Value kinds** (`kind`): `ValueKind` (shape, height, colour, whole material; `name`, `described`), `fit(from, to)`: `Fit::Fits`, `Fit::Converts(note)` (a shape as a height, greys as a colour, a material's height or colour) or the reason it can't ("a colour has no inside or outside").
- **Pictures** (`image`, `value`): `Image<P>` (square, row 0 the top; `at` wraps round past every edge, `sample` blends and wraps), `Grey` and `Rgb` (sRGB 0 to 1, or normals), `Size` (a power of two, 16 to 4096; `new` refuses others with the reason and the nearest size as a fix; `nearest`; `PREVIEW` 256). `Value` (`Shape`, `Height`, `Colour`, `Material`, each shared in an `Arc`; `converted`), `MaterialMaps` (colour, height, roughness, metal, normal (OpenGL, y up the texture), occlusion, curvature).
- **Refusals** (`refusal`): `Refusal { reason, fixes }`; `Fix` (put a box between, add a box, connect another output instead, use a size, use the nearest value, look at a box, remove a box; `describe` words it as a choice). Nothing changes until a fix is picked, which the interface does (milestone 2).
- **Kinds of box** (`registry`): `BoxType` (`info`, `run`), `BoxInfo` (id saved in files, plain name, `Group` (Make, Shape, Detail, Colour, Material: the experts' working order), "What does this do?", input and output `PlugSpec`s (key, name, kind, needed), `SettingSpec`s, "usually comes next", one per graph), `Label` (an English key through `words`, or a person's own words for boxes they make, milestone 7), `Job` (size, seed, cores, settings by key, inputs converted to each plug's kind; `fill_grey`, `fill_rgb`, `map_grey`), `Registry` (`standard()` made once, `with_standard_boxes`, `add` refuses a taken id, `iter` by group).
- **Settings** (`settings`): `SettingKind` (number, whole number, colour, choice, switch, seed), `SettingValue`, `SettingSpec::check` (out of range says so, offering the nearest), `nearest`; `Settings` keeps changed values by key (`resolved`, `normalised`), so kinds can gain settings and old graphs still read.
- **Graphs** (`graph`): `Graph` (name, seed, `GraphBox`es by `BoxId`, `Wire`s between `PlugRef`s, one per input, comment `Frame`s by `FrameId` with title, colour, rectangle and note; ids from one counter, never reused). `GraphEdit` (add, remove, dissolve (keeping the wire through), move, set a setting, connect, disconnect, add, change and remove frames, seed, rename; `describe` for undo) through `Graph::apply`, all checked. `check_connection` says how a wire fits or why not, with fixes (boxes that could go between, other outputs that fit, nearest first); loops and a box feeding itself are refused. `order` (each box after what feeds it), `feeds`, `boxes_in(frame)`, `to_parts` / `from_parts` (everything checked, settings kept in range, kinds from a newer app kept as they are). Limits: `MAX_BOXES`, `MAX_FRAMES`, `MAX_NAME`, `MAX_NOTE`.
- **Working out** (`eval`, `cache`, `parallel`): `Evaluator::run(registry, graph, size, wanted)` gives an `Evaluation` (each box's `BoxResult` (outputs and its picture) or the `Refusal` saying why it can't run: an empty needed input, a box it waits for, a kind from a newer app; `made`, `reused`, `problems`); `material` gives the Material out box's material. Each box's result is kept in the `Cache` under a hash of its kind, settings, size, seed and its inputs' keys, so only what changed is worked out again; results the last run didn't use are forgotten past the budget (512 MB standard). Each box's pixels are shared over the cores in bands of 64 rows (`std::thread::scope`), so the result is the same on any number of cores.
- **Seeds** (`seed`): `mix`, `box_seed` (the box's Seed setting mixed with the graph's), `unit`, `lattice`: patterns come from hashing, never a clock.
- **The first boxes** (`boxes`): Noise (layered gradient noise on a wrapping lattice), Tiles (with a shift for bricks), Levels, Blend (six modes, through an optional shape), Colour fill, Material from height (normal, occlusion and curvature from the height; colour, roughness and metal), Material out (the graph's result, with its real-world size; one per graph). `blur::box_blur` wraps round. The full first set comes with milestone 3.
- **Projects' graphs** (`store`): `GraphStore` (graphs by `GraphId`, each shared, ids never reused; `from_graphs` for files; `MAX_GRAPHS`).
- Tests in each module and `graph/tests.rs`, `eval/tests.rs`, `boxes/tests.rs`: refusals and their fixes, loops, frames, damaged parts, a fuzz run of random edits, seamless tiling of every maker and of a made material, cache hits after each kind of change, seeds, the same result on 1 to 16 cores, empty and unknown boxes, the cache budget. `graph_speed` (ignored) times a graph by hand.
