# The shape canvas

> Draw your own shapes (a leaf, a blade of grass, a rune) inside a material graph, build them into [assemblies](concepts.md#assembly) of coloured pieces and linked copies (a twig of leaves), and use them as a sharp mask, a rounded height, an outline and colours at any texture size.

## When to use it

When a material needs a shape the pattern boxes can't make: leaves for foliage, a blade, a badge, a rune carved into stone. You draw it once on the **Shape** box's canvas, and the rest of the graph uses it like any other box: plug its **Height** into **Material from height** to make it a material, its **Mask** into **Scatter** to scatter copies, its **Outline** into **Blend** to draw lines.

The shape is a [vector shape](concepts.md#vector): points joined by straight and curved sides. It is turned into pixels only when the texture is made, at whatever size the texture is, so it is never blurry, even at 4096 pixels.

For simple round or pointed shapes in a grid (studs, rivets), the **Basic shape** box is quicker. Randomised edges, so that no two leaves match, come in a later version.

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

Pieces are laid from the bottom of the list to the top: a cut-out takes away only from the pieces under it, so a piece added over it is whole again. **Up** and **Down** change the order (see [Assemblies](#assemblies-many-pieces)).

### The mirror line

Press **M** (or **Mirror line** on the bar) for the [mirror line](concepts.md#mirror-line): a dashed green upright line. Every piece is drawn on both sides of it, so you draw half a leaf against the line and get a whole one. Pen points close to the line snap onto it, so the two halves meet exactly. **Drag the line** (with any tool but the pen) to move it. A piece you don't want mirrored (a bite on one side only) can have **Mirrored** turned off in the pieces list.

### A reference image

Under **Reference image** in the pieces list, **Reference image…** brings in a picture (PNG, JPEG, TGA or BMP): a photo of a leaf, a sketch. It sits under the canvas, stretched over the texture square; **Opacity** sets how strongly it shows. It is only a guide: it never changes what the box gives. It is saved with the project (made smaller to 1024 pixels at most, so projects stay small); **Remove image** takes it away.

### Moving, turning and sizing pieces

- **Q** select: click a piece to select it (**Ctrl** adds, **Shift** takes away), drag round pieces to select several, or click them in the pieces list. Dragging a piece moves it.
- **W** move, **R** rotate, **S** scale, **T** all: the same keys as in the viewport, with a gizmo round the selected pieces. Drag anywhere to move, turn or size them; drag the red arrow or square for across only, the blue one for up and down only; with **T**, drag the ring to turn.
- **Flip ↔** and **Flip ↕** in the pieces list turn the selected pieces over, left for right or upside down, about their middle. (There is no key for flipping: the common ones are taken in the app.)
- **Delete** removes the selected pieces.

Moving, turning, sizing and flipping change where a piece **sits**, never its shape. That is what lets [linked copies](#copies-and-linked-copies) share one shape while each sits in its own place.

### Assemblies: many pieces

An [assembly](concepts.md#assembly) is a shape built from many pieces: a twig of leaves, a flower of petals. Each piece has its own:
- **Colour**: pick it under **Chosen piece** in the pieces list. It shows on the canvas and in the box's **Colours** output. New pieces are white, so Colours matches the Mask until you choose.
- **Height**: 0 to 1, how high the piece stands in the **Height** output (1 is the box's own **Height** setting). Lower the leaves at the back of a twig so the ones in front stand over them.
- **Place in the order**: **Up** draws a piece later, over the ones above it; **Down** earlier, under them. The piece on top is the one whose colour and height show where pieces overlap.

With several pieces selected, Colour and Height change them all at once.

### Copies and linked copies

Select pieces, then press **Ctrl + C** (or **Copy** in the pieces list). With the pointer where you want them:
- **Ctrl + V** pastes **plain copies**: each has a shape of its own, so changing one changes nothing else.
- **Alt + V** pastes [**linked copies**](concepts.md#linked-copy): they share one shape with the pieces they were copied from. **Double-click any of them and drag its points, and every linked copy changes**, each staying where you put it, turned and sized its own way. Changing **Does** (Add, Cut out, Line) changes them all too. Colour, height, name and mirroring stay each copy's own.

The pieces list ends a linked copy's name with "· linked", and with one chosen, says how many others it is linked with. **Unlink** gives it a shape of its own from then on.

Copied pieces paste onto any Shape box's canvas, in this project or another tab. Linked copies pasted onto another canvas are linked to each other there, not to the pieces they came from.

**Example: a twig.** Draw one leaf with the mirror line. Select it, **Ctrl + C**, then **Alt + V** five times with the pointer along a stem, turning each copy with **R**. Draw the stem as a thin piece. Now double-click any leaf and reshape it: the whole twig follows.

### Groups

Select pieces (Ctrl + click adds) and press **Alt + G** (or **Group** in the pieces list). The group shows as a heading in the list with its pieces under it.
- **Clicking any of its pieces on the canvas selects the whole group**, so dragging moves it as one, and W, R and S move, turn and size it together.
- To change just one piece of a group, click it **in the list**.
- A group's pieces stay together in the order. **Group up** and **Group down** (with the group chosen) move the whole group; a piece of a group moves up and down only among its group; other pieces move past a whole group at once.
- Type a new name for the group under **Group**.
- **Shift + G** (or **Ungroup**) takes the group apart; the pieces stay where they are.

### Edges: jagged, serrated, wavy, torn

Real leaves rarely have smooth edges. Choose a piece that adds or cuts out, and under **Edge** in the pieces list pick:

| Edge | Looks like |
|---|---|
| **Smooth** | The outline as you drew it. |
| **Jagged** | Sharp teeth of uneven heights: holly, nettle. |
| **Serrated** | Saw teeth leaning one way: rose, elm. |
| **Wavy** | Soft, even waves: oak-like or ruffled edges. |
| **Torn** | A rough, ragged edge: torn paper, a nibbled leaf. |

- **Amount**: how deep the teeth are. 0 leaves the outline as drawn; 1 makes them as deep as they are wide.
- **Tooth size**: how wide one tooth is along the outline, in centimetres on the finished material (from the graph's real-world size).
- **Seed**: a number picking which teeth; **↻** rolls new ones. The same seed always gives the same teeth.

You still draw the smooth outline; the teeth are worked out from it, so you can reshape the leaf and the teeth follow. They are vectors too: as sharp at 4096 pixels as at 64. The edge [belongs with the shape](concepts.md#edge-randomiser), so **linked copies share it**: give one leaf of a twig serrations and every linked leaf has them.

**With the mirror line:** a mirrored piece's mirror image has the same teeth, mirrored. A half leaf drawn against the mirror line keeps its middle smooth: its teeth fade out at the line, so the two halves still meet without a gap.

### Vary copies: no two leaves alike

Linked copies share one shape, so a twig of them can look stamped out. Choose a piece and tick **Each copy differs** under **Vary copies**. Every copy is then drawn:
- a little bigger or smaller (**Size ±**, in %),
- turned a little either way (**Angle ±**, in degrees),
- lighter, darker or tinted a little (**Colour shift**, in %),
- with teeth of its own (when it has an edge).

They still share one shape: reshape one and every copy follows, each still varied its own way. The setting is shared by linked copies, so ticking it on one leaf varies the whole twig. Each piece has its own [seed](concepts.md#seed); **↻ New look** gives the selected pieces new seeds when you don't like how one came out. Nothing is random from one opening to the next: the same seeds always give the same twig.

- **A group varies as one**, by its bottom piece's settings, so a leaf grouped with its veins is sized and turned with them.
- **A mirrored piece's mirror image** varies as a copy of its own (unless the piece touches the mirror line: a half leaf against the line varies as a whole leaf, sized about the line and never turned, so its halves still meet).

### Veins

Lines you draw (pieces that **Does: Line**) are [veins](concepts.md#vein): a groove or ridge in the box's **Height** and a darker or lighter line in its **Colours**, only where the shape is. Choose a line and, under **Vein**:
- **Width**: how wide, in centimetres.
- **Depth**: below 0 a groove (sunk in), above 0 a ridge (raised); a vein's middle is deepest, rounding off to its sides.
- **Shade**: below 0 darker, above 0 lighter.

New lines start as a gentle, slightly darker groove. Set Depth and Shade to 0 for a line that is only in the **Outline**. Lines drawn before veins existed open with Depth and Shade at 0, so older drawings look as they did.

**Example: a razor leaf.** Draw half a leaf against the mirror line, give it a **Serrated** edge (Amount 0.8, Tooth size about 1 cm), and draw the midrib and a few side veins as lines. Group the leaf and its veins (Alt + G), copy them (Ctrl + C) and paste linked copies (Alt + V) along a stem. Tick **Each copy differs**: every leaf comes out its own, and reshaping one still reshapes them all.

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
| **Height** | The shape puffed up: rising from its edge to its full height **Puff** centimetres in, rounded, like a cushion; each piece as high as its own **Height** sets; veins sunk or raised along the lines. |
| **Outline** | A line along the shape's edge, **Outline** centimetres wide, with the lines you drew. |
| **Colours** | Each piece in its own colour (shifted when copies vary), laid in order (the top piece shows where pieces overlap), black outside the shape, darker or lighter along veins. Cut-outs take colour away as they take the shape away. Plug it into **Material from height**'s colour, or blend it with a Gradient map's colours. |

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
| Copy the selected pieces | `Ctrl + C` (or `Alt + C`) |
| Paste plain copies at the pointer | `Ctrl + V` |
| Paste linked copies at the pointer | `Alt + V` |
| Group the selected pieces; ungroup | `Alt + G`; `Shift + G` |
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
- **Problem:** changing one leaf changed all of them. **Why:** they are linked copies (pasted with Alt + V), sharing one shape. **Fix:** choose the leaf and click **Unlink** first; next time paste with Ctrl + V for copies of their own.
- **Problem:** clicking a piece selects several. **Why:** it is in a group, and a click on the canvas selects the whole group. **Fix:** click it in the pieces list instead, or **Ungroup** (Shift + G).
- **Problem:** the app says sizing would make a piece nothing. **Why:** a piece sized down to almost nothing can't be sized back up. **Fix:** size it up a little first, or remove it.
- **Problem:** where leaves overlap, the Height has a step. **Why:** the top piece's height shows there; the puff follows the outside edge of all the pieces together. **Fix:** give overlapping pieces the same height, or keep them apart a little.
- **Problem:** the teeth look too small or vanish in the small pictures. **Why:** a tooth narrower than a pixel can't show at that size. **Fix:** make **Tooth size** bigger, or look at a bigger texture size; the edge is still there.
- **Problem:** a half leaf's middle is smooth though it has an edge. **Why:** teeth fade out at the mirror line so the halves meet. **Fix:** that's on purpose; the outer edge keeps its teeth.
- **Problem:** a line changes the Height. **Why:** lines are veins. **Fix:** set its **Depth** and **Shade** to 0 under **Vein**.
- **Problem:** two linked leaves don't look the same. **Why:** **Each copy differs** is on. **Fix:** untick it under **Vary copies**.
- **Tip:** a leaf: turn the mirror line on, draw half the outline from the stem to the tip with a few dragged (curved) points, starting and ending on the line, and close it. Add a vein with the pen and **Keep it as a line**.

## Related

- [Material boxes](material-boxes.md): every other box, and how the Shape box's outputs fit them.
- [Material graphs](material-graphs.md): the Material mode, wires and undo.
- [Concepts](concepts.md): [vector shape](concepts.md#vector), [mirror line](concepts.md#mirror-line), [cut-out](concepts.md#cut-out), [assembly](concepts.md#assembly), [linked copy](concepts.md#linked-copy), [edge randomiser](concepts.md#edge-randomiser), [vein](concepts.md#vein), [seed](concepts.md#seed), [seamless tiling](concepts.md#seamless-tiling).
