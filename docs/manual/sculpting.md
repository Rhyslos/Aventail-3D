# Sculpting

> Shape a model with brushes, as if it were clay: push the surface out, dig into it and smooth it, in Sculpt mode.

## When to use it

[Sculpting](concepts.md#sculpting) is for organic, soft and stylised forms that are slow to build point by point: a creature's muscles, a tree stump's roots, a rock's chunky lumps, the folds of a mushroom cap. Modelling (the [Points, Edges and Polygons modes](modes.md)) is better for anything exact: walls, planks, boxes, a crate's edges. Many stylised game artists do both: model a rough shape, then sculpt its forms.

Sculpting needs polygons to push around: a cube of six faces barely changes. Start from a shape with plenty of points, such as a sphere with more sides and rings, a subdivided shape (bake its Subdivide modifier in), or the **Remesh** modifier baked in ([Remesh](remesh.md)). A remesh made for sculpting arrives with a later step of sculpting.

What you sculpt is kept in a **Sculpt layer** of the shape's [edit layers](edit-layers.md), so everything under it stays editable: give a sphere a bigger radius later and your sculpting rides along on the bigger sphere.

## How to use it

1. Select a shape, then click the **Sculpt** button in the mode palette on the left (between Polygons and UV: a lump of clay), or the **Sculpt** layout button on the right of the menu bar. The window switches to the **Sculpt layout**: the brush bar floats over the top of the view, the **Shelf** of brushes is under it, the **Sculpt** tab sits in front of Attributes, and **Edit layers** beside Objects.
2. Press the left button over the shape and drag. The **Draw** brush pushes the surface out where you stroke. The circle shows the brush's size.
3. **Hold Ctrl** while you stroke to dig in instead. **Hold Shift** to smooth: bumps and dents even out, while the big shape stays.
4. Change the brush's size with **[** and **]**, or hold **Ctrl** and drag with the right button: sideways for size, up and down for strength.
5. Every stroke is one undo step (**Ctrl + Z**). The first stroke adds a layer called **Sculpt** to the shape's edit layers; the next strokes go into the same layer.
6. Press **Esc** to leave Sculpt mode (if a stroke is under way, Esc takes it back instead and you stay). Or pick another mode or layout.

While you sculpt, the shape is shown as clay (a [matcap](concepts.md#matcap)) with [cavity](concepts.md#cavity) shading, and no edges, so the forms read clearly. Only the view changes: your materials and paint are untouched.

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

The brush bar and the **Sculpt** tab hold the same settings. Each brush keeps its own size and strength.

| Setting | What it does |
|---|---|
| Brush | **Draw** pushes the surface out along the way it faces under the brush (Ctrl: in). **Smooth** evens the surface out; holding Shift smooths with any brush. |
| Size | The brush's radius on screen, in pixels. Zooming in makes it smaller on the shape, for detail. |
| Strength | How much each dab changes the surface. 30–60% is a good start; lower for gentle shaping. |
| Spacing | How far apart the brush's dabs are along a stroke, as a share of its radius. 10% gives smooth strokes. |
| Falloff | How the strength fades from the middle to the rim: **Smooth** for soft forms, **Sharp** and **Spike** for creases and points, **Flat** for an even push. |
| Look (Sculpt tab) | **Clay**, **Red wax** (glossy, shows small detail) or **Pale stone** (matte), or **Lit**, the app's usual look with your colours. |
| Cavity (Sculpt tab) | Darkens creases and lights ridges. On as standard. |

The **Shelf** under the view shows each brush with what it does; click one to choose it.

## Shortcuts

| Action | Shortcut |
|---|---|
| Sculpt | Left drag |
| Dig in instead of building up | Hold `Ctrl` while dragging |
| Smooth, with any brush | Hold `Shift` while dragging |
| The Draw brush | `1` |
| Brush smaller / bigger | `[` / `]` |
| Brush size (sideways) and strength (up and down) | `Ctrl` + right drag |
| Take back the stroke under way; otherwise leave Sculpt mode | `Esc` |
| (Stays in Sculpt mode) | `Tab` |

Keys 2 to 9 are kept for the sculpting brushes still to come: in Sculpt mode they don't switch to the Edges or Polygons mode. The keys are the plan's recommended choices, still to be reviewed.

## Tips and common problems

- **Problem:** a stroke barely changes the shape, or makes it spiky. **Why:** the shape has too few points; sculpting moves points, it doesn't add them. **Fix:** give the shape more polygons first (more sides and rings, or a baked Subdivide or Remesh).
- **Problem:** nothing happens when you press on a shape. **Why:** Sculpt mode works on the selected shape (the first one that can be sculpted). **Fix:** pick the shape in the Objects panel. Groups, instances and curves can't be sculpted; make an instance or curve editable first.
- **Problem:** the brush circle doesn't show. **Why:** Move, Rotate or Scale (W, R, S) is in use. **Fix:** press **1** or **Q**.
- **Problem:** "Getting … ready to sculpt" in the Sculpt tab. **Why:** a heavy shape (hundreds of thousands of polygons or more) is made ready on another thread so the window never freezes. **Fix:** wait a moment; it takes a second or two for millions of polygons.
- On heavy shapes (see [Heavy shapes](heavy-shapes.md)), strokes stay smooth because only the part of the shape the brush touches is worked out and sent to the graphics card.

## Related

- [Edit layers](edit-layers.md), where the Sculpt layer lives
- [Remesh](remesh.md), [Subdivide](subdivide.md), for enough polygons to sculpt
- [Concepts](concepts.md): [sculpting](concepts.md#sculpting), [brush strength and falloff](concepts.md#brush-strength-and-falloff), [matcap](concepts.md#matcap), [Sculpt layer](concepts.md#sculpt-layer)
