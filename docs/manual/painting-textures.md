# Painting textures

> Each material gets its own **texture set**: layers of paint laid on the shape through its UVs, shown on the model and in the UV Editor.

This is the start of the app's texture painter (Phase 11): you can paint on the model and in the UV Editor with a round brush in **Paint mode**, build up layers (paint, fill, groups, blend modes, clipping) in the **Paint layers** panel, and bring in pictures painted elsewhere. More brushes, masks and colour tools follow.

## When to use it

- You want to see a texture on your model while you work, for example a picture of wood planks on a crate, or a hand-painted texture from another program.
- You want your painting kept inside the project file, so it's saved, backed up and undone with everything else.

A texture lands where your [UVs](concepts.md#uvs-and-texturing) put it, so unwrap first (every ready-made shape already has UVs). If the picture looks stretched or in the wrong place, the UVs are what need changing: see [Unwrapping](unwrapping.md) and [Checker and Stretch views](checker-and-stretch.md).

## How texture sets work

A game engine gives each material its own textures. The app does the same: every [material](materials.md) can have a **texture set**, its textures and their layers. It goes by the material's **name**: materials with the same name on different objects are one material in an export, so they share one texture set.

A texture set's **layers** are mixed bottom first, like layers in a paint program: a layer higher up covers the ones under it where it's painted, and lets them show through where it isn't. Where no layer is painted, the material's own colour shows.

A new texture set is made at the project's texture size (UV → Texture, see [Texture size and texel density](texture-and-density.md)), 2048 pixels as standard.

### When paint shows on other shapes too

Every new shape starts with the **Default** material, and its UVs fill the whole texture square. So two new shapes share one texture, and their UVs lie on top of each other: paint on the sphere lands on the cube too, because both use the same spot of the same picture. Substance Painter and Blender behave the same way; it's how textures work, not a fault in the shape.

While you paint a shape that shares its texture like this, the **Paint layers** panel says so in yellow and names the other shapes, with two fixes:

- **Give it its own texture:** the shape gets a copy of the material (called, for example, *Default – Sphere*) with a copy of the painting so far, so nothing you see changes. From then on its paint stays on it. One more texture in the game.
- **Pack them onto one texture:** lays the UVs of all those shapes side by side on the shared texture, so each has its own part of it; their paint moves with them. One texture for them all, which games like (fewer textures to load and draw). Each shape gets a smaller part of the texture, so its detail is a little coarser.

Either is one step to undo. Shapes that are meant to share a spot, such as [instances](copies-and-instances.md) and pieces on a [trim sheet](trim-sheets.md), aren't warned about.

## Painting on the model

1. Click **Paint** on the mode palette (the brush, under UV), or the **Paint** layout tab on the right of the menu bar (from the Paint page, the other tabs there take you back). The window switches to the **Paint layout**: the viewport and the [UV Editor](uv-editor.md) side by side; on the right, the **Paint layers** at the top (with **Objects** as a tab beside them), and the **Paint** tab with the brush's settings under them (with **Attributes**, the selected object's own settings, as a tab beside it). Keeping the brush in its own tab means Attributes stays about the object, and the brush's settings don't push the object's out of sight. Over the top of the viewport floats the **brush bar** (see [The brush bar](#the-brush-bar) below). The viewport shows **Display → Surface → Painted**, and the modifiers column steps aside.
2. Move the mouse over a shape: a circle shows the brush's size, with a fainter ring where its full-strength part ends.
3. **Press the left button and drag** to paint. You can start beside the model: the stroke begins where the brush reaches it, as in other painting programs. The paint goes onto the texture set of the material under the brush, into the layer chosen in the **Paint layers** panel (the top paint layer if you haven't chosen one; a layer called *Paint* is made the first time, and the texture set too, at the project's texture size).
4. Let go: the whole stroke is one step to undo (**Ctrl + Z**).
5. Press **Esc** (or click another layout tab, top right) to leave Paint mode. If a menu is open, Esc closes the menu first. Entering Paint mode puts down a modelling tool (Extrude, Bevel and so on) and goes back to selecting, so it doesn't get in the brush's way. The window goes back to the layout you were in, and the viewport to the surface it showed before.

