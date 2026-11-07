# Phase 12 plan: materials with a node graph

The working plan for Phase 12 ([decision 0256](../decisions/0256-material-graph-plan.md); the roadmap is in [DESIGN.md](../../DESIGN.md#phase-12--materials)). It was agreed with the project lead on 2026-11-06, after research into why Substance Designer is hard to learn and how friendlier tools (Substance 3D Sampler, Quixel Mixer, InstaMAT, Material Maker) give the same power. It records what was chosen, the architecture, and a plan for every milestone. Update it as milestones are built.

## What the project lead asked for

- **Materials and textures like Substance Designer, as a node graph**, but not copied from it, because its way of working is confusing. The research behind that is summarised below.
- **No engine shaders inside the app.** Graphs run on the app's own engine. On export they are baked to ordinary textures, and a **translator** writes engine shader code (Godot first) only for what has to move or change in the game.
- **A shape canvas inside the node editor:** draw or cut out simple vector shapes (leaves, say) without leaving for another app, change them afterwards, layer many of them as an assembly with copies that can be dragged around, and an **edge randomiser** so every razor leaf comes out different. Made for foliage artists.
- **Starter graphs** to open and change instead of an empty canvas, and **your own boxes**: a group of boxes saved as one box with its own sliders.

## Why Substance Designer is hard, and what this graph does instead

| What makes Substance Designer hard | What this graph does |
|---|---|
| A simple brick wall "can be built in over 1000 ways"; no idea where to start | **Starter graphs** for common materials, and **"what usually comes next"** suggestions on each box |
| Tangled wires: "a spider web", wires running backwards and forwards | **The app keeps the layout:** everything flows left to right, boxes snap into columns, wires never run backwards. You can still move boxes; the app tidies around them |
| Colour, roughness, normal and height are each a separate chain of nodes | **One wire can carry a whole material** (all its maps together). **Height first:** a "Material from height" box makes the normal, occlusion and curvature, so most graphs only shape the height and colour it |
| The expert working order (big shapes, then detail, then colour) is hidden | Box groups are named in that order (**Make**, **Shape**, **Detail**, **Colour**, **Finish**), and the box search lists them that way |
| Unclear what a node does or what to plug where | **Plain names**, a **live picture on every box**, **wires coloured by what they carry** (a shape, a height, a colour, a material), plugs that only take what makes sense (or convert with a note), and **"What does this do?"** on every box |
| When something won't work, it just doesn't, or shows a red error | **Every refusal says why, in plain words, and how to make it work** (see below) |
| No sense of real size, so materials repeat badly | Every graph has a **real-world size** (how many centimetres one repeat covers) and **tiles seamlessly** by default |

## Choices made (2026-11-06)

- **Where it lives:** a **Material** mode in the left palette, after Paint, like the painter's mode. The node canvas takes the main area. A **preview** sits beside it (on the selected model, a ball, or a flat tiled square) and shows the 2D texture tiled. The chosen box's settings go in **Attributes**, as everywhere in the app.
- **Controls** are listed under [Controls](#controls), chosen by the project lead ([decision 0028](../decisions/0028-ask-about-controls.md)).
- **Hand-painted first, realism after**, as in the painter.
- **Saying why something won't work** (the project lead, after the mockups). Whenever the app refuses something, it explains why in plain words and offers ways to make it work. Nothing changes until you pick one. This covers wires between plugs that don't fit, a box that can't run (a missing input, a loop), a setting out of range, and a box the Godot translator has to bake. For example, dragging a colour onto Bevel's shape plug turns the plug red. A note then says Bevel needs a black-and-white shape to know which edges to round, and that a colour has no inside or outside. It offers "Put a Colour to shape box between them" and "Connect Tiles instead". Letting go cancels; the status bar repeats the reason.
- **Comment frames, like Unreal's Blueprint comments** (the project lead's wish). These are loose, coloured frames with a title, for grouping boxes:
  - Press **C** to put a frame around the selected boxes.
  - Dragging the title moves everything inside; dragging an edge resizes the frame, and a box is in a frame when it sits inside it.
  - Double-click the title to rename it; its colour and a longer note are in Attributes.
  - Frames can sit inside frames.
  - When zoomed out, the titles stay readable, so a big graph reads as its frames.
  - The tidy layout keeps a frame's boxes together.

## Architecture

### Crates

- **`matgraph`** (new, core layer, no UI or GPU; depends on `glam`): the graph (boxes, plugs, wires), the value kinds (shape mask, height, colour, whole material), evaluating a graph at a chosen size, seamless tiling, caching each box's result so only what changed is worked out again, random seeds, and the shader translators (`matgraph::translate::godot`). It can be tested without a window.
- **`shapes`** (new, core layer, no UI or GPU): vector shapes for the shape canvas. It covers paths with straight and curved sides, cut-outs, copies (plain and linked), the edge randomiser, and turning shapes into a sharp mask at any size. Planned on `kurbo` (curves, MIT/Apache-2.0) and `tiny-skia` (filling, BSD-3-Clause). Both licences are permissive; check them again when the milestone starts.
- **`paint`** gains a **graph fill layer** that runs a graph on the model, using the model's baked maps (curvature, occlusion, position, thickness) as inputs.
- **`ui`** gains the node canvas, the shape canvas and the Material mode.
- **`fileio`** saves graphs and shapes inside the project, and as `.matgraph` files to share.

### How graphs run

- The first version evaluates on the CPU, in tiles, using several threads (`std::thread`; `rayon`, MIT/Apache-2.0, if it helps). Results are cached per box. While you drag a slider, the preview uses a small size and fills in full size after you let go.
- If big graphs turn out slow, a later milestone moves the heavy boxes onto the GPU through the render crate's `wgpu`. The `matgraph` crate itself never depends on `wgpu`.
- **Material Maker** (MIT licence) is a reference for the pattern makers and their maths. Any code adapted from it keeps its MIT licence notice. Nothing comes from Substance (no names, icons or presets).

### Export

- **Baked:** a graph's material is baked to textures through the existing texture export, with the engine presets and channel packing (decisions 0204, 0208). This works in every engine.
- **Translated (later, not in this phase):** the project lead moved the translators out of Phase 12 on 2026-11-06, so the Godot, Unity and Unreal translators can be built together in one go later. How they will work: a graph with animated parts (time, scrolling, pulsing) or sliders marked "change in the game" is translated into a **Godot shader** (`.gdshader`) plus a material that uses it. Anything that doesn't change is baked into textures the shader reads, so the shader stays small. Only boxes marked "translatable" can stay live; the export check says which ones were baked instead. Until then, animated parts are baked at their current setting on export, and the export check says so.

## Milestones

### A. The core

1. **Material graphs in the project.** The `matgraph` crate: boxes, plugs, wires, the four value kinds, evaluation, seamless tiling, caching, seeds. Graphs saved in the project. No interface yet beyond a placeholder list. (File format version bump.)
2. **The node canvas and the Material mode.**
   - The canvas: boxes with live pictures, coloured wires, plugs that refuse the wrong kind, and the app keeping the layout left to right.
   - The box search, the preview beside the canvas, and box settings in Attributes.
   - Comment frames (C), and the explanations when something won't work.
   - Undo for every change.
3. **The first boxes**, about 25. Each has a plain name, a "What does this do?" note, and "what usually comes next".
   - **Make:** Noise, Clouds, Cells, Gradient, Bricks, Tiles, Planks, Basic shape (circle, square, polygon, star).
   - **Shape:** Blur, Warp, Levels, Edge finder, Bevel (rounds a shape's edges into height), Transform, Repeat, Blend.
   - **Colour:** Colour fill, Gradient map (greys to colours), Hue and saturation.
   - **Material:** Material from height (normal, occlusion, curvature), Mix materials (by height or a mask), Material out (with its real-world size).
4. **Preview and checks.**
   - Preview on the selected model, a ball, a cube, or a flat tiled square, with the app's looks (unlit, lit, toon, realistic).
   - A seam check that shows the tiles' joins.
   - The texture budget for the chosen size.
5. **Graphs in the painter.** Use a graph as a fill layer or a smart material. The model's baked maps are inputs: "lighter on edges" or "moss in the crevices" follow the model you are painting.
6. **Baked export.** Graphs into textures through the texture export, with every engine preset. A graph material can also be given to a shape without painting.

### B. Making it yours

7. **Your own boxes.** Select boxes and turn them into one box, choosing which settings become its sliders. They are saved in a library on the shelf, and shared as files. Opening one shows the boxes inside.
8. **Starter graphs**, with notes inside explaining each step: Bricks, Stone tiles, Cobbles, Planks, Stylised wood, Bark, Fabric, Metal plate, Ground, and (after milestone 13) Leaves.
9. **Scatter.** Place copies of a shape or a material in a grid or at random, with random size, turn, colour and height. This covers pebbles, grass tufts and scales.
10. **More boxes**, for both the hand-painted and the realistic side:
    - Cracks, Chips, Scratches, Fill the gaps (moss, dirt, grout);
    - Brush strokes (a hand-painted look), Edge highlight, Slope blur;
    - Directional light (painted-in light and shade, matching the painter's helper).

### C. The shape canvas (the project lead's idea)

11. **The shape canvas.** A **Shape** box opens a canvas.
    - **Drawing:** shapes with a pen (points with straight or curved sides), basic shapes, and **cut-outs** (one shape taken out of another). There is a **mirror line** for symmetrical shapes like leaves, and a reference image can sit underneath.
    - **What it makes:** a sharp mask at any texture size (vectors, so never blurry), a rounded height (shapes puffed up a little), and its outline.
    - **Changing it later:** you can go back and edit the shape at any time.
12. **Assemblies.**
    - Many shape pieces on one canvas, each with its own colour, height and order.
    - Pieces can be dragged, turned, sized, flipped and grouped.
    - Plain copies, and **linked copies**: change one and every linked copy changes, the way instances work in the scene.
    - For example, build a leaf once, then a twig out of linked leaves.
13. **The edge randomiser and variation.**
    - **Edge:** an edge setting per shape (jagged, serrated, wavy, torn), with amount, size and a seed.
    - **Vary copies:** gives every copy its own seed, size, angle and colour shift, so no two leaves match, even linked ones.
    - **Veins:** lines on the canvas become veins in the height and colour.
14. **Foliage output.**
    - An **opacity** (alpha) map from the shapes, and a translucency map (light through the leaf).
    - Normals from the rounded height.
    - Several leaves or twigs laid out on one **atlas** (a leaf sheet), using the packer.
    - Exported as alpha-card textures, with the engine presets setting the alpha right. Godot's settings are checked in Godot.

### D. Moving materials and engines

15. **Animated boxes.** Time, Scroll, Pulse and Flow, shown moving in the app's own preview (no engine shaders inside the app). Sliders can be marked "change in the game".
*The engine translators (Godot, Unity and Unreal, together) are left for later, at the project lead's request. Until they come, animated materials move only in the app and export baked at their current setting.*

### Done when (the gate)

The project lead makes, without help:
- a stylised stone-tile material, starting from an empty graph;
- a leaf atlas using the shape canvas, with randomised edges.

They use one of them in the painter, and see both look right in Godot (baked). The gate also includes one animated material, seen moving in the app's preview.

## Controls

**Chosen by the project lead on 2026-11-06:** the recommended choices below, except that wires are only removed by dragging their end away. Each is common in other node editors or follows the app's own keys, and none clashes with an existing key.

### Node canvas

| Action | Chosen | Why | Alternatives |
|---|---|---|---|
| Add a box | **Space** over the canvas opens a box search at the pointer; **right-click** on empty canvas gives the same list as a menu | Space already opens the command search; over the canvas it searches boxes instead. The right-click menu is the app's default (decision 0126) | Tab (Unreal/Blender-style; but Tab changes modes in the app), Shift + A (Blender) |
| Connect | **Drag** from a box's output dot to another box's input dot; drop on empty canvas to open the box search with matching boxes | What every node editor does; dropping on empty space is the quickest way to add the next box | — |
| Disconnect | **Drag a wire's end away** from its input (the project lead's choice: no Ctrl-drag cutting) | The most common way | Ctrl + drag across wires (Blender), Alt + click a wire (Substance) |
| Pan, zoom | **Middle drag** pans, **wheel** zooms, as in the UV Editor | Same as the rest of the app | — |
| Frame | **Z** frames the selected boxes (or all if none) | Z already frames the selection in the viewport | F (Blender, Substance; but F is Loop cut in the app) |
| Select | Click, drag a box around several, **Ctrl** adds, **Shift** removes | The app's rule (decision 0133) | — |
| Copy, paste, linked copy | **Ctrl + C / V**; **Alt + C / V** pastes a linked copy | Same as objects (decision 0055) | Ctrl + D duplicate (but D is Extrude) |
| Delete | **Delete** removes boxes; **Backspace** removes a box but keeps the wire through it | Delete and Backspace already mean delete and dissolve in the app | — |
| Make your own box | **Alt + G** groups the selected boxes into one; **Shift + G** opens it back up | Alt + G / Shift + G already group and ungroup objects | Ctrl + G (Blender) |
| Comment frame | **C** puts a frame around the selected boxes (or an empty one at the pointer); drag its title to move what's inside, its edges to resize; double-click the title to rename | Unreal Blueprints' key and behaviour, which the project lead likes; C isn't used on the canvas | Shift + C, menu only |
| Look through a box | **Click a box's picture** to show that box in the preview | Substance and Material Maker show any node in a 2D view; a click is the easiest | Double-click |

### Shape canvas

| Action | Chosen | Why | Alternatives |
|---|---|---|---|
| Select and move pieces | **Q** select, **W** move, **R** rotate, **S** scale, **T** all, with the same gizmos | The app's own tool keys (decision 0019) | V (Illustrator) |
| Pen | **P** draws points: click for a straight side, drag for a curve; click the first point to close | P is the pen in Illustrator, Inkscape, Figma and Affinity | — (in Paint mode P picks colour, but the canvas has its own keys) |
| Basic shapes | **B** opens the shape list (circle, square, polygon, star) | B already means new shape | — |
| Cut out | Hold **Alt** while drawing to make the shape a cut-out; or the Add / Cut switch on its bar | Alt for "take away" while drawing is common (Photoshop selections) | A Cut button only |
| Mirror line | **M** mirror on or off (the line can be moved) | Easy to remember; M (Weld) is a modelling key, not used on the canvas | A button only |
| Edit points | **Double-click** a shape to edit its points; Esc goes back | Common to vector apps | Enter |

## Where things stand

**Standing OK from the project lead (2026-11-06): milestones 1 to 15 are built one after another, each pull request merged as soon as its checks pass.** They test everything at the end. Each milestone adds its test steps to the testing document.

| # | Milestone | PR | Decision | File format |
|---|---|---|---|---|
| 1 | Material graphs in the project | #280 | [0257](../decisions/0257-material-graphs-core.md) | 95 |
| 2 | The node canvas and the Material mode | #281 | [0258](../decisions/0258-node-canvas.md) | — |
| 3 | The first boxes | #282 | [0259](../decisions/0259-first-material-boxes.md) | 96 |
| 4 | Preview and checks | #283 | [0260](../decisions/0260-preview-and-checks.md) | — |
| 5 | Graphs in the painter | #284 | [0261](../decisions/0261-graphs-in-the-painter.md) | 97 |
| 6 | Baked export | (this PR) | [0262](../decisions/0262-baked-export.md) | — |

Next decision: 0263. Next file format: 98.

### What milestone 1 built, for the milestones after it

- **The crate:** `crates/matgraph` (its README lists every part). Graphs change only through `GraphEdit` (`Graph::apply`), which the scene records with `Command::EditGraph { id, edit }`; for a slider drag, use `Document::execute_merging` with one key, as elsewhere.
- **Refusals:** every `apply`, `check_connection` and box that can't run gives a `Refusal { reason, fixes }`. `Graph::check_connection` is what the canvas calls while a wire is dragged over a plug (red plug, the note, the fixes as choices); `Fix::describe` words a fix.
- **The interface needs** (milestone 2): run `Evaluator` on another thread at `Size::PREVIEW` while dragging and at full size after; `Evaluation::picture(id)` is each box's live picture, `Evaluation::problems()` the boxes to mark. Box groups and order come from `Registry::iter`; `BoxInfo::comes_next` feeds the "usually comes next" list; `Label::show` gives any name in the language in use.
- **Comment frames** are data only (`Frame`: title, colour, `min`/`max`, note); `Graph::boxes_in(frame)` finds a frame's boxes by place. Moving a frame's title should be one `GraphEdit::MoveBoxes` of `boxes_in` plus a `ChangeFrame`, under one merge key.
- **New boxes** (milestone 3): one file in `matgraph/src/boxes/`, a line in `boxes::standard`. Keep box, plug and setting ids fixed once released; a pattern must repeat a whole number of times and read neighbours with `Image::at` (it wraps), and `boxes/tests.rs` checks seams. `boxes::blur::box_blur` wraps too.
- **Ids in files:** box kinds unknown to the app (a newer app's) are kept and refuse to run; settings out of range are brought into range on opening.
- **Not built yet:** no interface at all (as planned: nothing to see); the real-world size is Material out's setting, used from milestone 4.

### What milestone 2 built, for the milestones after it

- **Where it is:** `crates/ui/src/material.rs` and `material/` (the ui README lists every part); `workbench/material_mode.rs` switches the layout. Tests in `workbench/material_mode_tests.rs` show how to drive the canvas (`plug_on_screen`, `box_on_screen`, `frame_on_screen`).
- **Changing a graph from the interface:** `MaterialEditor::run(document, edits, label, key, tidy)` checks the edits on a copy, adds the tidy layout's moves when asked and makes them one undo step. A refusal comes back as a `Refusal`; `show_refusal` puts it in the note with its fixes, and `carry_out_fix` knows every `Fix` (`UseSize` and `UseNearest` do nothing on the canvas yet: milestone 4's size choice and the settings could use them).
- **New boxes (milestone 3)** need nothing on the canvas: the search, Attributes (`box_rows::setting_row` draws every `SettingKind`) and the plugs all come from `BoxInfo`. A box with many plugs just grows taller.
- **Preview (milestone 4):** `material/preview.rs` draws a CPU-lit ball and a flat 2 × 2 tiling from the box's picture. The model, a cube and the app's looks (unlit, lit, toon, realistic) are milestone 4's; so are the seam check and the texture budget. `pictures` works out at 256 while anything is held down and 512 after (`pictures::full`); tests use 32 and 64.
- **Linked copies and your own boxes (milestone 7):** Alt + C / V paste plain copies and say so; Alt + G / Shift + G only put a note in the status bar (`CanvasKey::OwnBox`, `OpenBox` in `canvas.rs`).
- **Not built:** the canvas's pan and zoom aren't saved; a frame's top edge doesn't resize (its title is there).

### What milestone 3 built, for the milestones after it

- **23 boxes** (decision 0259), listed in `matgraph::boxes` (its module doc and the crate README list them by group); the manual's `material-boxes.md` describes each. Milestone 1's ids are kept; Tiles gained a **Random** output and a **Seed**.
- **Sizes are centimetres:** `SettingKind::Length` and `Job::length_px`, through the graph's real-world size (Material out's `size_cm`, `eval::real_size_cm`). Milestone 4's preview and the painter (milestone 5) should use the same real-world size when laying the material on a model.
- **Gradients:** `SettingKind::Gradient` (`ColourGradient`, 2 to 8 colours), saved from file format 96, edited in Attributes by `box_rows::gradient_editor`. Starter graphs (milestone 8) can set one with `SettingValue::Gradient`.
- **The stone-tile graph** in `boxes/tests_each.rs` (`stone_tiles`) is a starting point for the Stone tiles starter graph: Tiles → Warp → Bevel → Blend (Noise, Add) → Gradient map and Material from height, Clouds → a moss Material from height, Mix materials by height (Rise −0.35) → Material out at 200 cm.
- **Speed** (`box_speed`, ignored; release, two cores, 2048 pixels): most boxes 30 to 400 ms, Noise and Planks about 1.1 s, the stone-tile graph about 4 s. If that is too slow, the GPU move (planned) or fewer noise layers in Planks' grain are the places to look.
- **Not built:** Transform and Repeat move heights only (colours can't be moved yet); gaps thinner than a pixel vanish at small sizes, so Bevel sees one shape at 32 pixels (the canvas's pictures are 256 and up).

### What milestone 4 built, for the milestones after it

- **The preview** (decision 0260): `ui/src/material/preview.rs`. Model (the first selected object with a surface), Ball and Cube (the Sphere and Cube shapes, `laying::ball` / `laying::cube`), Flat. The 3D shapes go through `ViewportRenderer::render_preview` (the app keeps a second `GpuViewport` for it) with a `ViewportView` from `preview_view`: a Painted surface in Unlit, Lit, Toon or Realistic (`PreviewLook`), no grid, the Studio backdrop, a fixed camera from the front right. The scene `Arc` is kept while nothing changes, so the renderer reuses its last picture.
- **Laying a material by its real-world size** (`material/laying.rs`): `repeats(mesh, transform, real_size_cm)` (texture repeats across the UV square, from `mesh::uvs::average_density` and the placement's scale) and `laid(value, repeats, size)` (the material repeated into a `PaintedTexture`: colour, and roughness / metal / height as the realism surface, decision 0211). **Milestone 5 (graphs in the painter)** and **milestone 6 (giving a material to a shape, baked export)** should lay materials the same way, so the preview and the result agree; `laid` is per texture square (all UV pieces share one repeat), which is what a shape given the material will show too.
- **Seam check:** `matgraph::SeamCheck` (`seams.rs`), with a plain-words `describe`. Pictures brought in from outside (a future image box) are what it will catch.
- **Texture budget:** `material/budget.rs` (`Budget::of(size, real_size_cm, export, aimed density)`); the maps come from `fileio::export::maps_going_out(MapsUsed, packing, glb)`, now the one rule the painter's export uses too. A graph's material counts as using roughness, metal and height, no glow. The chosen size (`Preview::budget_size`) isn't saved: **milestone 6** chooses the export's size and can start from it (`Fix::UseSize` still does nothing on the canvas).
- **Not built:** giving a graph's material to a shape (milestone 6, as planned; the preview lays it on the selected model without changing the scene); turning the preview's camera (it would add a mouse action, not in the agreed controls); the material's own normal map, occlusion and curvature on the 3D preview (the views bump by height, as painted height does).

### What milestone 5 built, for the milestones after it

- **Graph fills** (decision 0261): `Layer::graph` (`paint::GraphFill`: graph id, the layer's own values, `made`), a made layer whose pixels are ordinary paint. `ui/src/workbench/paint_graphs.rs` makes them: whenever the scene or drawing changes and nothing is held down, a layer whose `made` differs from `made_key` (the graph with the layer's values, texture size, layout key, repeats, realism) is made on another thread (`bake::model_maps`, then `bake::graph_fill`), one at a time, joining the change's undo step only if nothing changed meanwhile. Undo needs nothing special: a layer undone carries its old hash.
- **Running a graph on a model** (`matgraph::model`): `Evaluator::run_on_model` / `material_on_model` with an `OnModel` (the model's `ModelMaps` and the repeats). Boxes nothing from the model feeds make the repeat at `OnModel::tile_size`; the From the model boxes and what they feed work on the model's texture square, taking repeats laid across it (`Value::repeated`), told it covers `square_cm`. A graph with no From the model box comes out as its repeat laid across the square, the same as `material::laying::laid` lays it for the preview. **Milestone 6** (baked export, giving a graph's material to a shape) can use `material_on_model` with no maps (`ModelMaps::new(size, key)`) to lay a material on a shape, and `bake::model_maps` when the graph uses the model.
- **The From the model boxes** (`matgraph::boxes::from_model`, `Group::Model`): Curvature, Occlusion, Position, Thickness, Facing up. Without a model they give a stand-in of rounded bumps (`stand_in`); the Material mode's preview still shows the stand-in on the selected model (passing the model's maps there is a possible later step).
- **Shown in the painter:** `GraphBox::shown`, `GraphEdit::ShowInPainter`, `Graph::shown`, `Graph::with_settings`. **Milestone 7** (your own boxes, "choosing which settings become its sliders") can reuse the same idea and Attributes rows (`box_rows::shown_rows`).
- **Where to add them:** the Paint layers panel's ··· → + Graph fill (a menu of the project's graphs) and the shelf's Materials tab (`shelf_cards::graph_cards`, a dot on graphs that follow the model). The layer's settings (`paint_layers/graph_rows.rs`): Open in the graph (switches to the Material mode on it), making / why not, what of the model it follows, the shown settings with ↺.
- **File format 97:** `LayerV64::graph` (`GraphFillV97`) and `BoxV95::shown`; sample `tests/data/sample-v97.avtl`.
- **Not built:** the testing document wasn't updated from this session (see the pull request's How to test); a graph fill's repeats come from the first shape using the material; Warp and Blur on the model's square wrap round the texture's edges.

### What milestone 6 built, for the milestones after it

- **A material made by a graph** (decision 0262) is a texture set holding one graph fill and nothing else (`ui::material::giving::given_graph`); `with_graph_given` gives one (a painted set gets the graph as its bottom layer, under the paint). Everything else (making it on the model, following the graph, undo, saving, the texture export with every preset) is milestone 5's graph fills, unchanged. No file format change.
- **Where:** the Material mode's preview ends with **Use it** (`material/export_set.rs`): **Give to the selected shapes** (each selected shape's first material) and **Export textures…**; Attributes → Materials has **Graph…** per material (and **No graph** to take it back). The workbench side is `workbench/graph_giving.rs`.
- **A graph on its own** goes out through `fileio::export::graph_textures` (`texture_set`, `write_set`) at `MaterialEditor::budget_size` (the Texture size; `Fix::UseSize` now sets it), made for the Export window's Made for: the maps by `maps_going_out`, all beside, plus height (and occlusion with separate maps), the ORM or mask map with the graph's own occlusion, and a Godot `.tres` for Godot. **Milestone 14** (foliage output, alpha cards) adds an opacity map here: a `TextureMap` and a `MapsUsed` flag, with the engine presets' alpha settings in the `.tres`.
- **The hook for animated boxes (milestone 15):** set `BoxInfo::animated` on Time, Scroll, Pulse and Flow; `matgraph::bake_notes` then says, in the Material mode's export check and the Export window's **Material graphs** part, that they are baked at their current setting. The translators (later) can tell "translatable" boxes apart there.
- **Not built:** the occlusion part of a model export's ORM is still white for a material on a shape (a graph fill keeps no occlusion channel); a model export's normal map comes from the height, not the graph's own normal; the Material mode's Model preview still shows From the model boxes' stand-in rather than the selected model's maps; the Texture size isn't saved with the project.

