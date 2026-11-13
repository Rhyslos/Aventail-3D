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

### E. Extras the project lead chose (2026-11-12)

Picked from a list of ideas after milestone 8 ([decision 0269](../decisions/0269-material-extras.md)). Built after milestone 15, in this order, one pull request each. **Before any of them, a small one: 8b.**

8b. **Tiling, and Transform and Repeat on colours.**
   - A **Tiling** slider (×½, ×1, ×2, ×4…) on a material made by a graph (Attributes → Materials) and on a graph fill paint layer. It resizes the material on the model without opening the graph.
   - **Transform** and **Repeat** work on colours and whole materials as well as heights.
16. **Bypass and reroute dots.** Switch a box off for a moment (its input passes straight through) to compare with and without it. Click a wire to add a bend point and steer long wires round boxes.
17. **Find and minimap.** Search the graph for a box by name and jump to it; a small map in a corner of big graphs.
18. **Variations and the dice.** Save the shown sliders as named variations (Clean, Mossy, Ruined) and flip between them. A dice button rolls every seed for a fresh take.
19. **Project palettes.** A **Palette map** box colours greys with one of the project's palettes, so every material matches the game's colours.
20. **Paint inside a graph.** A **Painted** box you brush on in the Material editor (a mask, colour or height) for hand-placed cracks or moss, using the painter's brushes.
21. **Image to material.** Bring in a photo or a painting. The app makes it tile seamlessly, removes baked-in light, and guesses height and roughness from it.
22. **Stamp images.** Scatter your own small images (a rivet, a leaf, a rune) as stamps, as Scatter does with shapes.
23. **Stylised light boxes.** Toon bands, highlights painted in from a chosen light direction, and colour steps, for a hand-painted or cel look.
24. **Pixel-art output.** Pixelate to a chosen size and limit the colours to a palette, for pixel-art games.
25. **Weathering set.** Boxes for dirt in crevices, rust, edge wear, dust settling from above and water streaks, all following the model.
26. **Trim sheets.** Build a trim sheet: strips of different materials stacked on one texture, for walls, edges and pipes.
27. **Material health check.** Warns in plain words about colours too dark or bright to light well, detail that turns to mush at a distance, and pieces that repeat too obviously, each with a fix.
28. **Preview options.** Choose the preview's lighting and backdrop, compare two graphs side by side, and a before/after slider. (Turning the preview needs a mouse action: ask the project lead first.)
29. **Library browser.** One browser for all graphs and own boxes, with pictures, tags and search, and packs shared as one file.

Controls for these are asked of the project lead before each is built, unless the item above already says what they are (decision 0028).

### Done when (the gate)

The project lead makes, without help:
- a stylised stone-tile material, starting from an empty graph;
- a leaf atlas using the shape canvas, with randomised edges.

They use one of them in the painter, and see both look right in Godot (baked). The gate also includes one animated material, seen moving in the app's preview.

## Controls

**Chosen by the project lead on 2026-11-06:** the recommended choices below, except that wires are only removed by dragging their end away. Each is common in other node editors or follows the app's own keys, and none clashes with an existing key.

**Changed by the project lead on 2026-11-07: Tab is "back" in the Material editor.** Tab never leaves the Material mode (as in the texture painter); it backs out of your own box one level at a time, and otherwise says how to leave. Esc only cancels: the box search, a note, a window, a drag or a wire. **For milestone 11:** Tab leaves point editing and the shape canvas, back to the graph; Esc only cancels what is in progress there (a pen line, a drag).

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
| Back | **Tab** backs out of your own box, one level at a time; Tab never leaves the Material mode (changed 2026-11-07, see above) | Tab is the project lead's "back" key, as in the texture painter | Esc (the first choice) |

### Shape canvas