**A plain page:** while you paint, the modelling toolbar and the mode column step aside, the view shows no names or grid, and it stays one view (a middle click doesn't split it into four, as it does when modelling). What's left is the model and what paints it. To see the grid, tick **Grid** in the tool row's **Show** list; it's remembered. The left bar comes back only for the paint tools, if you put them there in Settings (see below).

**Why your paint doesn't leak or show seams:** the app paints each spot of the texture where it sits on the shape, seen from the camera. So paint carries across a UV seam onto the matching piece, even when the two pieces lie far apart on the texture, and the join doesn't show. Parts hidden behind others, or facing away, are left alone, so a stroke on the front doesn't come through on the back. (Tick **Paint through** to paint those too.)

### The brush bar

While you paint, a small bar floats over the top left of the painting area, under the view's name, with the settings you change most often. The **painting area** is the viewport and the [UV Editor](uv-editor.md) beside it together: you paint in both, so the bar lies across both rather than over the viewport alone. Only when the whole area is narrow do its groups take two or three rows. Keeping them over the views means you don't look away from the painting to change them, as in most painting programs. If you close the UV Editor or move it away from the viewport's side, the bar and the tools stay over the viewport.

- **The two colour squares.** The front one is the brush's colour. Click it to open the colour picker under it (see [Choosing colours](#choosing-colours)); click anywhere else to close it. The one behind is a second colour to keep at hand: click the part of it that shows to swap the two, say to go back and forth between a colour and its shadow. The eraser and the effect brushes paint no colour, so they have no squares.
- **The brush drop-down**: a small picture of the brush and the tool's name (Paint brush, Eraser, Smudge…). Click it for the ready-made brushes and the ones you kept, the same as in the brush shelf under the views; choose one to paint with it.
- **Size**, **Flow**, **Hardness** and **Spacing**, as bars that fill with their value, so you can read them at a glance. Drag a bar sideways to change it (hold **Shift** for small steps), or click it without moving and type. Size and Spacing fill faster at the small end, where most brushes sit, and change by a share of themselves as you drag: about twice as big for every 90 points right, so a small brush and a big one both change at a gentle, even pace. While you drag **Size**, the brush's circle shows in the middle of the 3D view at its new size, so you can see how big it is on the model. They're described in the table below.
- **The mirror switch** (two halves and the axis letter) turns mirror painting on and off (as `L` does); the small arrow beside it chooses the axis. See [Mirror painting](#mirror-painting).
- **The pen switch** turns on the pen's pressure changing the size; **the smoothing switch** (a wavy line) turns on smoothing (as `D` does). How much of each is set in the Paint tab, under **Pen and smoothing**. A switch is lit while it's on.

With the polygon fill tool and **Fill the layer** the bar shows only the colour squares and the tool's name. The rest of the brush's settings (what it paints on, its tip, how its size is measured, the realism channels) stay in the **Paint** tab.

### The Paint tab

The **Paint** tab (under the Paint layers in the Paint layout) holds everything about the tool in use. At its top, **Show in the views** switches what the views show (see [Showing one channel](#realism-channels)); each channel's button has a dot of its colour. Under it the settings sit in **folding sections**: click a section's name to open or close it. Each section says on the right what's set inside (the tip's name, "pressure → size", "Mirror X" or "off"), so you can see it without opening it. The app remembers which you left open.

| Section | What's in it |
|---|---|
| **Tip** (open to start with) | A picture of a dab as it lands, the tip's **Shape**, **Size space**, **Alignment** and **Paint on**. |
| **Lays down** (with the realism channels) | The roughness, metal and glow the brush lays down besides its colour. |
| **Pen and smoothing** | How the pen's pressure changes the size and flow, and smoothing. |
| **Symmetry and trim** | Mirror painting, and the trim row a stroke stays in when the project has a trim sheet. |
| **Paint through and effects** | **Paint through**, and the settings of the effect brushes (hue shift, the projection's picture, clone). |
| **Keys for painting** | Each key and what it does, one per line. |

The polygon fill tool has **Fills** (what a click fills, and erasing), **Lays down** and **Symmetry and trim**.

### The brush

| Setting | What it does |
|---|---|
| **Size** | The brush's radius, in the unit its **Size space** uses: pixels on screen, centimetres or texture pixels. Hand-painted textures often start with a big soft brush for colour, then smaller harder ones for detail. |
| **Size space** | Where the size is measured. **Viewport** (standard): on screen, so the brush stays the same size as you zoom and zooming in paints finer detail. **Object**: on the surface, in centimetres, the same real size on every object however far away. **Texture**: in texture pixels ([texels](concepts.md)), the same number of pixels wherever it lands, however close the camera is or how the UVs are scaled; good for pixel-precise detail and for matching a texel density. A warning shows if the brush is smaller than a texture pixel, as its detail wouldn't show. |
| **Alignment** | How each dab is laid on the surface. **Camera** (standard): round as the camera sees it, so on a surface turned away it stretches into a long oval, as a spray would. **Wrap**: round on the surface itself, bending round edges and curves without stretching: the usual choice for painting on a model, and Substance Painter's standard (its *Tangent \| Wrap*). **Planar**: round on the surface where it lands, like a stamp pressed flat, fading where the surface curves away, so it doesn't run round an edge (Painter's *Tangent \| Planar*). **UV**: round on the texture: it follows the UVs, stretching where they stretch and stopping at a seam, for painting by the layout. Each stroke remembers its alignment, so it is painted the same way again when the texture size changes or the model changes. |
| **Flow** | How much paint one stroke lays on. Within one stroke paint doesn't pile up where the brush passes twice; stroke again to build it up, as in Photoshop or Krita. |
| **Hardness** | How much of the brush paints at full strength: 100% has a crisp edge, 0% fades from the middle. |
| **Spacing** | The gap between the brush's dabs along a stroke, as a share of its width. Small gaps make smooth lines. The width is the brush's size where it lands, so a brush measured on the object or the texture keeps the same spacing however far away the camera is. |
| **Paint through** | Paints parts hidden behind others and facing away too. |

Under the settings, the panel says what you're painting on: the material and the layer.

### The tool row

Along the bottom left of the view, one row holds what the view and the brush need, so you never look away from the painting to find it. (While painting, the view has no menu line, dice or corner buttons: this row takes their place.)

**The view's controls**, first:

- **☰**: the view's menus (View, Cameras, Display, Options, Panel) and the navigation picker, for anything not in the buttons beside it.
- **3D**: which camera the view looks through. It says *3D* for the ordinary perspective view, or the name of a flat one (Top, Front…). The list also has the framing commands.
- **Lit**: how the model is drawn. It says the [paint preview](#previewing-your-painting) in use (Lit, Unlit, Toon…), or *Wireframe*. The list has Shaded and Wireframe, the previews, and [Seeing it as the player will](#seeing-it-as-the-player-will).
- **Show**: what is drawn over the model: **Edges**, **Grid** (off as standard while painting), **Cavity**, hard edges, seams, normals, collision and triangles.

**The tools**, right after them (above them, when the view is too narrow for one row):

- **Select**, **Move**, **Rotate** and **Scale** (`Q`, `W`, `R`, `S`): for a quick adjustment while texturing. With one of them chosen the brush steps aside: a click selects, and a drag moves, turns or sizes the selected objects with the gizmo, as when modelling. Pick a paint tool (or press `1` to `8`) to paint again.
- **Paint brush**, **eraser**, **projection**, **polygon fill**, **smudge**, **blur**, **clone**: the brushes, each as its key does (below). Blur softens what is under it a little each pass.
- **Fill the layer** (`8`, the bucket): a click on the model fills the chosen layer with the colour, everywhere on that material, in one step. It's the quickest way to give a layer its base colour. Its settings in the **Paint** tab are its own: it always fills the whole layer, so there's no choice of polygon or UV piece (that's the polygon fill's, `4`).
- **Pick a colour** (`P`, the eyedropper): the next click takes the colour under the mouse. (Pressing `P` with the mouse over the model picks at once.)

The tool in use is lit in the accent colour.

If you'd rather keep the view clear, **Edit → Settings → Interface → Paint tools in the left bar** moves the same tool buttons into the left bar, at the same size as the mode buttons.

### Brush keys (as in Substance Painter)

| Action | Shortcut |
|---|---|
| Paint | `1`, then left drag |
| Erase (take paint away) | `2`, then left drag |
| Projection (paint a picture onto the model) | `3` |
| Polygon fill | `4`, then click (or drag over several) |
| Smudge | `5` |
| Clone | `6`; `V` + click sets where it copies from |
| Blur | `7` |
| Fill the layer (the bucket) | `8`, then click |
| Pick the colour under the mouse | `P` (or the eyedropper button, then click) |
| Select, move, rotate, scale (the brush steps aside) | `Q`, `W`, `R`, `S` |
| Straight line from the last stroke | `Shift` + click |
| Smoothing on and off | `D` |
| Mirror painting on and off | `L` |
| Brush smaller / bigger | `[` / `]` |
| Size (sideways) and hardness (up and down) | `Ctrl` + right drag |
| Flow (sideways) | `Ctrl` + left drag |
| Leave Paint mode | `Esc` |

In Paint mode, `1` to `8` pick the tool instead of switching modes, `V` + click sets the clone source, and `P` picks a colour instead of switching the view between perspective and orthographic (that stays in the **3D** list), and `L` turns mirror painting on and off. The view still turns, pans and zooms as usual (Alt + drag, and the right and middle buttons in your navigation scheme).

## Painting in the UV Editor

The same brush works in the **UV Editor**, flat on the texture. While you paint, the editor is plain: at its top it says **UV ·** and the object's name (or what to do when nothing shows), with no menu line or notes to look past. This is handy for straight lines along a piece, for painting a row of a [trim sheet](trim-sheets.md), or for reaching a spot the 3D view can't see. While you paint, the UV Editor draws [seams](seams.md) in calm grey rather than orange: the brush paints across them anyway, so they don't need to stand out (they're orange again in the UV mode).

1. In Paint mode, open the **UV Edit** layout (top right), so the 3D view and the UV Editor sit side by side, and select the object.
2. **Press the left button inside a UV piece and drag** (or start outside one: the stroke begins where the brush enters a piece). The paint shows in the UV Editor and on the model as you go.

The brush only paints **inside the pieces** of the material under it when you press (and a hair outside their edges, so they're fully covered). The empty space between pieces stays empty, so nothing bleeds into neighbouring pieces when the texture is made smaller for a game. Every shown object with that material is painted, as they share one texture.

All the brush's settings and keys work the same, with two differences:

- **Size:** in the **Viewport** size space, the brush is its size on screen at the UV Editor's zoom. In **Object**, its centimetres are worked out from the piece you start on, so it paints the same size as it would on the model there. In **Texture**, it is its size in texture pixels. (Alignment doesn't matter here: in the UV Editor you paint on the texture itself.)
- **Paint through** doesn't matter here: nothing is hidden in a flat view.

Clicks in the UV Editor paint in Paint mode instead of selecting. **Measure** (in the UV menu) still measures while it's on.

## Choosing a brush

The **shelf** along the bottom of the Paint layout holds ready-made brushes, each card a small square picture of the brush (a dab for the round ones, a short stroke for the textured ones) with its name under it; the brush in use is outlined. Click one to use it: it changes the brush's tip and feel, and keeps your size and colour. With a pen, every brush that comes with the app follows pressure for its size, so picking one never turns your pen off.

| Brush | Good for |
|---|---|
| **Soft round** | Blocking in colour and soft gradients |
| **Hard round** | Clean shapes and crisp edges |
| **Chalk** | A broken, grainy edge, like chalk on rough paper; the grain stays put on the texture |
| **Bristle** | Visible brush strokes: separate bristle lines along the stroke |
| **Dry brush** | Bristles that skip, for scuffs, worn wood and stone |
| **Spatter** | Scattered specks, for dirt and chips |

- **All / Built in / Effects / Saved**, after the tabs, filter the cards; **Search**, on the right, finds a brush by name.
- **+** keeps the brush as it's set now (its tip, hardness, spacing, flow and pen settings) under a name you type. Your own brushes are kept by the app, for every project. Right click one of your brushes to delete it.
- **Shape**, in the **Paint** tab's **Tip** section, changes just the tip.

Coordinates is a tab behind the shelf in the Paint layout, if you need it.

## Smudge, blend, blur and hue shift

These brushes change what's already painted instead of laying on a colour. They're in the shelf (the **Effects** filter shows them), and Smudge has the key `5`. Their **Flow** is how strong the effect is; stroke again for more.

| Brush | What it does |
|---|---|
| **Smudge** | Drags paint along the stroke, like a finger through wet paint: it picks up the colour where you start and carries it, mixing in what it passes. |
| **Blend** | Runs the colours under it together, for soft transitions between painted areas. |
| **Blur** | Softens what's under it a little each pass. |
| **Hue shift** | Turns the hue of what's under it round the colour wheel, by the **Hue shift** angle in the **Paint** tab: warm a shadow, cool a highlight, vary a colour without repainting it. |

## Projection and clone

**Projection** (`3`) paints a picture onto the model, the way Substance Painter's projection tool does: handy for photo details, decals or a painted reference.

1. Press `3`. In the **Paint** tab, **Picture…** chooses an image (PNG, JPEG, TGA or BMP).
2. The picture appears over the views, see-through (**Shown** sets how much). **Scale** and **Move** place it; **Centre** puts it back in the middle.
3. Paint: wherever you paint, the picture's colours go onto the surface under them. In the UV Editor the picture is stretched over the texture square.

**Clone** (`6`) copies paint from one place to another: repair a seam, repeat a detail.

1. Press `6`, then hold `V` and click where to copy **from**.
2. Paint where the copy should go. The distance between the two is kept for the following strokes too, until you `V` + click somewhere else.

Clone copies on the texture, so near UV seams it copies what lies beside the source on the texture, not on the model.

## Painting with a pen

A pen tablet makes painting feel like painting: press lightly for thin, faint strokes, harder for bold ones. The app reads Wacom tablets (and most others) through their Wintab driver on Windows, and other pens through Windows itself. With a mouse there's no pressure, so the brush paints at its full size and flow.

**Pen and smoothing**, a section of the **Paint** tab (the brush bar's pen and smoothing switches turn the first and the last on and off):

| Setting | What it does |
|---|---|
| **Pressure → size** | Press harder for a bigger brush (on to start with, as in Substance Painter). The percentage is the size at the lightest touch; the curve is how fast it grows: **Soft** (light touches already paint well), **Even**, **Firm** (a firm press is needed). |
| **Pressure → flow** | Press harder for more paint (off to start with). The same percentage and curve. |
| **Smoothing** (`D`) | The brush follows the pointer on a string, so lines come out steadier and small wobbles are left out. The percentage is how long the string is. |

The line under them shows the pen's pressure as you paint, so you can see the tablet is being read. Turn the pen round and its **eraser end** rubs paint out, without changing your brush.

**Straight lines:** paint a dot or a stroke, then hold `Shift` and click somewhere else: a straight line joins them. Keep holding the button to carry on from there.

## Mirror painting

Most things are symmetrical: a face, a body, a car, a shield. **Mirror painting** paints the other side for you: every dab you paint lands on the opposite side of the object as well, reflected, so the two sides match exactly and you paint half as much.

1. In Paint mode, press `L` (or click the mirror switch on the brush bar).
2. Choose the axis with the small arrow beside it: **X** (left and right, the usual one), **Y** (front and back) or **Z** (up and down). These are the object's own axes; the mirror stands at its [pivot](concepts.md#objects-and-scenes), just like the [live mirror](symmetry.md), and moves and turns with the object.
3. Paint. A dashed circle shows where the other dab will land.

The other side is painted even when you can't see it, such as the back of the object when the mirror is set to front and back. The app paints it as if a camera stood in the mirror image of yours, so tips, pressure, projection's picture and the hidden-parts rule work the same on both sides. If the shape isn't quite symmetrical, the paint still lands on its surface, where the reflected brush meets it.

The polygon fill tool (`4`) fills the mirrored polygon too. **Smudge and clone don't mirror**: they carry colour from one place, which would drag the first side's colour across.

**In the UV Editor**, mirror painting flips left and right across the middle of the texture instead, whatever the axis. That suits UVs laid out symmetrically, such as a face unwrapped with its nose on the middle line.

> **Tip:** an object made with [Symmetry](symmetry.md) whose two halves share their UVs shows the paint on both halves anyway; mirror painting is for shapes modelled whole, or with their own UVs on each side.

## Choosing colours

The **colour picker** opens when you click the colour square on the brush bar. Hand-painted textures live on colour choices, so it has a few helpers.

- **Wheel:** drag round the ring to choose the hue, and in the square for how strong (left to right) and how light (bottom to top). **Sliders** shows hue, saturation and value as sliders instead.
- **New** over **Before**: the colour you're choosing over the one you last painted with. Click **Before** to go back to it.
- **Lightness:** how light the colour looks, ignoring its hue: a *value check*. A model reads well when its lit and shaded parts differ in lightness, not only in hue: a blue shadow on orange wood should still be darker.
- **Pick (P):** press `P` with the mouse over the model or the UV Editor to take the colour shown there (the painting mixed over the material's colour, averaged over a few pixels so one speck doesn't decide). Or click **Pick (P)**, then click where to pick.
- **Hex** and **R G B:** type a colour (`#E3AA66`, or `#FA6`), or copy it with **Copy**.

### Colours in the shelf

The shelf under the views has five tabs at its top left: **Brushes** (the brush cards, see [Choosing a brush](#choosing-a-brush)), **Colours**, **Materials**, **Smart masks** and **Stencils** (see [Materials, smart masks and stencils in the shelf](#materials-smart-masks-and-stencils-in-the-shelf)). Keeping colours there, beside the brushes, means you can pick one with a single click without opening anything.

- **Used lately:** the colours you've painted with, newest first, up to 16. They fill in by themselves as you paint and are kept when you close the app, so a colour you mixed an hour ago is still a click away.
- **Palette:** colours you keep with the project. **+** adds the brush's colour (making a palette if there's none); right click a colour to remove it. The list beside it chooses one of the project's palettes, makes a **New palette**, **Import palette…** or **Export palette…** as a GIMP palette file (`.gpl`, which Krita, GIMP, Inkscape and Aseprite read and write too), or **Delete palette**. Each change is one undo step.
- **Click** a colour to paint with it. **Double-click** it to paint with it and open the colour picker on the brush bar, to change it a little (a darker shade for a shadow, say). If the eraser or an effect brush was in use, the paint brush takes over, as only it paints a colour.

## Filling polygons

The **polygon fill** tool (`4`, as in Substance Painter) fills whole parts of the shape in one click instead of painting them: fast, with clean edges along the polygons. Under the mouse it shows a small cross rather than the brush's circle, since it fills whole polygons whatever the brush's size. The **Paint** tab shows **Polygon fill** with its settings:

- **Fill:** what a click fills: the **Polygon** under the mouse, its whole **UV piece**, or every polygon of its **Material**.
- **Erase:** take paint away instead of filling. Picking the fill tool always starts with this off, even straight after the eraser, so a click fills.

It fills with the colour of the colour square on the brush bar.

Click on the model, or inside a piece in the UV Editor. Drag to fill each part you pass over; the whole drag is one undo step. Press `1` to go back to the brush.

## Paint layers

Like layers in Photoshop or Substance Painter, layers let you keep parts of the painting apart: a base colour, shading on top, details on top of that. You can change or hide one without touching the others. The **Paint layers** panel (at the top right of the Paint layout) shows the layers of one material's texture set, **top first**, as paint programs list them.

- **Material:** which material's layers are shown: the selected objects' materials. Painting on a material shows its layers.
- **The list:** each layer on a row with its **eye** (click to hide or show it); a **small picture** of what it holds (what's painted on it, over a dark checker where it's clear; a fill's colour; stacked squares for a group, with an arrow before them to open or close it); its **mask's picture** beside it if it has one (white shows the layer, black hides it); its **name**, with **what kind** of layer it is under it (Paint, Fill, Gradient, Group, and "mask" or "clipped" when it has them); and on the right its **blend mode** over its **opacity**. The pictures change as you paint, so you can find a layer at a glance, as in Photoshop or Substance Painter. Layers inside a group are indented, and **↳** marks a clipped layer. **Click a layer to choose it:** the brush paints on the chosen layer.
- **Name, Opacity, Blend, Clip to the layer below** and (for a fill) **Colour**, under the list, change the chosen layer. They sit under it so that choosing a layer never moves the list under your mouse.

Under the list are the three buttons you use most, **+ Paint**, **+ Fill** and **+ Group**, and **···** with the rest: the other kinds of layer (+ Gradient, + Light and shade, + Edge highlight, + Material, and with the realism channels on + Dirt, + Wear and + Rust), what you can do to the chosen layer (move, copy, merge, group, masks, delete), the **Realism channels** switch and the **Presets**. Keeping the rarer ones in one menu leaves the panel short, so the list and the brush's settings both fit on the screen. The layer list and the chosen layer's settings under it scroll separately: a long list scrolls in its own box (up to about half the panel), and the settings have the rest of the panel, so tuning a sun, gradient or mask doesn't push the list away.

| Button | What it does |
|---|---|
| **+ Paint** | A new, empty paint layer above the chosen one. |
| **+ Fill** | A **fill layer** above the chosen one: one colour over the whole texture. On its own it covers everything; clip it to the layer below to colour only what that layer covers (masks to shape it come in a later step). |
| **+ Group** | A new empty group above the chosen layer. |
| **Up**, **Down** (in ···) | Move the chosen layer one place, within its group. |
| **Duplicate** (in ···) | A copy of the chosen layer just above it. |
| **Merge down** (in ···) | Mixes the chosen layer into the one under it, as you see them, into one paint layer. The result can't be split again (Ctrl + Z can undo it). |
| **Group** / **Ungroup** (in ···) | Puts the chosen layer in a new group, or takes a group's layers out and removes the group. |
| **Delete** (in ···) | Removes the chosen layer. |

**Blend modes** decide how a layer's colour mixes with what's under it ([Blend mode](concepts.md#blend-mode)): **Normal** covers it; **Multiply** darkens (good for shadows and dirt: white does nothing); **Screen** lightens (highlights and glow: black does nothing); **Overlay** adds contrast; **Add** and **Subtract** add or take away colour; **Colour** tints, keeping the shading under it; **Hue** changes only the hue. Where nothing is under a layer, its own colour shows, whatever the mode.

**Clip to the layer below** makes a layer show only where the layer under it is painted. Paint a shape on one layer, add a layer above it clipped to it, and you can shade, tint or pattern the shape with big loose strokes without going over its edges. A clipped layer follows the nearest unclipped layer under it, and hides when that layer is hidden.

A **group** mixes its own layers first, then lays the result on as one layer, with the group's own opacity and blend mode: handy to dim, hide or move a whole set of layers at once. Groups can hold groups.

Every change in the panel is one step to undo (a slider's drag or typing a name counts as one).


### Materials, smart masks and stencils in the shelf

The shelf's other three tabs put the things you add most often a click away, as cards like the brushes, so you don't have to look for them in a menu.

- **Materials:** Brushed metal, Wood, Stone and Paint coat. Click one to add a [material layer](#material-layers) of it above the chosen layer, the same as **···** → **+ Material** and then choosing that material.
- **Smart masks:** Edges, Cavities, Facing up and Height (and with the [realism channels](#realism-channels) on, Wear, Dirt and Rust). Click one to give the **chosen layer** that [smart mask](#smart-masks): one it has already changes to the new kind (its **Invert** stays as you set it), but a mask you painted by hand is never replaced, so you don't lose that painting (the status bar says to remove it first). The quickest way to put a colour only on the edges, in the creases or on top. With no layer yet, it adds a grey fill shown through the mask. Pointing at a card says where the mask shows.
- **Stencils:** the projection brush's picture. **Choose…** picks a picture file; once one is chosen, its card shows it, and clicking it switches to the projection brush (`3`) to paint it onto the model.

Each click is one step to undo.

### Starting layouts

Most painted models start with the same few layers. While a material has no layers yet, the panel offers **Start with a layout**:

- **Hand-painted:** a **Base colour** fill (the material's colour), a **Colours** layer for your painting, **Light and shade**, an **Edge highlight**, and a **Details** layer on top. **Colours** is chosen to start with, so what you paint or fill there stays shaded by the light and lit at the edges, as hand-painted colour should; paint small details that shouldn't be shaded (highlights, glints, markings) on **Details**. A quick start for stylised, painterly textures.
- **Realistic:** with the [realism channels](#realism-channels) on, and the view switched to the **Realistic** [preview](#previewing-your-painting) so you see its roughness and metal: a **Base colour** (fairly rough, not metal), **Cavity dirt** (dark, rougher colour in the creases, with a cavity [smart mask](#smart-masks)), **Edge wear** (a lighter, smoother colour along worn edges), **Dust** (fully rough, on what faces up), and a **Paint** layer on top.

Each layer is an ordinary layer: change its colour, opacity or mask, hide it with its eye, or delete it. The whole start is one undo step.

### Presets

A **preset** keeps a look or your brushes in a small file, so you can use it in other projects, on another computer, or share it. The **Presets** part of the Paint layers panel's **···** menu has:

- **Save style…** keeps the chosen layer, or the chosen group with everything in it. Put the layers of a look (say, moss: a green fill with a facing-up mask, and a darker one in the cavities) in a group to keep them together.
- **Save material…** keeps the material's colour and all its layers.
- **Load preset…** reads a preset file. A style is added above the chosen layer; a material replaces this material's colour and layers; brushes join your own. Each is one undo step.

The shelf has **Save brushes…** and **Load brushes…** for your own brushes.

Presets keep **recipes, not pixels** ([paint preset](concepts.md#uvs-and-texturing)): fill colours, gradients, light and shade, blend modes, opacity and [smart masks](#smart-masks) come with them and are made again on the model you load them onto, at its texture size. What you painted by hand doesn't come along, because it was painted for one model's [UVs](concepts.md#uvs-and-texturing). Preset files end in `.paintpreset`; they are plain text you can open in any text editor.

### Gradient fills

A gradient runs from one colour to another along the object: the quickest way to give a hand-painted shape its form, since light usually comes from above (darker low, lighter high).

1. In the Paint layers panel, click **···**, then **+ Gradient**. A gradient layer appears above the chosen one. It's set to **Multiply** and runs from a cool dark at the bottom of each object to white at the top, so it darkens the colours under it towards the ground and leaves the top as it is, instead of covering them.
2. In its settings, change:
   - **From** and **To**: the colours at the low and high ends;
   - **Along**: bottom to top (Z), left to right (X) or front to back (Y), along the object's own axes;
   - **Starts** and **Ends**: where along the object the change happens (set them close together for a sharp line, such as a painted band);
   - **Across**: **each object** (standard), **each part** (each piece not joined to the rest, such as every plank of a crate made as one object) or **each UV piece**, each from its own low side to its own high side.
3. Lower its **opacity** or set its **blend** to Multiply or Overlay to lay it over your colours rather than cover them.

The gradient is made from its settings on the shape, so it stays even across every UV piece and stays right as you keep modelling. You can't paint into it: a stroke with the gradient chosen goes on a new layer above it. Each object sharing the material gets the gradient from its own bottom to its own top.

### Material layers

A **material layer** is a ready-made material laid over the whole object: **Brushed metal**, **Wood**, **Stone** or a **Paint coat**. Click **···**, then **+ Material**, in the Paint layers panel, then in its settings:

- **Material:** which one. Each comes with colours and settings that suit it.
- **Colour** and **Second colour:** the main colour and the colour of its pattern (the streaks, the rings, the specks).
- **Scale:** how big the pattern is, in centimetres per repeat.
- **Sharpness:** how the pattern from each side meets at corners. Low blends the sides together; high keeps each side clean.
- **Roughness** and **Metal** (with the [realism channels](#realism-channels)): the material's own; the pattern varies the roughness a little.
- **New pattern:** the same material with a different pattern.

**Why it needs no UVs:** the pattern is laid on from three sides of the object, along X, Y and Z, like slides projected onto it from the front, the side and above. Each spot takes the projection it faces most, so the pattern doesn't stretch and doesn't show the UV seams. It is made again whenever you change the shape.

Like any layer it takes a mask, opacity and a blend mode. A typical realistic start: a material layer, then **+ Dirt**, **+ Wear** or **+ Rust** above it, then hand-painted details on top.

### Light and shade

Hand-painted textures carry their own light: the parts facing the sun are lighter and warmer, the parts turned away darker and cooler. **+ Light and shade** works that out for you from a sun direction, as a starting point to paint over.

1. In the Paint layers panel, click **···**, then **+ Light and shade**. A layer appears above the chosen one: warm light on the sides facing the sun (high, from the front left to start with), cool blue shade on the others, and shadows where one part of the object blocks the sun from another.
2. In its settings, change:
   - **Sun around:** which side the sun comes from (0° the right, 90° the back, 180° the left, 270° the front);
   - **Sun height:** how high it is (90° straight overhead);
   - **Light** and **Shade:** the two colours. Warm light with cool shade is the classic painter's choice, because sunlight is yellowish and shadows are lit by the blue sky;
   - **Warm and cool:** how much of that colour: 0% gives plain greys of the same lightness, 100% the colours as chosen;
   - **Softness:** low for a hard line between light and shade (a cartoon look), high for a gentle fade;
   - **Cast shadows:** untick for light and shade without shadows.
3. It starts as **Overlay** at **60%**, so it lightens and darkens your colours instead of covering them. Change its blend and opacity like any layer.

**Don't want lighting while you paint?** Click the layer's eye to hide it; show it again whenever you like. It's made from its settings on the shape, so it stays right as you keep modelling, and you can't paint into it: a stroke with it chosen goes on a new layer above it.

**Round shapes shade smoothly.** Light and shade follows the model's [smoothing](concepts.md#shading-and-normals), the same as the 3D view: a cylinder or a sphere gets a smooth turn from light to shade, not one flat band per side. Edges you've made hard (or that are sharper than the object's smoothing angle) keep a crisp change.

**Why bake light into the texture?** Many stylised games use little or no real-time lighting, so the texture has to show where light falls. Real lights in the game won't move this light and shade: that's what keeps the hand-painted look the same everywhere.

### Masks

A **mask** decides where a layer shows, without changing the layer itself: white shows it, black hides it, grey shows it partly. You can change your mind later by painting the mask again, which you can't do once paint is erased.

1. Choose a layer, click **···** and then **Add white mask** (all of the layer shows to start with) or **Add black mask** (none shows).
2. The mask appears as a small square at the end of the layer's row, outlined while it's chosen: the brush and the polygon fill now paint the **mask**, not the layer. Paint **white** to show the layer, **black** to hide it (the brush colour's lightness counts). Erasing on a mask takes it back to white or black as it started.
3. Click the layer's name to paint on the layer again, or the mask square to go back to the mask.

**Invert mask** swaps what shows and hides; **Remove mask** (in **···**) takes the mask away.

A quick way to colour parts of a model, as in Substance Painter: **+ Fill** with the colour you want, **Add black mask**, then press `4` and click the polygons (or pieces) to colour with white.

### Smart masks

A **smart mask** is a mask the app makes from the shape, instead of you painting it: the quickest way to get worn edges, dirt in the creases, or dust and moss on top. It's the same idea as Substance Painter's smart masks.

1. Choose the layer that should show only in some places: a **+ Fill** of light metal for worn edges, say, or a dark brown fill for dirt.
2. Click **···**, then **Add smart mask**. The layer now shows only along the object's outer edges.
3. In its settings, choose what it picks out:
   - **Edges:** outer edges, where paint wears and chips.
   - **Cavities:** inner edges and creases, where dirt gathers.
   - **Facing up:** surfaces facing up, where dust, snow and moss settle.
   - **Height:** everything above a height on the object, for a tide line or a fade toward the top.
   - **Wear:** outer edges broken into chips and scratches, where paint has worn off.
   - **Dirt:** creases and the lower part of the object, as if grime splashed up from the ground.
   - **Rust:** blotches spreading from edges and creases, with streaks running down the sides.
4. Then shape it:
   - **Width** (edges, cavities and wear), **Reach** (facing up: how far from straight up still counts; dirt: how far up from the bottom), **Level** (height: where it starts, 0% at the bottom of each object) or **Spread** (rust: how far it creeps);
   - **Softness:** a crisp line or a long fade;
   - **Noise** and **Noise size:** break the border up so it looks worn rather than drawn with a ruler;
   - **New pattern:** a different noise pattern with the same settings.

The mask is made from its settings on the shape, so it stays right as you keep modelling, with no seams between UV pieces. You can't paint on it: a stroke with the smart mask chosen paints the layer instead. **Invert mask** shows the layer everywhere *except* those places (clean edges on a dirty object, say). To touch it up by hand, put a painted layer above it.

**Why edges and cavities?** They come from the shape's [curvature](concepts.md#uvs-and-texturing): where the surface turns outward (an edge that gets knocked) or inward (a crease that collects dirt). They follow the model's [smoothing](concepts.md#shading-and-normals): the edges between the sides of a smooth cylinder or sphere aren't edges here (unless sharper than 60°), so you don't get stripes down a round shape, while its rims and any hard edges are. On a very low-poly shape the bands are wide and soft; a [bevel](bevel.md) on the edges you want worn gives a crisper result.

**Ready-made dirt, wear and rust.** With the [realism channels](#realism-channels) on, the panel's **···** menu adds **+ Dirt**, **+ Wear** and **+ Rust**: each a fill layer of the usual colour, roughness and metal (rough dark grime; bare, shiny metal; rough orange-brown rust) shown through its smart mask. Put **+ Wear** above the paint it should wear through. Change any of it in the layer's settings.

### Edge highlights

Hand-painted textures often run a thin, light line along a shape's outer edges, where the light catches them. It makes the form read clearly, even from far away.

1. Choose the top layer, click **···**, then **+ Edge highlight**. A layer called *Edge highlight* appears with a light line along every outer edge (its **Width** starts at 40%, wide enough to see on a whole model; narrow it for a finer line).
2. It uses the **Screen** blend, which only lightens: a red edge turns light red, a blue one light blue, so one highlight suits every colour.
3. To change it:
   - the **colour** and **opacity** are in the layer's settings (a warmer colour for a sunny look);
   - the **Width**, **Softness** and **Noise** are in its smart mask's settings (see [Smart masks](#smart-masks)): wider for chunky, cartoon-like edges, more noise for worn ones.

For the opposite, dark lines in the creases, set its smart mask to **Cavities**, its colour to a dark brown and its blend to **Multiply**.

## Previewing your painting

The same texture looks different under different light, and a game may not light it the way the viewport does. While the model shows its painting (**Display → Surface → Painted**, as in Paint mode), click the shading button at the bottom left of the viewport (on the Paint page, **Lit** in the tool row; or open **Display**) and choose a **Paint preview**:

| Preview | What it shows | When to use it |
|---|---|---|
| **Lit** | The viewport's soft studio light (standard). The view's [look](viewport-display.md#the-viewports-look) (Settings → Viewport → Look) shades it too: Dusk warms it a little, for example. Use **Unlit** to judge colours exactly. | Everyday painting. |
| **Unlit** | The painting's own colours, no light at all. | Hand-painted games mostly show textures unlit: this is what the player sees. Check your painted light and shade here. |
| **Toon** | A lit side and a cooler shaded side with a hard line between, and a thin rim of light around the outline. | Cartoon (cel-shaded) games. |
| **Godot** | Close to how Godot lights it as standard: a sun, the sky's light and its [tone mapping](concepts.md#uvs-and-texturing). | A quick idea of the model in Godot. Your scene's own lights will still change it a little. |
| **Godot toon** | Godot's look with its own toon shading (a material's *Diffuse Mode* set to *Toon*): the sun's light in a soft step. | Toon games made in Godot. |
| **Value check** | The painting in greys. | Checking [values](concepts.md#uvs-and-texturing): parts that should stand apart need different lightness, not just different colours. |
| **Squint** | The painting blurred. | Seeing only the big shapes and colours, as from across a room or far away in a game. |
| **Realistic** | Lit the way game engines light realistic materials: a sun, the sky's light and reflections, with the [roughness, metal and glow](#realism-channels) you painted. **Turn the light** under it moves the sun round the model. | Checking realistic materials: does metal look like metal, is the wet patch shiny? |
| **Roughness only**, **Metal only**, **Height only** (listed while the painted material uses the realism channels) | One [realism channel](#realism-channels) as greys (for height, mid-grey is flat, lighter raised, darker sunk). | Finding where you painted roughness, metal or bumps by mistake. |
| **Glow only** | The glow, its colour on black. | Seeing exactly where the surface gives off light. |

The button shows the preview's name while it isn't Lit (*Shaded · Unlit*, say), and the choice is remembered. [Cavity](viewport-display.md#cavity) is left off in every preview but Lit, since it would darken what they show.

**Why check values and squint?** A picture reads first by its lights and darks, then by its big shapes; small details and colour come last. Painters have always squinted at their work to see it that way. If your model's parts melt together in Value check or Squint, it will look muddy in the game too, however nice the details.

## Realism channels

Hand-painted textures paint the light into the colour. Realistic games instead tell the engine what each spot is made of, and the engine lights it: besides its colour, a surface has

- **[Roughness](concepts.md#uvs-and-texturing):** how rough it is. Smooth surfaces (polished metal, wet paint, glass) give sharp, shiny reflections; rough ones (chalk, cloth, old wood) spread them into a dull sheen.
- **[Metal](concepts.md#uvs-and-texturing):** whether it is bare metal. Metal reflects in its own colour and has no colour of its own underneath; everything else (paint, rust, wood, stone) doesn't. A spot is almost always either metal (100%) or not (0%).
- **[Glow](concepts.md#uvs-and-texturing):** light it gives off itself, like a screen, a lamp or lava.
- **[Height](concepts.md#uvs-and-texturing):** how high the surface stands, for detail too small to model: rivets, welds, cracks, grooves. 50% is level; lighter raises it, darker sinks it. The views light the bumps as you paint, and exports turn the height into a [normal map](concepts.md#uvs-and-texturing).

These are the **realism channels**. To use them, tick **Realism channels** in the Paint layers panel's **···** menu (the **Realistic** [starting layout](#starting-layouts) ticks it for you). Then:

- **The brush and the fill tool** get **Roughness**, **Metal**, **Glow** and **Height** rows in the **Paint** tab. Each ticked one is painted with its value wherever you paint the colour, as in Substance Painter. Roughness (50%) and Metal (0%) are ticked to start with, so paint lays down paint-like, non-metal surface; untick a channel to leave it as it is under the brush. Untick **Colour** (the first row, in the **Lays down** section) to paint only the ticked channels and leave the colours alone: a glossy wet patch with Roughness alone, or rivets with Height alone. The eraser takes away every channel. Smudge, blur and the other effect brushes change only the colour.
- **Fill layers** get the same rows: tick a channel for the layer to change it, and set its value. A *Steel* fill might be grey, Roughness 30%, Metal 100%; a *Rust* fill above it with a mask: orange-brown, Roughness 90%, Metal 0%. With the realism channels on, the tick beside a fill layer's **Colour** turns its colour off, so it changes only its channels, for example a clear coat that makes everything under it shinier.
- Each channel mixes like the colour: a layer's opacity, mask, clipping and eye count, but its blend mode doesn't (a channel is always laid on as Normal). Where no layer changes a channel, it is 80% rough, not metal and doesn't glow.

**Painting bumps:** tick **Height** under the brush (75% raises; 25% sinks), and untick **Colour** and the other channels, to paint only bumps: a weld bead along a seam, a row of rivets, a crack. Heights mix like any layer, so a crack painted over a raised panel sinks into it correctly. The full range is half a centimetre, as normal maps suit small detail; model anything bigger.

Choose **Paint preview → Realistic** to see the channels lit, and turn the light round to watch the shine move. **Roughness only**, **Metal only** and **Height only** show one channel as greys, and **Glow only** the glow on black.

**Show in the views**, at the top of the **Paint** tab, is the quick way to switch: **All** (the whole painting, in the preview you chose), **Colour** (the colours alone, with no light), and with the realism channels on **Rough**, **Metal**, **Height** and **Glow**. Picking **All** again puts back the preview you had before. The UV Editor beside the view shows the same channel. Texture painters work this way because a channel you can't see on its own is easy to get wrong: a stray patch of shine or a bump in the wrong place hides in the full view. The channels are saved with the project and go out with exports (see [Export and import](export-and-import.md)).

## Seeing it as the player will

You paint close up, but a player sees your prop from metres away, maybe a couple of hundred pixels tall. Fine details vanish, and the game reads the texture from its smaller, blurrier copies ([mipmaps](concepts.md#uvs-and-texturing)). To check what survives:

1. Click the shading button at the bottom left of the viewport (on the Paint page, **Lit** in the tool row; or open **Display**) and tick **See it as the player will**.
2. The view now shows the model exactly as many pixels tall as on a player's screen, enlarged so you can see each pixel. A note at the top says how far away and on what screen.
3. Under the switch, set:
   - **Distance:** how far the player's camera is from the model (5 m to start with, a prop across a room);
   - **Screen:** the game's screen height (720p to 2160p);
   - **View angle:** the game camera's field of view (60° suits most third-person games).

You can keep painting and orbiting while it's on. If you zoom out further than the player would be, the view is drawn as it is, and the note says so.

**What to look for:** details that turn to mush or noise at game size are wasted work, and edges or shapes that disappear may need stronger [values](#previewing-your-painting). Big, clear shapes with a few well-placed details read best.

## Why does this look wrong?

When a painted model looks off and you can't tell why, click **Why does this look wrong?** at the bottom of the Paint layers panel (or find it in the command search). A window lists what it finds on each painted object:

| What it says | Why it shows | What to do |
|---|---|---|
| **The paint breaks at seams** | A [seam](concepts.md#uvs-and-texturing) is where the shape is cut open to lie flat on the texture. The two sides of the cut were painted differently, so a hard line shows on the model. | Paint across the seam on the model, not in the UV Editor: the brush paints both sides at once. The **Blur** brush along it softens it too. |
| **Stretched pixels** | The UVs squash these polygons, so each pixel covers much more surface one way than the other, and painted detail smears into streaks. | **Show** picks them and switches to the Stretch view. Unwrap them again, or relax their UVs. |
| **Too few pixels** | These polygons get fewer pixels per metre than the project aims for (**UV → Texture**), so they look soft or blocky up close. | **Fix** raises the texture size just enough; your strokes are repainted sharp. Or make their UV pieces bigger. |
| **Flat lighting** | The lightest and darkest parts of the painting are close together, so the model looks flat, especially in games that don't light their textures. Only checked for hand-painted materials: a material with the [realism channels](#realism-channels) on is lit by the game's own lights, so baking light into it would be wrong. | Paint lighter tops and edges and darker undersides and creases. **Fix** adds a [Light and shade](#light-and-shade) layer to start from. |

Point at a line for the full explanation. **Show** selects the polygons concerned (in the Polygons mode) so you can see where they are; **Fix**, where there is one, is a single step **Ctrl + Z** can take back. The list updates as you work while the window is open.

## Vertex colours

**Vertex colours** are colours kept on the shape itself: one on each corner of each polygon, blended across the polygon. They need no UVs and no texture, so they're cheap in a game, and they're common for low-poly props, for tinting, and as masks a game's shaders read.

1. In Paint mode, choose **Paint on: Vertex colours** at the top of the **Paint** tab. The view switches to **Display → Surface → Vertex colours**.
2. Paint on the model with the brush: the corners within its reach take on its colour, fading with the brush's **Hardness** and **Flow** (and your pen's pressure). The **eraser** paints back toward white.
3. Use the **polygon fill tool** (`4`) to fill whole polygons, UV pieces or the material with one colour: each polygon's corners all get it, so it's a crisp, flat colour.

**Soft or crisp?** Where polygons meet at a point, each keeps its own corner colour. Painting with the brush gives neighbours the same colour, so the colour blends smoothly; filling polygons gives each its own, so colours meet in a crisp line, the classic low-poly look.

**From the texture** (under Paint on) turns a painted model into vertex colours: every corner of the selected objects takes the colour of the texture at its spot. Handy for a cheap, far-away copy of a prop, or a game without textures. The detail between corners is lost, so it suits shapes with enough points.

A stroke is one undo step. The colours go out in glTF files (as the mesh's vertex colours, `COLOR_0`); OBJ files can't hold them. Choose **Paint on: Texture** to go back to painting the texture. Vertex colours are painted on the model only, not in the UV Editor.

## Getting your textures into the game

**Seeing it in the game as you paint:** with a game project folder chosen (File → Export → Game project), tick **Update the game live**: each pause in your painting sends the model into the game project, and Godot shows it a moment later. See [Export into a game project](export-and-import.md#export-into-a-game-project).


Your painting goes out with the model when you export (**File → Export**, or every save into a game project folder): inside a glTF file for Godot, Unity, Unreal Engine and Solis, or beside an OBJ file. Choose PNG or TGA and the padding in the Export window; see [Export and import](export-and-import.md#settings). The exported texture looks like the **Unlit** [preview](#previewing-your-painting): your layers mixed over the material's colour.

## Bringing in a picture

1. Select an object. In the **Attributes** panel, under **Materials**, each material has an **Image…** button.
2. Click **Image…** beside the material you want to texture, and choose a picture: PNG, JPEG, TGA or BMP.
3. The picture becomes a paint layer on that material's texture set, named after the file, stretched to fill the texture square. The viewport switches to **Display → Surface → Painted**, so you see it on the model at once.
4. Open the **UV Edit** layout (top right) to see the same texture behind the UVs in the UV Editor.

Importing again onto the same material adds another layer on top (named *grass 2* if *grass* is taken). Each import is one step to undo (**Ctrl + Z**).

## Paint follows the model

You don't have to finish modelling before you paint. Paint belongs to the polygons it was painted on, so when a change moves polygons on the texture, the paint moves with them:

- **Modelling:** extrude, bevel, loop cut, slit and the rest. A polygon that is still there keeps its paint, even if it moved or changed shape. A new polygon (the sides of an extrude, say) picks up the paint nearest it on the shape as it was.
- **UV work:** moving, turning or scaling UVs, re-packing and re-unwrapping. The paint goes wherever its polygons go on the texture.

The paint moves when the change is finished (when you let go of a drag), and it's part of the same undo step: **Ctrl + Z** takes back the change and the paint's move together.

**Why it works this way:** the texture is a flat picture laid on the shape through its [UVs](concepts.md#uvs-and-texturing). Moving the shape's points without moving their UVs needs nothing: the picture is still laid on the same way. When UVs move, the app redraws each moved polygon's part of the picture from where it was before.

When paint moves, the status bar says so: *The paint moved with its polygons (one undo takes both back).*

**Re-packing and re-unwrapping** work the same way: the painted model looks the same afterwards. When packing makes pieces smaller on the texture, each new pixel is the average of the old pixels it now stands for, so fine paint (thin lines, small dots) stays visible as a softer version of itself rather than breaking up. When pieces get bigger, the paint is smoothly enlarged: it can't gain detail that wasn't painted.

Things to know:
- Paint that moves is read from its old place, so it can soften slightly where a polygon was stretched. Paint on polygons whose UVs didn't change is left exactly as it was.
- A modifier that rebuilds the polygons, such as subdivision, makes new polygons when it's switched on, so the paint is carried over by nearest point. Switching it off again before you paint gives the paint back exactly as it was, at once. If you painted in between, the paint is carried back by nearest point, which softens it slightly and can take a while on a dense shape: paint with modifiers in the state you'll keep.

## Bakes

Hand painters lean on the shape: corners and creases are darker, edges that stick out catch the light and wear first. **Bakes** work these out for you, as maps laid on the shape the same way as paint, ready to guide your painting (and, later, to drive smart masks).

1. Select the object and open the **Paint layers** panel (in Paint mode, on the right).
2. Under **Bakes**, choose a **quality** and click **Bake maps**. The maps are worked out in the background; you can keep working meanwhile. *Draft* is quick and a little grainy, *Good* is the standard, *Best* is smooth and slow. The section says which map it is on and how far (*Baking 3 of 6 maps… 40%*); **Cancel baking** stops it. Opening another project or starting a new one stops it too.
3. The six maps appear small under the button, in as many rows as the panel's width needs. Hover over one to see what it shows.

| Map | What it shows |
|---|---|
| **Position** | Where each part is on the shape: red across, green front to back, blue up. |
| **Normals** | Which way each part faces: blue up, red and green sideways. |
| **Occlusion** | How open each part is to the light around it: white in the open, dark in corners and creases. |
| **Curvature** | Light along edges that stick out, dark along edges that go in, grey on flat parts. |
| **ID** | A different flat colour for each material slot. |
| **Thickness** | Light where the shape is thin, dark where it's thick. |

The maps come from the shape's own polygons, so they show its big forms: the corners between parts, the edges of bevels and panels. Each object is baked on its own, so one object doesn't darken another.

### Using bakes as guides

- **See one on the model:** click a map under **Bakes**. It shows over your paint in the 3D view and the UV Editor, so you can paint by it: occlusion darkens the corners, curvature lightens edges that stick out and darkens those that go in. **Strength** sets how strongly it shows. Click it again to hide it. The guide is only shown: it isn't part of the paint, and isn't exported.
- **Add as a layer:** puts the map in a new paint layer above the chosen one, mixed the same way (occlusion multiplies, curvature overlays). Paint over it, lower its opacity, or mask it, like any layer.
- **Use as the layer's mask:** makes the map the chosen layer's [mask](#masks), so the layer shows only where the map is light. Two classics:
  - **Dirt in corners:** a dark fill layer with the occlusion map as its mask, **inverted** (dark corners become where it shows).
  - **Worn edges:** a light fill layer with the curvature map as its mask: it shows on the outer edges.

Both are one step to undo.

When you change the shape (so its polygons move on the texture) or the texture size, the section says the maps are **out of date**: click **Bake maps** again. The maps aren't saved with the project; bake them again after opening it.

## Changing the texture size

Painted textures follow the project's texture size: change it in **UV → Texture** and every texture set is made the new size, in the same undo step.

The app keeps every brush stroke and polygon fill you make, so it can **paint them again at the new size**: a layer painted only with the brush (paint and eraser) and the polygon fill comes out crisp at any size. That makes a good way of working: block in colours on a small texture, then raise the size and add detail.

Anything the kept strokes can't make again is resized as a picture instead: smaller, each pixel is the average of those it replaces; bigger, the paint is smoothly enlarged and softens a little. That happens to:
- layers with an imported picture;
- layers whose paint moved with its polygons after modelling;
- merged layers;
- layers with blur, blend, smudge, hue shift, projection or clone strokes;
- every layer when several objects share the material, as the app doesn't know which object each stroke was painted on.

**Very big textures:** if the texture is bigger than your graphics card can hold in one piece (often 8192 or 16384 pixels across, less on older cards), the views show the largest copy that fits. The full-size paint is still what you paint on and what is exported.

## Big textures and many layers

Painting stays smooth on big textures: a 4K texture (4096 × 4096 pixels) with 20 layers paints, undoes and redoes without waiting. The app only works on the parts of the texture a stroke reaches, and shares the work out over all of your computer's processor cores, so a computer with more cores paints big brushes faster.

A few things still take a moment on very big textures:

- **The first time a texture set is shown**, every layer is mixed into the picture you see: a second or two for 4K with 20 layers. After that only the parts that change are mixed again.
- **A brush as big as the whole object** on a 4K texture reaches millions of pixels at once, so the first dab of such a stroke can take a fraction of a second. Smaller brushes start at once.

**Undo** keeps only the parts of the texture each stroke changed, so many strokes fit in the undo history. It shares the history's memory with everything else (**Edit → Undo memory**, 2 GB to start with); when that fills up, the oldest steps are forgotten first.

## Settings

| Setting | What it does |
|---|---|
| **Image…** (Attributes → Materials) | Imports a picture as a new paint layer on top of that material's texture set. Pictures of any size are stretched to the texture size. |
| **UV → Texture** size | The size of every texture set. Changing it paints brush and fill strokes again at the new size, and resizes the rest as pictures. |
| **Mirror** (`L`; the mirror switch on the brush bar) | Paints the other side of the object too, across its **X**, **Y** or **Z** axis through its pivot. In the UV Editor, flips left and right. |
| **Paint preview** (shading button or Display menu, Painted surface) | How the painting is lit: Lit, Unlit, Toon, Godot, Godot toon, Value check or Squint. Remembered. |
| **Paint on** (the Paint tab) | **Texture** (standard): the brush paints the material's texture. **Vertex colours**: it paints the colours on the shape's polygon corners. |
| **+ Material** (Paint layers, in ···) | A material layer: brushed metal, wood, stone or a paint coat, laid on from three sides, made again as the shape changes. |
| **Realism channels** (Paint layers, in ···) | Paint roughness, metal and glow besides the colour; the brush, fill tool and fill layers get their rows. |
| **Roughness**, **Metal**, **Glow** (the Paint tab; fill layers) | Whether each is painted (or changed by the fill layer), and its value. |
| **Turn the light** (Paint preview → Realistic) | Turns the realistic preview's sun round the model, 0–360°. Remembered. |
| **Start with a layout** (Paint layers, while a material has no layers) | Hand-painted or Realistic: ready-made layers to start from. |
| **Presets** (Paint layers, in ···; the shelf) | Save the chosen layer as a style, the material, or your brushes as a `.paintpreset` file; load one. |
| **Why does this look wrong?** (Paint layers panel, command search) | Lists seams that show, stretched pixels, too few pixels and flat lighting, with Show and fixes. |
| **Trim row: Stay in the row** (the Paint tab, while there are trim rows) | Keeps each stroke inside the trim row it starts in; see [Painting a trim sheet](trim-sheets.md#painting-a-trim-sheet). |
| **See it as the player will** (shading button or Display menu) | Shows the model as small as on a player's screen, at the **Distance**, **Screen** and **View angle** you set. Remembered. |
| **Display → Surface → Painted** | Shows each material's painted texture on the model, over the material's colour. The UV Editor shows the texture of the first selected shape's material behind the UVs. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Undo a stroke or an import | `Ctrl + Z` |
| Paint mode's keys | See [Brush keys](#brush-keys-as-in-substance-painter) above |

## Tips and common problems

- **A preset won't load:** the file may come from a newer version of the app (update it), or it isn't a preset file. The status bar says which.

- **Problem:** nothing happens when I paint. **Why:** the brush is over empty space (in the UV Editor, between pieces), or the shape has no UVs. **Fix:** paint on a shape or inside a UV piece; give it UVs with [Unwrapping](unwrapping.md) (the ready-made shapes have them).
- **Problem:** the paint is blocky. **Why:** the texture is small for how close you look, or the brush is smaller than a texture pixel (the Paint tab warns). **Fix:** raise the texture size (UV → Texture), or paint with a bigger brush.
- **Problem:** the model doesn't show the picture. **Why:** the viewport shows another surface (Colours, Checker...). **Fix:** **Display → Surface → Painted**.
- **Problem:** the picture is squashed or repeats oddly. **Why:** it was stretched to a square texture, or the UVs don't fill the square the way the picture expects. **Fix:** use a square picture painted for these UVs, or change the UVs.
- **Problem:** the eraser does nothing, and the status bar says *Nothing to erase here*. **Why:** the chosen layer is a fill or a layer made from its settings (a gradient, light and shade, a material): there's no painted paint on it to take away. **Fix:** choose a paint layer; to see less of a fill, lower its opacity or paint its mask.
- **Problem:** my strokes don't show. **Why:** the chosen layer is hidden, at 0% opacity, clipped to an empty layer, or under an opaque layer. **Fix:** check its eye, opacity and ↳ mark in the Paint layers panel, or move it up.
- **Problem:** the mirrored paint lands in the wrong place. **Why:** the object's pivot isn't on its middle line, or the axis is the wrong one. **Fix:** move the pivot to the middle ([Pivots](pivots.md)), or choose another axis beside **Mirror**.
- **Problem:** painting feels slow on a big texture. **Why:** a very big brush on a 4K or larger texture reaches millions of pixels, and a computer with few cores shares the work out less. **Fix:** use a smaller brush for detail, or paint big areas with a fill layer or the polygon fill tool (`4`).
- **Problem:** after an extrude, the new sides show the colour next to them. **Why:** new polygons pick up the nearest paint. **Fix:** paint over them; that's often what you'd want for the sides of a shape anyway.
- **Problem:** two objects show the same texture. **Why:** their materials have the same name, so they share one texture set (as they would in the game). **Fix:** rename one of the materials.
- **Problem:** the picture is blurry. **Why:** it's smaller than the texture size, or the texture size is small for how close you look. **Fix:** use a bigger picture, or raise the texture size.

## Related

- [Materials](materials.md), [Texture size and texel density](texture-and-density.md), [The UV Editor](uv-editor.md)
- [Concepts](concepts.md): [texture set and paint layer](concepts.md#uvs-and-texturing)
