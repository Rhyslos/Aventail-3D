# Sculpt brushes

> The brushes for building forms in Sculpt mode: lay on clay, swell, crease, pinch, flatten, grab and pull, each with what it's for and its key; and how to stroke with them: a pen, smoothing, straight lines, stamps and alphas.

## When to use it

Sculpting a stylised creature, rock or tree is mostly a handful of brushes used over and over: build the big forms up with **Clay strips**, shape the silhouette with **Grab**, cut folds with **Crease**, flatten planes with **Flatten** and **Scrape**, and **Smooth** (hold Shift) all the time. Each brush below says what it's good for. You don't need them all: start with Clay strips, Grab and Smooth.

Every brush works the way [Sculpting](sculpting.md) describes: select a shape, go into Sculpt mode, and stroke over it. Each stroke is one undo step, kept in the shape's Sculpt layer.

## How to use it

1. In Sculpt mode, pick a brush: press its number (**1** to **7**), click its card on the **Shelf** under the view, choose it in the brush bar's **Brush** list, or press **E** for the tool menu, which holds every brush.
2. Stroke over the shape with the left button. Hover a card on the shelf, or read the line above the cards, to see what the brush does.
3. **Hold Ctrl** while you stroke to turn most brushes round: dig instead of building, deflate instead of inflating. **Hold Shift** to smooth with any brush.
4. The [grabbing brushes](concepts.md#grabbing-brushes) (**Grab**, **Elastic grab**, **Snake hook**) work differently: press on the surface to take hold of it, then drag. What you hold follows the mouse, in every view, flat ones too.

### The brushes

The pictures on the shelf show the surface from the side: dashed as it was, solid after a stroke.

| Brush | Key | What it does | Good for | With Ctrl |
|---|---|---|---|---|
| **Draw** | `1` | Pushes the surface out along the way it faces under the brush | Quick bumps and lines | Pushes in |
| **Smooth** | hold `Shift` | Evens the surface out: bumps go, the big shape stays | Tidying after every few strokes | — |
| **Clay strips** | `2` | Lays square-edged strips of clay on the surface, filling the hollows first. The square turns with your stroke | Building forms: muscles, rocks, bark, the main brush | Carves strips away |
| **Clay** | shelf or `E` | The same, but round and softer | Gentle build-up | Carves away |
| **Inflate** | `3` | Swells the surface out in every direction it faces, like a balloon | Cheeks, bellies, fat branches, mushroom caps | Shrinks it |
| **Crease** | `4` | Cuts a sharp groove and pinches its sides together | Folds, wrinkles, the gaps between planks | A sharp ridge |
| **Pinch** | shelf or `E` | Pulls the surface towards the brush's middle | Sharpening edges and ridges | Pushes apart |
| **Flatten** | `5` | Flattens towards the surface's average level under the brush | Flat planes on rocks, wood and armour | Pushes away from it |
| **Scrape** | shelf or `E` | Flattens only what sticks out above that level | Chiselled planes, knocking bumps down | Fills instead |
| **Fill** | shelf or `E` | Fills only the hollows below that level | Filling dents without losing bumps | Scrapes instead |
| **Grab** | `6` | Holds the lump under the brush and moves it with the mouse | The silhouette and proportions | Nothing different |
| **Elastic grab** | shelf or `E` | Like Grab, but the whole shape bends around the lump like rubber, keeping its volume | Big changes: a longer snout, a bent trunk | Nothing different |
| **Snake hook** | `7` | Pulls the surface out along your stroke, the tip following the mouse | Horns, tentacles, tufts, branches | Nothing different |

**Tip:** Clay strips, Clay, Flatten, Scrape and Fill all work against the [plane under the brush](concepts.md#plane-under-the-brush): the average level of the surface the brush covers. That's why Clay strips fills hollows before building on top, and Scrape only knocks down what's above that level.

### Strokes, alphas and pens

**Kinds of stroke.** The brush bar's **Stroke** list (also in the Sculpt tab) says how a stroke is laid down:

- **Freehand** (standard): the brush follows the mouse wherever it goes.
- **Line**: press, drag and let go. The stroke goes straight from where you pressed to where you let go, and shows on the shape as you drag, so you can aim it. Good for the gaps between planks, a straight seam, a crisp edge.
- **Drag a stamp**: press where it should go, then drag. How far you drag is the brush's size, and which way you drag turns it. Let go and it's one dab. This is how you stamp an alpha: a rivet, a scale, a patch of stones exactly where and as big as you want.

Grab, Elastic grab and Snake hook always follow the mouse, whatever the Stroke list says.

**Alphas.** An [alpha](concepts.md#alpha) is a greyscale picture a brush stamps or drags: where it's white the brush works fully, where it's black not at all. Draw with the **Stones** alpha raises stones instead of a round lump; Clay strips with **Bark** lays bark. Pick one on the shelf's **Alphas** tab (the buttons at the shelf's top left switch between **Brushes** and **Alphas**) or in the brush bar's **Alpha** list; **None** goes back to a plain brush. The app comes with seven, made in code:

| Alpha | Good for |
|---|---|
| **Stones** | Cobbles, rocky ground, castle walls |
| **Bark** | Tree bark, wood grain, stringy muscle |
| **Scales** | Dragons, fish, armour |
| **Stitches** | Leather, cloth, patched creatures |
| **Rivets** | Metal plates, armour, chests |
| **Cracks** | Stone, dry earth, old wood (hold Ctrl to dig them in) |
| **Scallops** | Roof tiles, shells, feathers |

To add your own, click **Add…** at the end of the Alphas tab and choose a picture (PNG, JPEG, TGA or BMP). Its light parts work, its dark and see-through parts don't; a colour picture is turned grey. It's kept in your **alpha library**, the same in every project, and pictures bigger than 1024 pixels are made smaller. Right-click one of yours to take it out. A damaged picture, or one black everywhere, is refused with the reason.

The alpha lies flat on the surface under the brush. Its top points **up the view**; **Angle** in the Sculpt tab turns it, and **Follow the stroke** points its top the way the stroke goes instead (for scales or stitches running along a stroke). A stamp's top points the way you dragged.

**With a pen.** Pressing harder makes the brush bigger and stronger, as in Paint mode. In the Sculpt tab's **Pen and smoothing**, **Pressure → size** and **Pressure → strength** each have a switch, the share at the lightest touch, and a curve (**Soft** grows quickly at first, **Firm** needs a firm press). A mouse always sculpts at full size and strength. A Line or a stamp keeps the pressure you pressed with.

**Smoothing** (**D**) makes the brush follow the pointer on a short string, so strokes come out steadier: wobbles are left out ([smoothing](concepts.md#smoothing-lazy-mouse)). The number is how long the string is.

## Settings

Each brush keeps its own size, strength, spacing and falloff, set in the brush bar or the **Sculpt** tab (see [Sculpting](sculpting.md#settings)). Good starting points:

| Setting | What it does |
|---|---|
| Strength | Most brushes start at 50%. Grab, Elastic grab and Snake hook start at 100%, so what you hold follows the mouse exactly; lower it for a gentler pull. |
| Size | For Grab and Elastic grab, the size when you press decides how much you hold. Elastic grab bends the shape out to four times the brush's size. Snake hook keeps measuring as you pull, so zooming in while pulling gives a thinner tip. |
| Falloff | Crease starts with **Sharp** for a crisp groove; the rest with **Smooth**. |
| Spacing | How often a stroke dabs. Grab doesn't use it: only where the mouse is matters. |
| Stroke | **Freehand**, **Line** or **Drag a stamp** (above). |
| Alpha | The picture the brush stamps or drags, its **Angle** and **Follow the stroke** (above). |
| Pressure → size, Pressure → strength | How a pen's pressure changes the brush. Both on as standard, a light touch giving a tenth. |
| Smoothing | The lazy mouse's string length; **D** turns it on and off. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Draw | `1` |
| Clay strips | `2` |
| Inflate | `3` |
| Crease | `4` |
| Flatten | `5` |
| Grab | `6` |
| Snake hook | `7` |
| Every brush, in a ring at the mouse | `E` (the [tool menu](tool-menu.md)) |
| Smooth, with any brush | Hold `Shift` while dragging |
| Turn the brush round (dig, deflate, ridge) | Hold `Ctrl` while dragging |
| Smoothing on or off | `D` |
| A straight stroke (with **Line**) | Press, drag, let go |
| A stamp (with **Drag a stamp**) | Press where it goes, drag to size and turn it, let go |

Keys 8 and 9 are kept for the Trim and Mask brushes still to come. The keys are the plan's recommended choices, still to be reviewed.

## Tips and common problems

- **Problem:** Clay strips or Clay does little on a round shape with a big brush. **Why:** they build up to a level above the average under the brush; on a small, very round shape most of the surface is already above it. **Fix:** make the brush smaller than the shape's curve, or zoom in.
- **Problem:** Flatten does nothing. **Why:** the surface under the brush is already flat. Flatten levels towards the surface's own average, not to a fixed height.
- **Problem:** a horn pulled with Snake hook looks stretched and jagged. **Why:** sculpting moves points, it doesn't add them, so long pulls stretch the polygons. **Fix:** pull in shorter steps on a shape with plenty of points; a remesh made for sculpting arrives with a later step of sculpting.
- **Problem:** Grab moved nothing. **Why:** a grabbing brush takes hold where you press; pressing off the shape holds nothing. **Fix:** start the drag on the surface.
- **Problem:** the alpha's pattern looks blurred or blotchy. **Why:** an alpha can only move the points the shape has; a picture finer than the polygons under the brush can't show. **Fix:** make the brush bigger, or use a shape with more points.
- **Problem:** an alpha does nothing with Grab. **Why:** the grabbing brushes hold the surface and take no alpha. **Fix:** use Draw, Clay strips or Inflate with it.
- **Problem:** a long Line lags behind the mouse on a heavy shape. **Why:** the line is laid again from the start as you drag, as far as a frame allows. **Fix:** let go: the whole line is laid then.
- **Problem:** Elastic grab is slow on a heavy shape. **Why:** it bends everything within four brush sizes, which on millions of polygons is most of the shape. **Fix:** use a smaller brush or Grab for local changes.

## Related

- [Sculpting](sculpting.md): Sculpt mode, Sculpt layers, the brush's settings
- [Tool menu](tool-menu.md): the E ring
- [Painting textures](painting-textures.md#painting-with-a-pen): the same pen and smoothing in Paint mode
- [Concepts](concepts.md): [alpha](concepts.md#alpha), [stamp](concepts.md#stamp), [pen pressure](concepts.md#pen-pressure), [sculpting](concepts.md#sculpting), [brush strength and falloff](concepts.md#brush-strength-and-falloff), [plane under the brush](concepts.md#plane-under-the-brush)
