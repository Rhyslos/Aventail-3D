# 0264. Starter graphs

- **Status:** Accepted
- **Date:** 2026-11-07

## Problem

Phase 12's eighth milestone ([the plan](../plans/phase-12.md), [decision 0256](0256-material-graph-plan.md)) is "Starter graphs": ready-made graphs to open and change instead of an empty canvas (Bricks, Stone tiles, Cobbles, Planks, Stylised wood, Bark, Fabric, Metal plate, Ground; Leaves after milestone 13), each with notes inside explaining each step. Research for the plan found that not knowing where to start is the first thing that makes Substance Designer hard ("a brick wall can be built in over 1000 ways"). Things to decide: how the starters are stored, how one is opened, where the notes go, what they teach, and how we know they look right.

## Options

- **Stored as:** Rust code building each graph at run time, or **data files read through a checked reader** (chosen). A starter is a **graph file** (`.matgraph`, readable JSON: the graph as the project file stores it, with any own boxes it uses), the format the plan names for sharing graphs, so the starters are read by the same fuzz-tested reader that will read graph files people share. The files are made from **recipes in the tests** (`fileio::starter_graphs` tests, `recipes/materials.rs`): each recipe adds boxes by column and groups them into steps; the recipe lays the boxes out under each frame's note and the tests fail while a file differs from its recipe (`UPDATE_STARTERS=1` writes them again). Editing a recipe is easier to review than editing JSON, and the shipped file is still plain data.
- **Card pictures:** worked out when the window opens (nine graphs at once, a second or more on two cores), or **a small picture kept beside each file** (chosen): 128 pixels, the material lit from the top left by its normal (`card_picture`), made with the files, and a test checks the picture still matches what the graph makes.
- **Opening one:** link the starter (changes elsewhere would follow), or **copy it into the project** (chosen), named after it ("Bricks 2" when taken), with any own boxes it uses (`GraphFile::fitted_to` renames a starter's own box when the project holds a different box by that id, so a copy never changes, or is changed by, boxes already there). One undo step (`Command::SetOwnBox`s and `Command::AddGraph`). The canvas shows it from its first step at a zoom where the notes are readable (`View::frame_readable`, 0.8 at least) rather than the whole graph too small to read.
- **Where:** a window only, or **the canvas bar's Starter graphs button, the empty canvas, and the shelf** (chosen). The bar's button opens a window of large cards (picture, name, what it teaches); the empty canvas's main button is now **Start from a starter graph** (New graph stays under it); the Material mode's shelf gets a **Starter graphs** row of small cards, a click opening a copy.
- **The notes:** a "why this is here" note on each box, which would need a new box field (a file format change), or **comment frames' notes** (chosen): one frame per step, its title numbered in working order ("1. Lay the bricks", "2. Shape them"…), its note saying what the step does and why in plain words. Frame notes were only in Attributes; they are now **drawn on the canvas** under the title, in the room above the frame's first box, when zoomed in enough to read (0.45). The recipes leave that room; a test measures each note with the app's font. A frame's note drawn this way works for anyone's frames, not just the starters.
- **What each teaches:** the starters build on each other, simplest first, each showing one or two ideas on top of the pattern → shape → colour → finish order. Bricks: the three basic steps, and a grey that says what each spot is before colouring. Stone tiles: Warp, lighter edges with Edge finder, moss by height. Cobbles: Cells' Edges look as a shape, colour per stone from Random, Hue and saturation through a Where shape. Planks: one box's three outputs. Stylised wood: a hand-painted look with the light painted into the colour. Bark: Planks turned upright and warped, lichen by shape. Fabric: a chequer from stripes, a weave, a 6 cm real-world size. Metal plate: one pattern limited by another (rivets near edges), metal and roughness. Ground: layering materials by height and by shape. Each uses only the app's own boxes; own boxes inside starters are supported by the files but none is used yet.
- **Looking right:** each starter was tuned by drawing it at 512 pixels (colour, lit, height, two by two) and looking at the pictures (`starter_renders`, by hand), hand-painted first (light and shade in the colour, bright edges), then lit and realistic (normal, roughness, metal). Each has a real-world size, three to eight settings **shown in the painter**, tiles seamlessly at the preview's sizes, and is tidy (the tidy layout would move nothing).
- **Leaves:** after milestone 13 (the edge randomiser), as planned; the window says so.

## Decision

- **fileio:** `graph_file` (`GraphFile` (graph and own boxes; `of`, `fitted_to`), `write_graph_file`, `read_graph_file`, `GRAPH_EXTENSION` "matgraph", `GRAPH_FORMAT` 1, `MAX_GRAPH_FILE`), fuzz-tested; `starter_graphs` (`Starter` (key, name, what it teaches, the file, the card), `STARTERS`, `find`, `load`, `card_picture`; the files in `crates/fileio/starters/`). No project file format change.
- **ui:** `material::starters` (the window, the shelf row, `open_starter`); the bar's **Starter graphs** button; the empty canvas's **Start from a starter graph**; frame notes drawn on the canvas (`look::NOTE_SIZE`, `NOTE_PAD`, `NOTE_ZOOM`); frame titles clipped to their band; `View::frame_readable`.
- **Controls:** buttons and cards only; no keys or mouse actions added.

## Consequences

- A newcomer has nine worked examples to read and change, each explaining itself on the canvas.
- Graph files exist; **Export as a graph file… / Bring in a graph file…** to share whole graphs need only the buttons (not built yet).
- A frame's note shows on the canvas for every graph, so a long note in an old project now shows above its boxes (when there is room).
- Changing a box's maths changes the starters' look: the card test fails and says to write the files again, and the pictures should be looked at again then.
