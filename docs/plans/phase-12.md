# Phase 12 plan: materials with a node graph

The working plan for Phase 12 ([decision 0256](../decisions/0256-material-graph-plan.md); the roadmap is in [DESIGN.md](../../DESIGN.md#phase-12--materials)). It was proposed to the project lead on 2026-11-06, after research into why Substance Designer is hard to learn and how friendlier tools (Substance 3D Sampler, Quixel Mixer, InstaMAT, Material Maker) give the same power. It records what was chosen, the architecture, and a plan for every milestone. Update it as milestones are built.

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
| No sense of real size, so materials repeat badly | Every graph has a **real-world size** (how many centimetres one repeat covers) and **tiles seamlessly** by default |

## Choices proposed (to be confirmed by the project lead)

- **Where it lives:** a **Material** mode in the left palette, after Paint, like the painter's mode. The node canvas takes the main area. A **preview** sits beside it (on the selected model, a ball, or a flat tiled square) and shows the 2D texture tiled. The chosen box's settings go in **Attributes**, as everywhere in the app.
- **Controls** are listed under [Controls](#controls); they need the project lead's OK before milestone 2 is built ([decision 0028](../decisions/0028-ask-about-controls.md)).
- **Hand-painted first, realism after**, as in the painter.

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
- **Translated:** a graph with animated parts (time, scrolling, pulsing) or sliders marked "change in the game" is translated into a **Godot shader** (`.gdshader`) plus a material that uses it. Anything that doesn't change is baked into textures the shader reads, so the shader stays small. Only boxes marked "translatable" can stay live; the export check says which ones were baked instead. Unity and Unreal translators come later, outside this phase.

## Milestones

### A. The core

1. **Material graphs in the project.** The `matgraph` crate: boxes, plugs, wires, the four value kinds, evaluation, seamless tiling, caching, seeds. Graphs saved in the project. No interface yet beyond a placeholder list. (File format version bump.)
2. **The node canvas and the Material mode.**
   - The canvas: boxes with live pictures, coloured wires, plugs that refuse the wrong kind, and the app keeping the layout left to right.
   - The box search, the preview beside the canvas, and box settings in Attributes.
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
16. **The Godot translator.** On export, a graph with animated parts or in-game sliders becomes a `.gdshader` and a material. Everything that doesn't move is baked into textures for it. The export check lists what was baked and why, and it is tested by opening the result in Godot.

### Done when (the gate)

The project lead makes, without help:
- a stylised stone-tile material, starting from an empty graph;
- a leaf atlas using the shape canvas, with randomised edges.

They use one of them in the painter, and see both look right in Godot. The gate also includes one animated material that goes through the translator.

## Controls

Recommended choices. Each is common in other node editors or follows the app's own keys, and none clashes with an existing key. Nothing is final until the project lead confirms.

### Node canvas

| Action | Recommended | Why | Alternatives |
|---|---|---|---|
| Add a box | **Space** over the canvas opens a box search at the pointer; **right-click** on empty canvas gives the same list as a menu | Space already opens the command search; over the canvas it searches boxes instead. The right-click menu is the app's default (decision 0126) | Tab (Unreal/Blender-style; but Tab changes modes in the app), Shift + A (Blender) |
| Connect | **Drag** from a box's output dot to another box's input dot; drop on empty canvas to open the box search with matching boxes | What every node editor does; dropping on empty space is the quickest way to add the next box | — |
| Disconnect | **Drag a wire's end away** from its input, or **Ctrl + drag** across wires to cut them | The first is the most common; Ctrl-drag cutting is Blender's | Alt + click a wire (Substance) |
| Pan, zoom | **Middle drag** pans, **wheel** zooms, as in the UV Editor | Same as the rest of the app | — |
| Frame | **Z** frames the selected boxes (or all if none) | Z already frames the selection in the viewport | F (Blender, Substance; but F is Loop cut in the app) |
| Select | Click, drag a box around several, **Ctrl** adds, **Shift** removes | The app's rule (decision 0133) | — |
| Copy, paste, linked copy | **Ctrl + C / V**; **Alt + C / V** pastes a linked copy | Same as objects (decision 0055) | Ctrl + D duplicate (but D is Extrude) |
| Delete | **Delete** removes boxes; **Backspace** removes a box but keeps the wire through it | Delete and Backspace already mean delete and dissolve in the app | — |
| Make your own box | **Alt + G** groups the selected boxes into one; **Shift + G** opens it back up | Alt + G / Shift + G already group and ungroup objects | Ctrl + G (Blender) |
| Look through a box | **Click a box's picture** to show that box in the preview | Substance and Material Maker show any node in a 2D view; a click is the easiest | Double-click |

### Shape canvas

| Action | Recommended | Why | Alternatives |
|---|---|---|---|
| Select and move pieces | **Q** select, **W** move, **R** rotate, **S** scale, **T** all, with the same gizmos | The app's own tool keys (decision 0019) | V (Illustrator) |
| Pen | **P** draws points: click for a straight side, drag for a curve; click the first point to close | P is the pen in Illustrator, Inkscape, Figma and Affinity | — (in Paint mode P picks colour, but the canvas has its own keys) |
| Basic shapes | **B** opens the shape list (circle, square, polygon, star) | B already means new shape | — |
| Cut out | Hold **Alt** while drawing to make the shape a cut-out; or the Add / Cut switch on its bar | Alt for "take away" while drawing is common (Photoshop selections) | A Cut button only |
| Mirror line | **M** mirror on or off (the line can be moved) | Easy to remember; M (Weld) is a modelling key, not used on the canvas | A button only |
| Edit points | **Double-click** a shape to edit its points; Esc goes back | Common to vector apps | Enter |

## Where things stand

Proposed, not built. Next decision: 0257. Next file format: 95.
