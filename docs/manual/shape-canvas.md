# The shape canvas

> Draw your own shapes (a leaf, a blade of grass, a rune) inside a material graph, and use them as a sharp mask, a rounded height and an outline at any texture size.

## When to use it

When a material needs a shape the pattern boxes can't make: leaves for foliage, a blade, a badge, a rune carved into stone. You draw it once on the **Shape** box's canvas, and the rest of the graph uses it like any other box: plug its **Height** into **Material from height** to make it a material, its **Mask** into **Scatter** to scatter copies, its **Outline** into **Blend** to draw lines.

The shape is a [vector shape](concepts.md#vector): points joined by straight and curved sides. It is turned into pixels only when the texture is made, at whatever size the texture is, so it is never blurry, even at 4096 pixels.

For simple round or pointed shapes in a grid (studs, rivets), the **Basic shape** box is quicker. Leaves made of many pieces, linked copies and randomised edges come in later versions.

## How to use it

1. In the **Material** mode, add a **Shape** box (Space over the canvas, type *Shape*; it is under **Make**).
2. **Double-click** it, or choose it and click **Open the canvas** in Attributes. The shape canvas takes the graph's place: the texture square in the middle, the **pieces list** on the right, and a strip along the bottom showing what the box gives.
3. Draw (see below). Each change shows straight away in the strip and in the box's picture.
4. Press **Tab** (or click **Back to the graph (Tab)**) to go back to the graph. Double-click the box any time to change the shape.

### Drawing with the pen

1. Press **P** for the pen.
2. **Click** for a corner. **Click and drag** for a curve: the drag pulls out the point's two handles, which bend the sides either side of it.
3. Go round the shape, then **click the first point** (it has a ring round it) to close it. It becomes a piece, filled.
4. **Esc** throws away a line you are drawing. **Enter** finishes it without closing: a line that isn't closed has no inside, so the app asks what you meant: **Close it**, **Keep it as a line** (lines are drawn in the Outline; later versions turn them into leaf veins), or **Throw it away**.

### Basic shapes

Press **B** (or **Basic shapes** on the bar) for a list: **Circle**, **Square**, **Polygon** and **Star**. **Sides** sets a polygon's sides or a star's points. The shape is placed where the list opened, and the **All** tool (T) is picked so you can move and size it straight away. Basic shapes are ordinary pieces: double-click one to edit its points.

### Cut-outs

A [cut-out](concepts.md#cut-out) takes its inside away from the pieces under it: a bite out of a leaf, a hole in a plate. Hold **Alt** when you place the first point (or choose a basic shape), or switch **Add / Cut** on the bar to **Cut** for everything you draw. In the pieces list, **Does** changes a piece between **Add**, **Cut out** and **Line** later.

Pieces are laid from the bottom of the list to the top: a cut-out takes away only from the pieces under it, so a piece added over it is whole again. **Up** and **Down** change the order.

### The mirror line

Press **M** (or **Mirror line** on the bar) for the [mirror line](concepts.md#mirror-line): a dashed green upright line. Every piece is drawn on both sides of it, so you draw half a leaf against the line and get a whole one. Pen points close to the line snap onto it, so the two halves meet exactly. **Drag the line** (with any tool but the pen) to move it. A piece you don't want mirrored (a bite on one side only) can have **Mirrored** turned off in the pieces list.

### A reference image

Under **Reference image** in the pieces list, **Reference image…** brings in a picture (PNG, JPEG, TGA or BMP): a photo of a leaf, a sketch. It sits under the canvas, stretched over the texture square; **Opacity** sets how strongly it shows. It is only a guide: it never changes what the box gives. It is saved with the project (made smaller to 1024 pixels at most, so projects stay small); **Remove image** takes it away.

### Moving, turning and sizing pieces

- **Q** select: click a piece to select it (**Ctrl** adds, **Shift** takes away), drag round pieces to select several, or click them in the pieces list. Dragging a piece moves it.
- **W** move, **R** rotate, **S** scale, **T** all: the same keys as in the viewport, with a gizmo round the selected pieces. Drag anywhere to move, turn or size them; drag the red arrow or square for across only, the blue one for up and down only; with **T**, drag the ring to turn.
- **Delete** removes the selected pieces.

### Editing points

**Double-click** a piece (or **Edit points** in the pieces list): its points (squares) and handles (dots) show.
- Drag a point to move it, or a handle to bend the sides next to it. A smooth point's two handles stay in line.
- **Double-click a point** to switch it between a corner and a smooth point.
- **Double-click a side** to add a point there; the shape doesn't change until you move it.
- Click a point to choose it, and **Delete** removes it.
- **Tab** leaves editing points; Tab again goes back to the graph.

## What the box gives

| Output | What it is |
|---|---|
| **Mask** | The shape, white inside and black outside, with smooth edges, at any texture size. |
| **Height** | The shape puffed up: rising from its edge to its full height **Puff** centimetres in, rounded, like a cushion. |
| **Outline** | A line along the shape's edge, **Outline** centimetres wide, with the lines you drew. |

A shape crossing an edge of the texture square comes back in on the other side, so the texture still [tiles seamlessly](concepts.md#seamless-tiling); the canvas shows the copies round the square faintly. To keep a leaf whole, keep it inside the square.

## Settings

The Shape box's settings are in Attributes, as for every box.

| Setting | What it does |
|---|---|
| **Puff** | How far in from the edge the height rises to its top, in centimetres on the finished material. More makes the shape rounder; 0 gives a flat top with straight sides. For a leaf about 10 cm long, 1 to 2 cm. |
| **Height** | How high the top of the puffed-up shape is: 1 is white. |
| **Outline** | How wide the outline and drawn lines are, in centimetres. |

## Shortcuts

With the mouse over the shape canvas:

| Action | Shortcut |
|---|---|
| Select, move, rotate, scale, all | `Q`, `W`, `R`, `S`, `T` |
| The pen | `P` |
| Basic shapes | `B` |
| Make what you draw a cut-out | Hold `Alt` while placing its first point |
| Mirror line on or off | `M` |
| Edit a piece's points | Double-click it |
| Finish a line without closing it | `Enter` |
| Remove the selected pieces (or the chosen point) | `Delete` |
| Cancel the line being drawn, or a drag | `Esc` |
| Leave editing points; back to the graph | `Tab` |
| Pan, zoom | Middle drag, wheel |
| Undo | `Ctrl + Z` |

## Tips and common problems

- **Problem:** the app says an open path can't be filled. **Why:** a line has an inside only once its last point joins its first. **Fix:** click **Close it**, or next time click the first point to finish.
- **Problem:** the app says a shape has no room inside. **Why:** its points are all on one line (or a point was dragged onto the others). **Fix:** move a point off the line; Ctrl + Z undoes the last change.
- **Problem:** part of the shape appears on the other side of the texture. **Why:** it crosses the square's edge, and textures wrap round so they tile. **Fix:** move it or make it smaller to keep it inside the square.
- **Problem:** the two halves of a mirrored leaf don't meet. **Why:** the points along the middle are off the mirror line. **Fix:** drag them onto it (they snap when close), or start the pen on the line.
- **Tip:** the strip along the bottom is the box's own result with its settings, so changing **Puff** in Attributes shows there at once.
- **Tip:** a leaf: turn the mirror line on, draw half the outline from the stem to the tip with a few dragged (curved) points, starting and ending on the line, and close it. Add a vein with the pen and **Keep it as a line**.

## Related

- [Material boxes](material-boxes.md): every other box, and how the Shape box's outputs fit them.
- [Material graphs](material-graphs.md): the Material mode, wires and undo.
- [Concepts](concepts.md): [vector shape](concepts.md#vector), [mirror line](concepts.md#mirror-line), [cut-out](concepts.md#cut-out), [seamless tiling](concepts.md#seamless-tiling).