| Action | Chosen | Why | Alternatives |
|---|---|---|---|
| Select and move pieces | **Q** select, **W** move, **R** rotate, **S** scale, **T** all, with the same gizmos | The app's own tool keys (decision 0019) | V (Illustrator) |
| Pen | **P** draws points: click for a straight side, drag for a curve; click the first point to close | P is the pen in Illustrator, Inkscape, Figma and Affinity | — (in Paint mode P picks colour, but the canvas has its own keys) |
| Basic shapes | **B** opens the shape list (circle, square, polygon, star) | B already means new shape | — |
| Cut out | Hold **Alt** while drawing to make the shape a cut-out; or the Add / Cut switch on its bar | Alt for "take away" while drawing is common (Photoshop selections) | A Cut button only |
| Mirror line | **M** mirror on or off (the line can be moved) | Easy to remember; M (Weld) is a modelling key, not used on the canvas | A button only |
| Edit points | **Double-click** a shape to edit its points; **Tab** goes back (changed 2026-11-07, see above) | Double-click is common to vector apps; Tab is the project lead's "back" key | Esc (the first choice), Enter |

## Where things stand

**Standing OK from the project lead (2026-11-06): milestones 1 to 15 are built one after another, each pull request merged as soon as its checks pass.** They test everything at the end. Each milestone adds its test steps to the testing document.

| # | Milestone | PR | Decision | File format |
|---|---|---|---|---|
| 1 | Material graphs in the project | #280 | [0257](../decisions/0257-material-graphs-core.md) | 95 |
| 2 | The node canvas and the Material mode | #281 | [0258](../decisions/0258-node-canvas.md) | — |
| 3 | The first boxes | #282 | [0259](../decisions/0259-first-material-boxes.md) | 96 |
| 4 | Preview and checks | #283 | [0260](../decisions/0260-preview-and-checks.md) | — |
| 5 | Graphs in the painter | #284 | [0261](../decisions/0261-graphs-in-the-painter.md) | 97 |
| 6 | Baked export | #286 | [0262](../decisions/0262-baked-export.md) | — |
| 7 | Your own boxes | #287 | [0263](../decisions/0263-your-own-boxes.md) | 98 |
| 8 | Starter graphs | #288 | [0264](../decisions/0264-starter-graphs.md) | — |
| 8b | Tiling, and Transform and Repeat on colours | #298 | [0270](../decisions/0270-tiling-and-moving-colours.md) | 99 |
| 9 | Scatter | #299 | [0271](../decisions/0271-scatter.md) | — |
| 10 | More boxes | #300 | [0272](../decisions/0272-more-boxes.md) | — |
| 11 | The shape canvas | #301 | [0273](../decisions/0273-shape-canvas.md) | 100 |
| 12 | Assemblies | #PR | [0274](../decisions/0274-assemblies.md) | 101 |

Next decision: 0275. Next file format: 102.

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


### What milestone 7 built, for the milestones after it

