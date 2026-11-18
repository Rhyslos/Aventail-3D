# Sculpting

> Shape a model with brushes, as if it were clay: push the surface out, dig into it, grab and pull it, and smooth it, in Sculpt mode.

## When to use it

[Sculpting](concepts.md#sculpting) is for organic, soft and stylised forms that are slow to build point by point: a creature's muscles, a tree stump's roots, a rock's chunky lumps, the folds of a mushroom cap. Modelling (the [Points, Edges and Polygons modes](modes.md)) is better for anything exact: walls, planks, boxes, a crate's edges. Many stylised game artists do both: model a rough shape, then sculpt its forms.

Sculpting needs polygons to push around: a cube of six faces barely changes. The quickest way to get them is **[Remesh for sculpting](remesh-for-sculpting.md)** (**Ctrl + R** in Sculpt mode): one even surface at the size you choose, and any other shapes selected merged into it, so you can block a creature out of primitives and sculpt it as one. A sphere with more sides and rings, or a subdivided shape (bake its Subdivide modifier in), works too.

What you sculpt is kept in a **Sculpt layer** of the shape's [edit layers](edit-layers.md), so everything under it stays editable: give a sphere a bigger radius later and your sculpting rides along on the bigger sphere.

## How to use it

1. Select a shape, then click the **Sculpt** button in the mode palette on the left (between Polygons and UV: a lump of clay), or the **Sculpt** layout button on the right of the menu bar. The window switches to the **Sculpt layout**, arranged as the [Paint layout](painting-textures.md): on the right, **Edit layers** at the top (where the Sculpt layer shows, with **Objects** as a tab beside it), and the **Sculpt** tab with the brush's settings under them (with **Attributes** as a tab beside it); the **Shelf** of brushes under the view; the **brush bar** floating over the top of the view, and the **tool row** along its bottom (see [The Sculpt page](#the-sculpt-page) below).
2. Press the left button over the shape and drag. The **Draw** brush pushes the surface out where you stroke. The circle shows the brush's size. Pick another brush with the keys **1** to **7**, the shelf, or the tool menu (**E**): [Sculpt brushes](sculpt-brushes.md) describes them all.
3. **Hold Ctrl** while you stroke to dig in instead. **Hold Shift** to smooth: bumps and dents even out, while the big shape stays.
4. Change the brush's size with **[** and **]**, or hold **Ctrl** and drag with the right button: sideways for size, up and down for strength.
5. Every stroke is one undo step (**Ctrl + Z**). The first stroke adds a layer called **Sculpt** to the shape's edit layers; the next strokes go into the same layer.
6. Press **Esc** to leave Sculpt mode (if a stroke is under way, Esc takes it back instead and you stay). Or click another layout button at the top right.

While you sculpt, the shape is shown as clay (a [matcap](concepts.md#matcap)) with [cavity](concepts.md#cavity) shading, and no edges, so the forms read clearly. Only the view changes: your materials and paint are untouched.

### The Sculpt page

Sculpt mode's page is as plain as [Paint mode's](painting-textures.md): what you learn on one works on the other.

- **A plain page:** the modelling toolbar, the mode column and the modifiers column step aside, highlights turn a quiet slate grey-blue whatever your accent colour, the selection isn't outlined over the shape, the view shows no names or grid, and it stays one view (a middle click doesn't split it into four). What's left is the shape and what sculpts it. To see the grid, tick **Grid** in the tool row's **Show** list; it's remembered, and it's the same switch as the Paint page's. The layout buttons at the top right and **Esc** leave the page.
- **The brush bar**, over the top left of the view: the brush (click it for every brush, each with its picture), **Size**, **Strength** and **Spacing** as bars (drag sideways, or click to type), switches for **Symmetry** (with its axes; **L**), the **pen's pressure** and **smoothing** (**D**), and the **Falloff**, **Stroke** and **Alpha** lists. On a narrow view the bar takes two rows.
- **The tool row**, along the bottom left of the view. First the view's controls, as on the Paint page: **☰** (the view's menus), **3D** (which camera), the look (it names the [matcap](concepts.md#matcap): **Clay**, **Red wax**, **Pale stone** or **Lit**; its list holds them) and **Show** (**Grid**, **Cavity** and the other overlays; sculpts are drawn without edges). Then the tools: **Select** (`Q`), **Move** (`W`), **Rotate** (`R`), **Scale** (`S`), and the brushes with a key: **Draw** (`1`), **Clay strips** (`2`), **Inflate** (`3`), **Crease** (`4`), **Flatten** (`5`), **Grab** (`6`), **Snake hook** (`7`) and **Mask** (`9`). The tool in use is lit.
- **Select** (`Q`) puts the brush down: a click selects instead of sculpting, which is how you pick another shape to sculpt from the view. Pick a brush (its key or its button) to sculpt again. With **Move**, **Rotate** or **Scale** a drag moves, turns or sizes the selected shape with the gizmo (or only its unmasked part, see [Symmetry and masks](sculpt-symmetry-and-masks.md)).
- **Edit → Settings → Interface → Paint tools in the left bar** puts the tool buttons in the left bar instead, in Sculpt mode as in Paint mode.

### Shapes with booleans, a mirror or modifiers

[Booleans](booleans.md), the [live mirror](symmetry.md) and [modifiers](modifiers.md) are worked out after a shape's edits, so after anything you sculpt. The first time you sculpt such a shape, the app asks:

- **Bake them in** (recommended): they become part of the shape, in one undo step, and you sculpt exactly what you see. The shape's base settings (a primitive's size, say) are baked in too.
- **Sculpt under them:** they're hidden while you sculpt, and come back when you leave Sculpt mode. Handy for a Subdivide modifier you want to keep live.

Esc leaves Sculpt mode without choosing.

### Sculpt layers and other edits

- Edits you make in the modelling modes after sculpting go into a new **Edits** layer on top of the Sculpt layer. Sculpting after those goes into a new **Sculpt 2** layer on top, so each keeps its order.
- Switch the Sculpt layer's eye off to see the shape without your sculpting. A Sculpt layer can't be merged with another layer: it holds only its sculpting.
- If something under the sculpt changes its points (a primitive given more sides, say), the sculpting is fitted to the new shape by position: each point takes the sculpting of the nearest point it had. The layer's edit shows **fitted** in the Edit layers panel. Undo brings the old shape back exactly.
- Sculpt layers are saved with the project (projects saved with this version can't be opened by older versions).

## Settings

The brush bar holds the settings you change all the time; the **Sculpt** tab holds them all, in folding sections like the Paint tab's: **Brush** (open to start with), **Stroke and alpha**, **Symmetry**, **Mask**, **Remesh**, **Pen and smoothing**, **Look** and **Keys for sculpting**. A closed section says on its right what's set inside (*Mirror X*, *pressure → size*, *off*), so you can see at a glance without opening it; click its name to open or close it, and it stays as you left it while the app is open. Size, Strength and Spacing are on the brush bar, so the tab leaves them out. Each brush keeps its own size and strength.

| Setting | What it does |
|---|---|
| Brush | Which brush strokes make: **Draw**, **Smooth**, **Clay strips**, **Clay**, **Inflate**, **Crease**, **Pinch**, **Flatten**, **Scrape**, **Fill**, **Grab**, **Elastic grab**, **Snake hook** and **Mask** (see [Sculpt brushes](sculpt-brushes.md)). A list in the brush bar, buttons in the Sculpt tab's Brush section, and the ones with a key in the tool row. Holding Shift smooths with any brush. |
| Size | The brush's radius on screen, in pixels. Zooming in makes it smaller on the shape, for detail. |
| Strength | How much each dab changes the surface. 30–60% is a good start; lower for gentle shaping. |
| Spacing | How far apart the brush's dabs are along a stroke, as a share of its radius. 10% gives smooth strokes. |
| Falloff | How the strength fades from the middle to the rim: **Smooth** for soft forms, **Sharp** and **Spike** for creases and points, **Flat** for an even push. |
| Look (Sculpt tab, and the tool row's look button) | **Clay**, **Red wax** (glossy, shows small detail) or **Pale stone** (matte), or **Lit**, the app's usual look with your colours. |
| Cavity (Sculpt tab's Look, and the tool row's **Show** list) | Darkens creases and lights ridges. On as standard. |
| Stroke and alpha | **Freehand**, **Line** or **Drag a stamp**, and the alpha the brush stamps or drags, with its **Angle** and **Follow the stroke** (see [Sculpt brushes](sculpt-brushes.md#strokes-alphas-and-pens)). |
| Pen and smoothing (Sculpt tab; switches on the brush bar) | How a pen's pressure changes the size and strength, and smoothing (**D**). |
| Remesh (Sculpt tab; the voxel size and a **Remesh** button on the brush bar) | The **Voxel size**, **Keep detail** and **Fix holes** of [Remesh for sculpting](remesh-for-sculpting.md) (**Ctrl + R**). |

The **Shelf** under the view has two tabs: **Brushes** and **Alphas** (the buttons at its top left). The Brushes tab shows a card per brush with a picture of what it does (the surface from the side, before and after); the line above the cards says what the chosen brush does. Click a card to choose it.

## Shortcuts

| Action | Shortcut |
|---|---|
| Sculpt | Left drag |
| Dig in instead of building up | Hold `Ctrl` while dragging |
| Smooth, with any brush | Hold `Shift` while dragging |
| The Draw brush | `1` |
| Clay strips, Inflate, Crease, Flatten, Grab, Snake hook | `2`, `3`, `4`, `5`, `6`, `7` |
| Every brush, in a ring at the mouse | `E` |
| The select arrow: clicks select instead of sculpting | `Q` |
| Brush smaller / bigger | `[` / `]` |
| Smoothing on or off | `D` |
| Remesh for sculpting | `Ctrl + R` |
| Brush size (sideways) and strength (up and down) | `Ctrl` + right drag |
| Take back the stroke under way; otherwise leave Sculpt mode | `Esc` |
| (Stays in Sculpt mode) | `Tab` |

Key 9 is the Mask brush ([Symmetry and masks while sculpting](sculpt-symmetry-and-masks.md)) and key 8 is kept for the Trim brush still to come: in Sculpt mode none of the number keys switch to the Edges or Polygons mode. The keys are the plan's recommended choices, still to be reviewed.

## Tips and common problems

- **Problem:** a stroke barely changes the shape, or makes it spiky. **Why:** the shape has too few points; sculpting moves points, it doesn't add them. **Fix:** give the shape more polygons first: [Remesh for sculpting](remesh-for-sculpting.md) (**Ctrl + R**), more sides and rings, or a baked Subdivide.
- **Problem:** nothing happens when you press on a shape. **Why:** Sculpt mode works on the selected shape (the first one that can be sculpted). **Fix:** pick the shape in the Objects panel. Groups, instances and curves can't be sculpted; make an instance or curve editable first.
- **Problem:** the brush circle doesn't show, and clicks select instead of sculpting. **Why:** Select, Move, Rotate or Scale (Q, W, R, S) is in use. **Fix:** press a brush's key (**1** for Draw) or click a brush in the tool row.
- **Problem:** the toolbar and the mode buttons are gone. **Why:** Sculpt mode's page is plain, as Paint mode's. **Fix:** click **Standard** (or another layout) at the top right, or press **Esc**; the Mesh and Select menus and command search (**Space**) still work while sculpting.
- **Problem:** "Getting … ready to sculpt" in the Sculpt tab. **Why:** a heavy shape (hundreds of thousands of polygons or more) is made ready on another thread so the window never freezes. **Fix:** wait a moment; it takes a second or two for millions of polygons.
- On heavy shapes (see [Heavy shapes](heavy-shapes.md)), strokes stay smooth because only the part of the shape the brush touches is worked out and sent to the graphics card.

## Related

- [Sculpt brushes](sculpt-brushes.md), every brush and what it's for
- [Edit layers](edit-layers.md), where the Sculpt layer lives
- [Remesh for sculpting](remesh-for-sculpting.md), [Subdivide](subdivide.md), for enough polygons to sculpt
- [Concepts](concepts.md): [sculpting](concepts.md#sculpting), [brush strength and falloff](concepts.md#brush-strength-and-falloff), [matcap](concepts.md#matcap), [Sculpt layer](concepts.md#sculpt-layer)