- **Own boxes** (decision 0263): `matgraph::own`. A project's own boxes live in `GraphStore` beside its graphs; a placed one is a box whose kind is the own box's id (`own:` and hex), so all placed boxes of one are linked. **Use `document.scene().graphs().registry()` (the project's registry), never `Registry::standard()`, wherever a project's graph is checked or worked out**: the canvas (`MaterialEditor::registry` / `registry_of`), scene commands, graph fills, exports already do.
- **Making and opening up** are plain edits: `own::Proposal::of(graph, registry, selection)` then `make` (the `OwnBox` and the `GraphEdit`s), and `own::open_up`. The canvas runs them with `MaterialEditor::run_with(document, own_boxes, edits, …)`, one undo step. Inside an own box, the canvas's `current` is its inner graph and `run` sends `Command::EditOwnBox` (`Target`). **Starter graphs (milestone 8)** can ship own boxes (say "Stones from tiles") by adding them with `Command::SetOwnBox` before the graph.
- **Running:** `OwnBoxType` runs its inside with `Evaluator::run_given` (what is plugged in, the outer graph's seed and real-world size), so grouping never changes the picture; `Evaluator::run_inside` gives the pictures inside a placed box. Its cache key includes `BoxType::version`. From the model boxes and Material out can't go inside (refused with why); letting From the model boxes work inside is a possible later step.
- **Linked copies:** Alt + V pastes own boxes linked (the same kind), Ctrl + V makes a copy of its own (a new own box). **Milestone 12 (assemblies)** has its own linked copies of shape pieces; the same words ("linked: change one and every one changes") fit.
- **Shelf:** the Material layout now has the shelf (`Panel::Brushes`) under the canvas, showing `material::own_shelf` (this project's boxes and your library) while the Material mode is on. Your library (`MaterialEditor::library`, `BoxFile`s) is kept with the app's settings under `own_boxes`. Starter graphs could get a tab there.
- **Files:** format 98 (`project/v98.rs`, own boxes in `GraphsV95::own`); box files (`fileio::own_box_file`, `.matbox`, JSON). Sample `tests/data/sample-v98.avtl`.
- **Not built:** the testing document wasn't updated from this session (see the pull request's How to test); an own box opened from the shelf has nothing plugged in, so boxes inside needing an input say so.

### What milestone 8 built, for the milestones after it

- **Starter graphs** (decision 0264): nine (Bricks, Stone tiles, Cobbles, Planks, Stylised wood, Bark, Fabric, Metal plate, Ground), `fileio::starter_graphs::STARTERS`, each a graph file in `crates/fileio/starters/` with a 128-pixel card picture. **Leaves** is left for after milestone 13 (the edge randomiser), as planned: add it then as a tenth recipe (it can use the Shape box and own boxes), and drop the "Leaves comes later" line from the window (`material/starters.rs`) and the manual.
- **Changing or adding one:** edit or add a recipe in `fileio/src/starter_graphs/recipes/materials.rs` (`Recipe::add` by column, `wire`, `show`, `step` for each comment frame with its note and lane; `finish` lays it out), add it to `STARTERS`, then `UPDATE_STARTERS=1 cargo test -p fileio starter` writes the file and card. Look at it with `STARTER_RENDERS=<folder> cargo test -p fileio --release starter_renders -- --ignored` (colour, lit, height, two by two at 512). Quicker than `--release`: `--config 'profile.dev.package.matgraph.opt-level=3'`. **Milestones changing a box's maths** (10, more boxes, if it touches existing ones) will fail the card test: write the files again and look at the pictures.
- **Graph files** (`fileio::graph_file`, `.matgraph`): a graph and the own boxes it uses, fuzz-tested; `GraphFile::fitted_to` renames clashing own boxes when bringing one into a project. Export / bring in a graph file buttons aren't built; they need only the workbench's file actions (as box files have).
- **Frame notes on the canvas:** drawn under a frame's title, above its first box, from zoom 0.45 (`look::NOTE_*`). A frame's room for its note is just where its boxes start, so the tidy layout needs nothing new.
- **Not built:** a reference image of a starter on the canvas (the pictures are worked out on another thread, so a snapshot could catch them half done); own boxes inside starters (supported by the files, none used yet: "Stones from tiles" could be one).

### What milestone 8b built, for the milestones after it

- **Tiling** (decision 0270): `paint::GraphFill::tiling` (⅛ to 32, `usable_tiling`), set by the slider in `ui::material::tiling` (`control`, snapping to doublings, `shown` "×½", `parsed`), shown in Attributes → Materials under "Made by the graph" (`attributes/materials.rs`, `tiling_row`) and in a graph fill's settings (`paint_layers/graph_rows.rs`). It lays the repeat more often: `matgraph::OnModel { repeats, tiling }` (`OnModel::new(..).with_tiling(..)`): `repeats()` includes it, `square_cm` doesn't, so **From the model boxes and what they feed keep the model's size** (lengths there stay real centimetres) and only the repeating parts shrink. `bake::graph_fill(…, repeats, tiling)`; the made key hashes both. The preview's Model lays the selected model's material at its Tiling (`giving::tiling_on`, `laying::tiled`, which also clamps the painter's product to the preview's limits), so they agree. Ball, Cube, Flat and Export textures… are at ×1. File format 99 (`project/v99.rs`; `v97::GraphFillV97::tiling`, 1 by default). **Milestone 18 (variations)** may want Tiling in a variation; **milestone 26 (trim sheets)** will need its own idea of size per strip.
- **Boxes on any kind:** `boxes::moving` moves a height, a colour or a whole material by a per-pixel `Source` (place and scale) and turns a material's normals by a given rule. Transform and Repeat use it: inputs `in` (Height), `colour`, `material`; outputs `out`, `colour_out`, `material_out` (released keys kept; the new plugs are added after them). `BoxType::run_outputs` lets a box leave an output empty (`BoxResult::outputs` is `Vec<Option<Value>>`; a box wired to an empty output is refused with why) and `BoxType::needs_an_input` makes a box with no single needed input refuse when nothing is plugged in (with fixes) and keeps it out of `makers_of`. **Other boxes that should work on any kind** (Blur, Warp, Scatter in milestone 9, Stamp images in 22) can do the same with `moving::plugs` / `moved_all`, or their own `run_outputs`. A plug whose kind follows its input was considered and left: every place reading a plug's kind would need the graph.
- **Starters:** Transform is taller now, so the Bark starter's layout moved (its card is unchanged). Any milestone adding plugs to a box used in a starter will see the same: `UPDATE_STARTERS=1` and check the cards didn't change.
- **Not built:** Tiling on the ball, cube and flat previews (they show the material at ×1); a different Tiling across and down; Tiling for the graph's own Export textures… (a seamless set is one repeat by definition).

### What milestone 9 built, for the milestones after it

- **Scatter** (decision 0271): `matgraph::boxes::scatter`, the first box in `Group::Detail`. Inputs `shape` (its greys are each copy's height), `colour`, `material`, `where` (a shape: copies land by its grey at their middles); outputs `material_out` and `colour_out` (only when plugged in, `run_outputs`), `out` (height), `copies` (where any copy is), `random` (a grey per copy, 0.1 to 1). With nothing plugged in it places round pebbles (a dome), so it is a maker too. Sizes are lengths in centimetres (Spacing, Size); Size varies only shrinks.
- **Reusable parts** (`scatter/placing.rs`): `place(&Plan, where)` gives the copies (`Placed`: middle, size, turn's cos and sin, height, tint, grey, rank) on one core from the seed, at random (one per Spacing squared) or in a grid fitted to whole rows (Loose, Shift rows making rows even), with Keep apart (wrapped distances through a cell grid). `Lookup::new(&copies)` / `near(x, y)` lists the copies reaching each cell, with their middles moved by whole textures over the edges. **Milestone 22 (stamp images)** can use both and the copy-on-top rule (`hits` in `scatter.rs`) with an image for each copy; **milestone 13 (vary copies)** may borrow the per-copy tint and turn. Don't change the order random numbers are drawn from the seed: every graph's look depends on it.
- **Limits:** `MAX_COPIES` 16 384, `MAX_GRID` 128 across, `MAX_DEPTH` 32 (copies' areas never add up to more than 32 textures; the rest are left out). Speed at 2048 on two cores (`scatter_speed`, ignored, release): pebbles 0.13 s, a material 0.63 s, scales 0.35 s, 16 384 grass copies 1.9 s, the deepest pile 3.8 s. The deep piles are where a GPU move would help.
- **Starters:** the Ground starter's pebbles are a Scatter now (Spacing 20 cm, Size 17 cm, Size varies 0.5, Keep apart; Random into the pebble colours; Warp after). Its card changed on purpose; the other cards didn't.
- **Not built:** copies taking their colour from a picture under them (a "pick up the ground colour" option); copies scaled by the Where shape's grey instead of thinned out; a copy blending into the ones under its soft edge (it fades into the gap colour instead, one pixel wide); a different Spacing across and down.

### What milestone 10 built, for the milestones after it

- **Eight boxes** (decision 0272), one file each in `matgraph::boxes`: **Detail:** Cracks, Chips, Scratches, Fill the gaps; **Colour:** Brush strokes, Edge highlight, Directional light; **Shape:** Slope blur. 37 standard boxes. Their ids, plugs and settings are listed in the decision; tests in `boxes/tests_more.rs` (plus every shared check in `tests_each.rs`), `more_box_speed` (ignored).
- **Shared parts:** `matgraph::light` (`sun`, `lit`, `warmed`) is the painter's light-and-shade maths, now called by `paint::Lighting` too: **milestone 23 (stylised light boxes)** should build on it (Softness 0 already gives a hard toon line) rather than copy it. `scatter::placing` is `pub(super)` and used by Scratches and Brush strokes (each with its own `Plan`); **milestone 22 (stamp images)** can do the same. `pattern::whole_repeats` (a length in pixels to a whole number of pieces across, so it tiles) and `pattern::smooth`. Warp's `slope` and `steepest` are shared with Slope blur.
- **Kept general for later milestones:** wear here works on the texture's own height. **Milestone 25 (weathering set)** follows the model: it can combine the From the model boxes with Fill the gaps (crevices), Edge highlight's idea (edge wear) and Cracks, and should keep its own boxes for rust, dust from above and water streaks.
- **Boxes offered between two plugs** (`graph::connect::boxes_between`) now leave out boxes that take the wanted kind themselves (they only pass it on): any later box with a material in and out (as these have) won't crowd Material from height out of the three offered. The evaluator's "nothing plugged in" note names each kind once.
- **Starters:** the Metal plate starter has faint Scratches (its card changed, looked at); the others are unchanged. Brush strokes, Edge highlight or Directional light could give the Stylised wood or Stone tiles starters a more hand-painted look; that was left, as the cards already read well and the project lead hasn't seen these boxes yet.
- **Speed** at 2048 on two cores (matgraph optimised): Scratches 0.12 s, Edge highlight 0.25 s, Brush strokes 0.6 s, Chips 0.8 s, Directional light 1.4 s, Fill the gaps 1.9 s, Cracks 3.1 s, Slope blur with all three inputs 6.1 s. Slope blur and Cracks are the candidates for the GPU move.
- **Not built:** Slope blur follows a slope only in straight steps from each pixel (no path that bends as the slope does); Directional light's shadows look at most 160 pixels along; Brush strokes repaint colour only (a material's height, roughness and normals are kept); Chips' sides are straight-edged cells with a little noise, no rounder look.

### What milestone 11 built, for the milestones after it

- **The crate:** `crates/shapes` (layer 1, on `kurbo` and `tiny-skia`; its README lists every part). A `Drawing` holds `Piece`s in order (id never reused, name, `Path`, `PieceKind` Add / Cut / Line, `mirrored`), the `Mirror` line (upright, on or off, `x`) and a `Reference` image; every change is a checked `with_…` returning a new drawing or a `Problem`. `raster::mask` and `raster::lines` fill it at any size, wrapping round the edges (decision 0273: shapes crossing an edge come back on the other side, so the box tiles).
- **In the graph:** the **Shape** box (`matgraph::boxes::drawn_shape`, id `drawn_shape`, outputs `mask`, `height`, `outline`; settings `puff`, `height`, `outline`). The drawing is `GraphBox::drawing`, changed only by `GraphEdit::SetDrawing`, accepted only by kinds whose `BoxType::takes_drawing`; `Job::drawing` gives it to the box; its `key` (not the reference image) is in the cache key. `GraphBox::copy_edits` copies settings and drawing (pasting, opening an own box back up): **any later per-box data should go there too.**
- **The canvas** (`ui::material::shape_canvas`): `MaterialEditor::shape` (`ShapeCanvas`) replaces the node canvas while open; every change is `set_drawing` (one `SetDrawing` through `run`, drags under one merge key made again each frame from the drawing as it was, Esc abandons). Keys are taken by `take_shape_keys` before the node canvas's. Tab is `shape_step_back`. Workflow tests in `workbench/material_shape_tests.rs` show how to drive it (`canvas_page`, `at`, `pen_triangle`).
- **Files:** format 100 (`project/v100.rs`, `BoxV95::drawing`), sample `tests/data/sample-v100.avtl`; reference images through `fileio::image_file::read_reference` (at most 1024 pixels each way).
- **For milestone 12 (assemblies):** give `Piece` a placement (a `path::Change`-like transform kept apart from its points) and, for linked copies, a source piece id; `Drawing::drawn_paths` is the one place that turns a piece into the paths filled, so placing and linking go there and both `raster` and the canvas's outlines follow. Per-piece colour, height and order are new `Piece` fields (with `v100`'s model growing a new file format). The canvas's gizmos already move, turn and size several pieces; grouping and flipping are new. Plain copy/paste of pieces isn't built.
- **For milestone 13 (edge randomiser, veins):** an edge setting per piece can change the path before filling inside `drawn_paths` (keep `Drawing::key` covering it). Line pieces exist (drawn into the Outline now): veins can read them.
- **For milestone 14 (foliage output):** the Mask is the opacity; the Height is ready for normals; wrapping may want to be switched off for atlas cards (the box always wraps now).
- **Keys:** the plan's, plus **Enter** (finish an open pen line: the note offers Close it / Keep it as a line / Throw it away) and **Delete** (the chosen pieces, or the point while editing), the common choices, said at the top of the pull request. Polygon sides are set in the basic shapes list (no Ctrl + wheel: the list isn't a tool in use).
- **Not built:** a level mirror line (only upright); snapping to a grid; copy and paste of pieces; the reference image moved or scaled (it fills the texture square); the canvas's pan and zoom aren't saved.

### What milestone 12 built, for the milestones after it

- **Placed pieces** (decision 0274): `shapes::Piece` keeps its shape (`path`) apart from its `place` (a `Change` about the corner; `Change::then`, `inverse`, `is_usable`). Moving, turning, sizing and flipping change the place (`Drawing::with_change`, `flipped`); `Piece::placed` is the shape where it sits, and `Drawing::drawn_paths` (still the one place a piece becomes what is filled) uses it, the mirror line applied after. **Editing points works on the placed path**: `Drawing::with_path(id, path_on_the_canvas)` turns it back through the place; `with_shape` sets the shape itself. Anything reading a piece's points for the canvas must use `placed()`, not `path`.
- **Linked copies:** pieces with the same `LinkId` share their shape and kind; `with_path`, `with_shape` and `with_kind` change every one. `copied` / `with_pasted(copies, linked, by)` (`PieceCopies`) make copies; `with_unlinked`, `linked_count`. `from_parts` refuses linked pieces whose shapes differ. **Milestone 13 (vary copies)**: give `Piece` a seed (saved beside `colour` in `v100::PieceV100`, a new file format) and vary each piece's path inside `drawn_paths` after the place, so linked copies share the shape but come out different; the edge setting belongs with the shape (shared, changed through `changing_linked`). Keep `Drawing::key` covering anything new.
- **Groups** (`GroupId`, `Group`, one level): a group's pieces are always together in the order (`check_groups`); `with_group`, `without_group`, `with_group_name`, `with_order` (past whole groups; within a group for a grouped piece), `with_group_order`, `group_pieces`.
- **Colours and heights:** `Piece::colour` (sRGB, white at first) and `height` (0–1); `raster::layers` lays mask, colours and heights in one pass. The Shape box's fourth output **Colours** (`colours`); its Height is the puffed shape times the top piece's height (the puff follows the outside of all pieces together, not each piece: one distance field). `drawn_shape::drawn` returns `Drawn`. **Milestone 14** can take the opacity from `Drawn::mask` and a leaf colour from `Drawn::colours`.
- **The canvas** (`ui::material::shape_canvas`): `assembly.rs` (copy, paste plain / linked at the pointer, group, ungroup, flip, colour and height of the selection, unlink), `list.rs` (the pieces list with groups as headings, linked copies marked, the selection's rows). Keys: the material page's copy, paste and group shortcuts (Ctrl / Alt + C, Ctrl + V, Alt + V, Alt + G, Shift + G) are taken by `take_shape_keys` while the canvas is open. A click on a grouped piece on the canvas selects its group (`clicked_pieces`); the list selects single pieces. `MaterialEditor::pieces_clipboard` is shared by every tab. Workflow tests in `workbench/material_assembly_tests.rs` (helpers from `material_shape_tests.rs` are `pub(super)` now).
- **Flip** has no key (the plan named none, and the keys other apps use for it, Shift + H and the like, already show and hide objects here): **Flip ↔ / Flip ↕** buttons in the pieces list. A key can be added later if the project lead wants one.
- **Files:** format 101 (`project/v101.rs`; new fields on `v100::PieceV100` and `DrawingV100::groups`), sample `tests/data/sample-v101.avtl`.
- **Not built:** nested groups; linking pieces across two Shape boxes (copies pasted linked onto another canvas are linked to each other only); a per-piece puff (overlapping pieces of different heights meet in a step); snapping copies to a stem or path (a twig is placed by hand).
