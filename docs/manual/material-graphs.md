# Material graphs

> Build a seamless material from **boxes joined by wires** on the node canvas, in the **Material** mode, and see it beside it on your model, a ball, a cube or tiled flat.

A [material graph](concepts.md#material-graphs) makes a texture from nothing: a box makes a pattern (noise, tiles), the next shapes it, another colours it, and the last turns it into a whole material (colour, bumps, shine) ready for a game. Every box shows a small live picture of what it makes, so you can see each step. The textures it makes **tile seamlessly**: their left edge matches their right, and their top their bottom.

This is the start of the app's material graphs (Phase 12). New to graphs? Open one of the [starter graphs](#starter-graphs): ten ready-made materials (Leaves among them, an alpha card of drawn leaves), each laid out step by step with notes explaining what each step does. You can make [your own boxes](#your-own-boxes) from groups of boxes. The first set of boxes is in: patterns (Noise, Clouds, Cells, Bricks, Tiles, Planks and more), boxes that shape them (Warp, Bevel, Blur…), colour them (Gradient map, Hue and saturation) and finish them (Material from height, Mix materials, Opacity for alpha cards). [Material boxes](material-boxes.md) lists every one in plain words.

## When to use it

- You want a material that repeats over a big surface without seams: stone tiles, bricks, ground, wood.
- You want to change a material later by moving a slider rather than painting it again.
- For one-off details on one model (a scratch here, a logo there), [painting](painting-textures.md) is quicker.

## How to use it

1. Click **Material** on the mode palette on the left (under Paint), or the **Material** tab at the top right. The window changes to the Material layout: the **Material graph** canvas in the middle with the **shelf** under it (starter graphs and your own boxes), the **Preview** at the top right and **Attributes** under it. The viewport is still there, as a tab behind the canvas.
2. Click **New graph** (above the canvas), or **Starter graphs** to open a copy of a ready-made one (see [Starter graphs](#starter-graphs)). A new graph starts with one box, **Material out**: whatever is plugged into it is the graph's material. Graphs are saved in the project, and the list above the canvas switches between them; **Rename** and **Delete graph** are beside it.
3. **Add a box:** point at the canvas and press **Space**, or right-click on an empty part of it. The box search opens at the pointer. Type a few letters ("noi"), or pick from the list, and click a box or press **Enter**. Boxes are listed in the order experts work in: **Make** (patterns from nothing), **From the model** (the model's own shape, for the painter), **Shape**, **Detail**, **Colour**, then **Finish** (whole materials).
4. **Connect boxes:** drag from a box's **output dot** (on its right edge) to another box's **input dot** (on its left edge). Wires take the colour of what they carry:

   | Colour | Carries |
   |---|---|
   | Grey | A **shape**: black and white, inside and outside |
   | Amber | A **height**: greys, light is high |
   | Blue | A **colour** picture |
   | Green | A **whole material**: colour, height, roughness, metal and the rest together |

   Let go of a wire on **empty canvas** and the box search opens with only the boxes that can take it; the box you pick is connected straight away.
5. **Disconnect:** drag a wire's end away from the input dot it goes into, and let go anywhere that isn't a dot.
6. Click a box to see its **settings in Attributes**: what it does, its sliders, and **Usually comes next**, buttons that add the box usually used after it, already connected. Drag a slider and the pictures follow as you drag (smaller while you drag, sharper when you let go).
7. Plug a **Material from height** box into **Material out** and the preview shows your material.

### The app keeps the layout tidy

Wires always run **left to right**, so you can read a graph like a sentence: patterns on the left, the material on the right. You can drop a box anywhere; when you let go, the app tidies around it. Boxes snap into columns, a box that would sit left of what feeds it is pushed to the right (with what it feeds), and a box dropped onto another pushes it down. The tidying is part of the same step, so one **Ctrl + Z** takes the move and the tidying back together.

### When something won't work, the app says why

Some wires don't make sense: a colour has no inside or outside, so it can't go into a plug that wants a **shape**. When you drag a wire over a plug that won't take it, the plug turns **red** and a note says why in plain words, with ways to make it work, such as "Connect Tiles instead". The status bar says it too. Let go and nothing is connected, but the note stays: click one of its buttons to do that instead, or **Never mind** (or click anywhere else) to close it.

A box that can't run (say Levels with nothing plugged into its input) has a red outline and a **!** in its corner. Click the **!**, or select the box and look in Attributes, for why and what would fix it.

Some wires are allowed but change what they carry: a shape can be used as a height, greys as a colour, and a material gives its height or its colour. While you drag over such a plug, a green note says how it will be used.

### Bypassing a box

To see what a box adds, switch it off for a moment: **bypass** it. A bypassed box doesn't do its work; what goes into it passes straight through to what comes after, so you can compare the material with and without it. Its settings are kept for when you switch it back on. See [bypass](concepts.md#bypass).

- Click the small **switch** on the box's title (left of where a **!** would go). Click it again to switch the box back on.
- Or right-click the box and choose **Bypass** (**Switch back on** when it is bypassed). With several boxes selected, it bypasses them all.
- Or select the box and tick **Bypassed** in Attributes, beside the box's group.

A bypassed box is drawn **dimmed**, marked **Bypassed**, with its switch lit, and a wire is drawn through it from the input it passes on to each output. Each change is one step to undo. Bypass is saved with the project, kept when you copy and paste the box or put it inside [your own box](#your-own-boxes), and the export check reminds you of bypassed boxes, since the textures are baked without them.

**What passes through.** Each output passes on the first input that carries the same thing (a height to a height output, a colour to a colour output). If there is none, an input that can stand in for it is used, as a wire would: a shape as a height, greys as a colour, a material's height or colour. An output with nothing that can pass to it gives nothing, and the boxes after it say why in plain words, offering **Switch … back on**. For example, **Material from height** makes a material from a height; bypassed, it has no material to pass on, so Material out says "Material from height is bypassed… it makes that itself". A box that makes a pattern from nothing (Noise, Tiles) has nothing to pass on at all. **Material out** can't be bypassed: nothing comes after it.

Bypassing a [moving box](#animated-materials) stops it moving: while the preview plays, nothing after it is worked out again.

### Reroute dots

A long wire can cut across boxes. **Click the wire** to put a **reroute dot** on it where you clicked, then **drag the dot** to steer the wire round the boxes: the wire bends through its dots, still reading left to right. A wire can have several dots (up to 16); click it again for another.

- **Click a dot** to select it (it gets a ring), and press **Delete** to remove it.
- Dots belong to their wire: they go when the wire is taken out or replaced, or one of its boxes is removed.
- Move both boxes at a wire's ends and its dots come along. When the app tidies the layout, dots move with the boxes they are near and stay between the wire's two ends, so the wire never runs backwards; a dot dragged behind the box the wire comes from is put back just after it.
- Adding, moving and removing a dot are each one step to undo. Dots are saved with the graph and copied with the boxes at both ends of their wire.

### Finding a box

In a big graph (a starter, or one you have grown), a box can be hard to spot. Type in the **Find in the graph** field on the right of the bar above the canvas, and the first box that matches is **selected** and brought into view, with its settings in Attributes. Every box and [comment frame](#comment-frames) that matches gets a **yellow ring**, and they stay ringed while the words are in the field.

- **‹** and **›** beside the field go to the previous and next match (**Enter** in the field goes to the next too); "2 of 5" says where you are. After the last comes the first again.
- A **list** of every match opens under the field while you type: click one to go to it. Click anywhere else to close the list; the words stay, so ‹ and › still work.
- **Esc** in the field empties it and takes the rings away.
- Right-click a box and choose **Find others like it** to go through every box of that kind (every Noise, say).
- **Find a box in the graph…** in the [command search](command-search.md) puts the cursor in the field. There is no key for it yet.

**What is searched:** each box's name (for [your own boxes](#your-own-boxes), the name you gave them), the choice a box is set to (Blend set to **Multiply**, Basic shape set to **Star**), the names of the pieces drawn on a [Shape box](shape-canvas.md), and each comment frame's title and note. Every word you type must be there, in any order, and capitals don't matter: "mat out" finds Material out. Numbers in settings aren't searched. Matches are in reading order, left to right as the graph flows, then top to bottom. Inside your own box, only the boxes inside are searched.

Going to a match changes nothing in the graph, so there is nothing to undo.

### The minimap

When the graph is too big to see at once, a small **minimap** shows in the canvas's bottom right corner: the whole graph in miniature, with the comment frames in their colours, the wires (bending through their [reroute dots](#reroute-dots)) and the boxes as small blocks (selected boxes in the highlight colour, [found](#finding-a-box) ones in yellow). The **white rectangle** is the part on screen.

- **Click** on the minimap to move the view there, or **drag** on it to slide the view about. The zoom stays as it is. Nothing on the canvas under the map is clicked.
- It hides itself when the whole graph fits on screen (press **Z** with nothing selected to frame everything), and on a very small canvas.
- To switch it off, untick **View → Minimap** on the bar above the canvas. The **View** menu can also frame the whole graph or the selected boxes. The minimap comes back on when the app starts again.
- Inside [your own box](#your-own-boxes), it maps the boxes inside.

### Comment frames

A comment frame is a coloured box with a title, for grouping boxes ("Stones", "Moss in the gaps"), so a big graph reads as its parts.

- Select some boxes and press **C**: a frame goes round them, its title ready to type (press **Enter** when done). With nothing selected, **C** puts an empty frame at the pointer.
- **Drag its title** to move the frame with every box inside it. A box is inside a frame when its top left corner is.
- **Drag its edges** (left, right, bottom) to make it bigger or smaller.
- **Double-click its title** to rename it.
- Click its title to select it: its **colour** and a longer **note** are in Attributes. **Delete** removes the frame, not its boxes.
- The note shows on the canvas too, under the title, in the room above the frame's first box (drag the boxes down to make room). Zoomed far out it is too small to read, so it isn't drawn; the title still is.
- Frames can sit inside frames, and their titles stay readable however far out you zoom.

### Starter graphs

Starting from an empty canvas is hard: a brick wall can be built a hundred ways. The **starter graphs** are ready-made materials to open, look at and change. Each is laid out left to right in **comment frames**, one per step, in the order experts work (make the pattern, shape it, add detail, colour it, finish it). Each frame has a **note** under its title saying, in plain words, what that step does and why. Read them left to right, click boxes to see their settings, and click a box's picture to [look through it](#looking-through-a-box) at what that step adds.

**Open one.** Click **Starter graphs** above the canvas (or **Start from a starter graph** on an empty canvas): a window shows each one with its picture and what it teaches. Click one. Or click its card on the shelf under the canvas, in its **Starter graphs** tab (the shelf shows **Your own boxes** instead once you have some; click the tabs to switch). A **copy** goes into your project, named after it ("Bricks", then "Bricks 2"…), and the canvas shows it from its first step, close enough to read the notes. It is yours: change anything, nothing else changes. One **Ctrl + Z** takes it out again.

Each starter also has a few settings already **shown in the painter** (the number of bricks, the colours, how much moss…), so it works as a [graph fill](painting-textures.md#graph-fills) straight away, and a **real-world size** on Material out, so it lies on a model at a sensible scale. They are made to look right in the **hand-painted** look first (the light and shade are in the colour), and in **Realistic** too.

| Starter | One repeat | What it teaches |
|---|---|---|
| **Bricks** | 90 cm | The three steps most materials start with: a pattern box (Bricks), Bevel to round its edges, and a Gradient map to colour it by its height. Also building a grey that says what each spot is (mortar, each brick, lighter edges) before colouring. |
| **Stone tiles** | 2 m | Warp makes neat tiles look cut by hand; Edge finder paints lighter edges; **Mix materials by height** lays moss in the gaps. |
| **Cobbles** | 1 m | Cells' Edges look gives one cell per stone; Levels turns it into a shape for Bevel. Cells' **Random** output gives each stone its own colour, and Hue and saturation with a **Where** shape darkens only the stones' edges. Sand fills the gaps by height. |
| **Planks** | 1.2 m | One box with three outputs (the boards, a grey per board, the grain), each used for something different, and Blend's **Multiply** to darken the gaps. |
| **Stylised wood** | 1 m | A hand-painted look: big simple shapes, bold grain (Warp and Levels on stripes), a sharp Bevel, and bright edges painted into the colour. |
| **Bark** | 60 cm | Reusing a box for something else: Planks with wide gaps, turned upright by Transform and warped, become bark plates. Ridged Clouds adds cracks; lichen goes on by shape. |
| **Fabric** | 6 cm | A pattern from nothing but smooth stripes: two sets of stripes make a chequer (each taken from the other, then added), which decides which thread is on top. A very small real-world size. |
| **Metal plate** | 1 m | Limiting one pattern by another: rivets only near the plates' edges (Edge finder, Levels, Multiply). Metal at 1 and low roughness for shine; bright worn edges and fine scratches (Scratches) for a used look. |
| **Ground** | 2 m | Layering materials: earth from Noise, pebbles of different sizes (Scatter) mixed on by height, and grass patches by shape. |
| **Leaves** | 40 cm | The [shape canvas](shape-canvas.md) and an [alpha card](concepts.md#alpha-card): one leaf drawn with a serrated edge, a stem and veins, grouped; linked copies that each vary in size, turn, colour and teeth; **Layout → Leaf sheet** lays them out apart; **Opacity** cuts the card out and lets light through. Double-click the Shape box to see the drawing. |

**Make your own from one.** Change a starter's settings, colours or boxes, rename it, and you have your own material; open the same starter again for a fresh copy. The starters themselves never change.

### Looking through a box

Click a box's **picture** to show that box in the preview on its own, whatever comes after it: handy to see what a box adds. The picture gets an outline while you look through it. Click it again, or **Back to the material** under the preview, to go back.

### Painting inside a graph

Most boxes make their pictures from settings. A **Painted** box ([Material boxes](material-boxes.md#detail-small-things-on-top)) is painted by hand instead, with the **same brushes as the [texture painter](painting-textures.md)**: for a crack just there, moss in one corner, a darker stone, things a seed would never put in the right place. Add one from the box search (**Detail**), then:

1. **Open the painting view:** right-click the box and choose **Paint on it**, or double-click it, or click **Paint on it** in Attributes. The view takes the canvas's place.
2. **Choose what it paints** on the bar above the view: **Mask**, **Height** or **Colour** (the box's **Kind**). For a mask or a height, the brush's colour counts by how light it is: white paints in, black paints out.
3. **Paint.** The texture is shown flat, with a dimmed copy on every side: paint over an edge and the stroke comes back on the other side, so the material still [tiles seamlessly](concepts.md#seamless-tiling). Everything about the brush is the painter's: the **brush bar** floats over the view (the colour squares, the brush list, Size, Flow, Hardness, Spacing, mirror, pen, smoothing), the **paint tools** run along its bottom, the **shelf** under the canvas shows the painter's Brushes and Colours, and **Attributes** shows the brush's settings under the box's. The painter's keys work too (see [Shortcuts](#shortcuts)).
4. **Paint over something:** plug a mask, a height or a colour into the box's input of its kind, and the paint goes on top of it (by **Blend** and **Strength**, in Attributes). To see it while you paint, choose **Its input under** on the bar: it shows faintly under the paint (the **Faint** slider says how faintly). **The result under** shows the whole graph's result instead, handy for putting moss exactly on the stones you see.
5. Press **Tab** (or click **Back to the graph**) to go back to the canvas. The boxes after the Painted box follow the paint.

**Each stroke is one step to undo.** The **eraser** (**2**) takes paint off and shows what is under again; the **bucket** (**8**) fills the whole painting; **Clear** on the bar takes all the paint off (one step to undo, too). **Esc** cancels a stroke you are in the middle of, and only that.

**Resolution.** The paint is kept at its own size, chosen on the bar: 256, 512, 1024 (the standard) or 2048 pixels across. It is saved with the project and scaled to whatever size the graph is worked out at (small while you drag a slider, bigger for the preview and exports), so a 1024 painting looks right in a 2048 export, only softer. Changing the resolution scales what is painted (one step to undo). More pixels keep finer strokes but take more memory.

**On a flat texture,** some of the painter's tools work a little differently: **L** (mirror painting) flips left and right across the texture's middle, as in the UV Editor; **P** picks the colour the box shows under the pointer. Projection (**3**), polygon fill (**4**) and clone (**6**) need a model, and say so. Smudge and blur don't reach across the texture's edges. Painting the box straight onto the model in the preview isn't possible yet.

### From a photo to a material

A photo of a wall, a path or a tree's bark already holds the colours and the detail of a real surface. An **Image** box ([Material boxes](material-boxes.md#make-patterns-from-nothing)) brings one in, and four boxes after it turn it into a material that repeats without a join, each step with its own picture and settings so you can see what it did:

1. **Add an Image box** from the box search (**Make**), select it, and click **Bring in a picture…** in Attributes (or double-click the box). Pick a PNG, JPEG, TGA or BMP file. Big pictures are made smaller (2048 pixels across at most, the shape kept) and kept packed in the project, so the project doesn't need the file again. A painting of your own works the same way.
2. **Choose the square part.** A material is square, a photo usually isn't: **Square size** says how much of the picture the square takes (1 is the biggest square that fits), **Left to right** and **Top to bottom** where it sits. Leave out anything that isn't the surface: a corner of sky, a foot, a lamp post.
3. **Click Make into material.** The app adds, wired up and inside a comment frame called **From a photo**:
   - **Even out light** takes out the [light and shadow the photo was taken in](concepts.md#baked-in-light) (one side lighter, patches of shade), which would show as stripes and blotches when the texture repeats. The game lights the material instead.
   - **Make seamless** hides the joins: near the edges it shows a copy moved half a texture along, whose joins are in the middle, blended so they vanish.
   - **Guess height** guesses the bumps from how light each part is (light parts high, dark gaps low), and **Guess roughness** guesses the shine from how grainy each part is.
   - **Material from height** makes the material from the colour, the height and the roughness, into **Material out** (added if the graph has none).
   Undo takes the whole chain away in one step.
4. **Set the real-world size.** In the Image box's Attributes, **Real-world size** is how many centimetres the square part shows across: measure something in the photo (a brick is about 22 cm long, a paving slab 60 cm) and count. It is Material out's own setting, the same for the whole graph, shown here so it is set while you look at the photo.
5. **Check and tune.** The Image box's Attributes show the **seam check** of the photo (its joins show) and Make seamless's of its result (they don't); switch on the preview's [seam check](#the-seam-check) to see the joins for yourself. Then look at the lit preview: if the gaps stand up and the stones sink, tick Guess height's **Invert**; if it is too shiny or too matt, set Guess roughness's **Shiniest** and **Roughest**. [Bypass](#bypassing-a-box) a step to compare with and without it.

Why guesses: a photo shows how light each spot is, not how high or how shiny. On most stone, bark, ground and fabric what sticks out catches the light and the gaps sit in shade, so lightness is a good guess for height; busy, grainy parts are usually the rough ones. Where a photo breaks that rule, the settings put it right, or a **Levels**, **Blur** or a **Painted** height after the guess does.

**Tips for good photos:**

- **Shoot straight on**, the camera square to the surface, not at an angle: an angle makes the far side smaller and the material looks stretched when it repeats.
- **Even, soft light**: an overcast day or full shade. Hard sun makes deep shadows that Even out light can only partly take out, and that Guess height reads as holes. No flash.
- **Fill the frame with the surface**, and take more of it than you need: a wide stretch of wall gives fewer obvious repeats than one stone.
- **Avoid one big feature** (a single crack, a drain, a bright stone): it comes back in a grid when the material repeats. Choose a square part without it.
- **Sharp and plain**: no blur, no filters, no heavy editing. A phone photo is fine.
- **Your own photos, or ones you may use**: a photo from the web usually belongs to someone else.

### The preview

The **Preview** (top right) shows the graph's material: whatever goes into Material out, or the box you are looking through. With no Material out, it shows the selected box. Under the picture it says which box it shows.

Choose what to show it on:

- **Model:** the model selected in the scene (click it in **Objects**, or in the viewport behind the canvas). The material is laid by the model's [UVs](concepts.md#uvs-and-texturing), so this is how it will look once the model wears it. A model with no UVs yet can't show it: the preview says so; unwrap it in the UV mode first.
- **Ball:** a ball 1 m across, the standard.
- **Cube:** a 1 m cube, for corners and flat sides.
- **Flat:** the texture flat, repeated twice each way.

On the model, the ball and the cube, the material is shown at its **[real-world size](concepts.md#real-world-size)**: Material out's **Real-world size** says how many centimetres one repeat covers, so a 1 m stone material shows stones of the right size on a 4 m wall, and on the 1 m ball one repeat wraps a third of the way round. Change the real-world size and the preview follows. Why this matters: a material that looks right on its own can have bricks the size of a hand once it is on a model; the preview shows that straight away.

Choose how it is lit, the same way the viewport shows painted models:

- **Unlit:** the material's own colours with no light, as most hand-painted games show them.
- **Lit:** the viewport's soft studio light, with the height's bumps.
- **Toon:** cartoon shading, a lit side and a shaded side with a hard line between.
- **Realistic:** lit as a game engine lights realistic materials, with the material's roughness and metal.

An [alpha card](concepts.md#alpha-card) (a material made see-through by the **Opacity** box) shows **cut out**: on the model, the ball and the cube its see-through parts are holes, cut where they are less than half solid, as a game cuts them; **Flat** shows it over a grey checker. The light passing through it isn't shown in the preview.

The camera is fixed, looking from the front right a little above. Without a graphics card (or when it can't be used), the ball is drawn more simply and says so, and the model and cube can't be shown.

### The seam check

A material [tiles](concepts.md#seamless-tiling) when its left edge matches its right and its top matches its bottom, so copies laid side by side join without a line. Choose **Flat** and switch on **[Seam check](concepts.md#seam-check)**: the texture is repeated three times each way and the joins between the copies are marked with thin lines. Under it, the check says in plain words whether the joins show. It compares how much the picture jumps across each join with how much it changes between neighbouring pixels everywhere else; a join that jumps much more shows as a line in the game. A join that shows is marked in yellow.

Every box in the app tiles, so a graph's material passes. For an alpha card, only what shows counts: its colour where it is solid, and its opacity. Even then, look at the three-by-three for **repeats that are easy to spot**: one large stone or dark patch that comes back in a grid. More variation, or a larger real-world size, hides it.

### The texture budget

Under the preview, the **[texture budget](concepts.md#texture-budget)** says what the material will cost in a game. Choose the **Texture size** the material will be made at (256 to 4096 pixels across; it starts at the project's texture size). The budget then says:

- **Which maps** it goes out as, for the export as it is set up now (Export → Made for and its texture settings). For Godot that is the colour, an ORM (occlusion, roughness and metal in one picture) and a normal map. It counts exactly as the Export window's texture budget does. An alpha card adds its translucency map (Unity: thickness), and with separate maps its opacity.
- **What they take on the graphics card**: four bytes a pixel, and a third more for the smaller copies ([mipmaps](concepts.md#mipmaps)) games make so far-away surfaces don't shimmer.
- **How sharp it is:** pixels of texture per metre of surface at the real-world size, against the [texel density](concepts.md#texel-density) the project aims for (set in the UV Editor's **Texture** menu; see [Texture size and texel density](texture-and-density.md)).

It warns, in yellow, when one material takes more than 128 MB, and when it is much blurrier or sharper than the project aims for, naming the size that would match. A [pixel-art material](#pixel-art-materials) counts its big pixels instead and isn't compared with the project's aim. Why: a texture twice as wide takes four times the memory, so a size that matches the rest of the game keeps it both sharp and light.

The size is also the size **Export textures…** writes the material at (below). It isn't saved with the project yet. When the app says a size won't do and offers **Use … pixels across**, picking it sets this size.

### Giving the material to a shape

A graph's material can go straight onto a model, without painting. Select the shape (in Objects or the viewport), then under the preview click **Give to the selected shapes**. The shape's first material is now **made by the graph**:

- the viewport switches to painted textures and shows it at the material's real-world size, laid by the shape's [UVs](concepts.md#uvs), exactly as the preview's **Model** shows it;
- change the graph and the shape follows a moment later (the material is made again on another thread, so the window never waits);
- **Ctrl + Z** takes it back in one step, and the project saves it;
- exporting the model **bakes** it into ordinary textures with every engine preset (see [Export and import](export-and-import.md#materials-made-by-a-graph)).

Why it works this way: a material made by a graph is a [graph fill](painting-textures.md#graph-fills) layer on its own, so everything the painter does works on it too. Open Paint mode and you can paint scratches or dirt on top; the graph stays the bottom layer. If the material was already painted, the graph goes **under** the paint, which stays.

You can also give a graph from **Attributes → Materials**: each material has a **Graph…** button listing the project's graphs. That one gives the graph to that exact material (handy for a shape with several), and the list under the material then says **Made by the graph Stone**. **Graph… → No graph** takes it back; the material shows its own colour again.

A shape sharing the standard material with another first gets a material of its own (named after it), so giving a graph to one cube doesn't change the other.

### Changing a material's size

Say the bricks of the Bricks starter look too big on a small crate. There are two ways to make them smaller, for two different jobs:

- **For one shape: Tiling.** Under **Made by the graph …** in **Attributes → Materials**, the **Tiling** slider lays the material more often on that shape: **×4** makes the bricks four times smaller (sixteen bricks where there was one), **×½** twice as big. It snaps to ×½, ×1, ×2, ×4 and so on as you drag near them, and you can click the number and type one (4, or 1/2). **↺** goes back to ×1. The graph doesn't change, so the same graph can be ×1 on a big wall and ×4 on a small crate. A [graph fill](painting-textures.md#graph-fills) layer in the Paint mode has the same slider.
- **For every shape: the real-world size.** Material out's **Real-world size** says how big one repeat is everywhere the graph is used. Lower it and the bricks get smaller on every shape, but sizes in centimetres inside the graph (the mortar's width, a bevel) stay as they are, so the mortar looks thicker against the smaller bricks. Use it when the material itself is the wrong size.

Why two: [Tiling](concepts.md#tiling) shrinks the whole picture evenly, mortar and all, as tiling does in every 3D app; the real-world size is what the material *is*. Tiling is saved with the project (files with it need this version or newer to open).

**With From the model boxes:** what follows the model (moss in its crevices, light on its edges) stays where it is on the model; only the repeating parts get smaller. Moss still gathers in the same crevices, over smaller stones.

The preview's **Model** shows the selected shape's material at its Tiling, so it agrees with the viewport. The ball, the cube and Flat show the material at ×1, and **Export textures…** writes one repeat as always: Tiling is how often a shape lays it, and it is baked into that shape's textures when you export the model.

### Exporting the textures on their own

**Export textures…** writes the material as a set of **seamless picture files** you can tile anywhere: on a floor in Godot, in Blender, in another texturing app. Pick a folder; the files are made at the **Texture size** above, for the engine the Export window is **Made for**:

| Made for | Files (for a graph called Stone) |
|---|---|
| Godot | *Stone.png* (colour), *Stone_ORM.png*, *Stone_Normal.png*, *Stone_Height.png*, and *Stone.tres*: a Godot material using them, ready to drop on a mesh |
| Unity | the colour, *Stone_MaskMap.png* (Unity's [mask map](concepts.md#the-game-asset-pipeline)), the normal map and the height |
| Unreal Engine | the colour, the ORM, the normal map with green pointing down (as Unreal wants) and the height |
| Solis, Substance Painter | the colour, the ORM, the normal map and the height |
| Custom settings with **Separate maps beside it** | the colour, roughness, metal, height, normal map and occlusion each on their own |

**Alpha cards** (a material made see-through by the [Opacity](material-boxes.md#finish-whole-materials) box, such as the Leaves starter) go out with their [opacity](concepts.md#opacity) in the colour picture's **alpha** (every engine reads it there), and the light passing through them beside it:

| Made for | Extra for an alpha card | What the engine does with it |
|---|---|---|
| Godot | *Stone_Translucency.png* | *Stone.tres* is set up for you: **Transparency: Alpha Scissor** at 0.5 (sharp edges that cast shadows, the usual choice for leaves), **Cull mode: Disabled** (both sides show), and **Backlight** on with the translucency picture (light from behind shines through). |
| Unity | *Stone_Thickness.png* | Alpha clipping reads the Base Map's alpha. The thickness map is the translucency the other way round (black thin, white thick), as HDRP's transmission wants it. |
| Unreal Engine, Solis, Substance Painter | *Stone_Translucency.png* | Set the material to **Masked** and use the colour's alpha as the opacity mask; the translucency suits a two-sided foliage material's subsurface colour. |
| **Separate maps beside it** | *Stone_Opacity.png* and *Stone_Translucency.png* | The opacity as its own grey picture too. |

The translucency picture is the card's colour, darker where less light passes (black where none). The colour under the see-through parts is filled from the leaves, so no dark fringe shows when the game blurs the texture far away.

The ORM packs the material's own **occlusion** (dark in its dips), roughness and metal into one picture's red, green and blue. PNG is used unless the Export window's **Textures** row says TGA. Nothing is padded: the textures repeat seamlessly, so their edges already lead into each other. The status bar says when the files are written.

**The export check.** Above the button, anything that won't come out as the preview shows it is said in yellow, with what to do:

- A graph with no **Material out** box has no material to export: add one.
- **From the model** boxes follow a model, and a texture set on its own has none, so their stand-in of rounded bumps is baked. To bake them on the model, give the graph to the shape and export the model instead.
- Boxes that move over time ([Animated materials](#animated-materials)) are baked at the moment chosen with **Bake this moment**: exported textures can't move, so the game shows that moment standing still.
- Settings marked **Change in the game** are listed: they are baked as they are set now, and kept for the engine translators.
- Boxes made by a newer version of the app can't be worked out here.

### Pixel-art materials

A [pixel-art](concepts.md#pixel-art) game draws its textures as a few big, crisp pixels in a handful of colours. Any graph can become one: put a **Pixelate** box ([Finish](material-boxes.md#finish-whole-materials)) last, between the material and **Material out**.

1. Make the material as usual (Stone tiles, say), at a normal size.
2. Drag from the material's output (Material from height's, or Mix materials') onto empty canvas and choose **Pixelate** in the box search that opens, then drag Pixelate's **Material** output onto Material out's **Material**.
3. Set **Pixels**: how many squares across one repeat. 32 suits a floor tile a metre or two across; 16 is chunky and retro; 64 and up is finer.
4. Tick **Limit to a palette** and choose the game's [palette](concepts.md#palette) (made in the painter's Colours), with a little **Dither** for soft blends. Add **Tidy stray pixels** and an **Outline** if your art style has them.

**Why the material is worked out big first.** The boxes before Pixelate (noise, bevels, cracks) need room to make their patterns; at 16 pixels across they would be mush. So the graph is worked out at the **Texture size**, and Pixelate cuts the result into squares at the end. Each square takes the average of what is under it, then the palette's closest colour.

**What changes once a graph is pixel art** (Pixelate plugged straight into Material out, not bypassed):

- **The preview** draws the squares crisp, with no blur between them, on the 3D shapes and when Flat is drawn large.
- **The texture budget** says *Pixel art: the Pixelate box makes this material 32 × 32 big pixels*, counts the memory at that size, and gives **pixels a metre** and how many centimetres each pixel covers. It doesn't call the material blurry: big pixels are the look. Give every pixel-art material the same pixels a metre (choose Pixels and Material out's **Real-world size** together), so a pixel on the floor is as big as a pixel on the wall. For example, 32 pixels over 200 cm is 16 pixels a metre, each pixel 6.25 cm.
- **Export textures…** writes every map at **exactly** the pixel count (32 × 32 here), one texel a pixel, each texel the square's own value. The files' names are as always.
- **Godot's material** (*.tres*) is set to read the textures crisp: **Texture filter: Nearest Mipmap** (sharp squares up close, blended smaller copies far away so they don't shimmer). Nothing to set in Godot.
- **Unity, Unreal Engine and other programs** get the pictures only, so set them to stay crisp there: in **Unity**, select each texture and set **Filter Mode** to **Point (no filter)**; in **Unreal Engine**, open each texture and set **Texture Group** to **2D Pixels (unfiltered)** (or **Filter** to **Nearest**). Without it, the engine blends neighbouring texels and the pixels look soft.

Bypass the Pixelate box (its switch) to see and export the smooth material again. A Pixelate box anywhere else in the graph (with Material from height after it, say) pixelates only that part, and the export is written at the Texture size as usual: Material from height after Pixelate would work out its bumps on the screen's small pixels, making each square bumpy again.

**On a model.** A pixel-art graph given to a shape (or used as a [graph fill](painting-textures.md#graph-fills)) is laid on the model crisp, with no blend at the squares' edges. Its squares land wherever the model's UVs and size put them, so the graph fill's settings in the Paint layers panel say how they come out, for example *32 pixels a repeat, laid 2 times across this 1024-pixel texture: about 16 texels a pixel*, and which texture size would give exactly one texel a pixel. To keep them crisp in the game, turn on **Pixel art → Crisp pixels** in the Export window ([Export and import](export-and-import.md#settings)).



### Animated materials

A graph with a **moving box** (Time, Scroll, Pulse or Flow: [Material boxes](material-boxes.md#moving-boxes-that-change-over-time)) can be watched moving in the preview. Under the preview's picture a **player** appears:

- **Play** / **Pause** plays the moving boxes, in the preview and on the boxes' own pictures, and stops them where they are.
- **Back to the start** goes to 0 seconds.
- **Time** (a slider, 0 to 60 s): drag to see any moment; it plays on from there. Playing starts again from 0 after a minute.

While it plays, the pictures are worked out a little smaller (as while you drag a slider), on another thread, so the window never waits for them. Only the moving boxes and what comes after them are worked out again each frame; everything before them is kept. A big graph may play at fewer frames a second, but always at its real speed. The player has no keys: Space already opens the box search over the canvas.

**What gets baked.** Exported textures, the painter's [graph fills](painting-textures.md#graph-fills) and shapes given the material are still pictures, so they show one **moment** of the animation: 0 seconds to start with. Under the player it says which. To bake another moment, stop the player there (or drag Time to it) and click **Bake this moment**; it is one step to undo, and every shape and paint layer using the graph follows. The graph's Attributes (nothing selected) say the moment too. The export check says the moving boxes are baked at that moment.

**Why it moves only in the app, for now.** A game can only move a material with a [shader](concepts.md#shader): a small program the engine runs. The app never runs engine shaders itself: it works its own graphs out. Translators that write a Godot (and Unity and Unreal) shader from a graph come later, all three together. Until then, animated materials move in the app's preview and go out baked.

**Change in the game.** Some sliders are worth changing while the game runs: a glow brightening as the player comes near, a river speeding up after rain. Select a box and, under **Change in the game** in Attributes, tick those sliders (numbers, colours and switches; a choice decides how the box works and is always baked). Each tick is a step to undo. They are saved with the graph and listed in the export check and the graph's Attributes, ready for the translators, which will make them settings the game can change. For now they are baked as they are set.

### Using a graph in the painter

A graph can paint a model as a layer in the painter: a [graph fill](painting-textures.md#graph-fills). In Paint mode, click **···** → **+ Graph fill** in the Paint layers panel and choose the graph, or click the graph's card in the shelf's **Materials** tab. The layer is made from the graph on the model, at the graph's real-world size, and made again whenever you change the graph here: come back to Paint mode and it has followed.

Graphs using the **From the model** boxes ([Material boxes](material-boxes.md#from-the-model-following-the-model-you-paint)) follow the model they paint: moss in its crevices, light on its edges. Here they show a stand-in of rounded bumps.

### Showing settings in the painter

Some settings are worth changing per layer: the moss's colour, how big the stones are. Select a box, and under **Shown in the painter** in Attributes, tick those settings. Every graph fill using the graph then shows them under its own settings, where each layer can set them its own way without changing the graph (or the other layers using it). Ticking one is a step to undo.

### Variations and the dice

One graph can give several looks: a wall that is **Clean** in one room, **Mossy** by the river and **Ruined** at the gate. Rather than copying the graph three times (and changing all three every time you improve it), keep its looks as **[variations](concepts.md#variation)**: named looks of the same graph that you flip between.

**What a variation keeps:** the settings ticked **Shown in the painter** ([above](#showing-settings-in-the-painter)), every **seed** in the graph (each box's own and the graph's), and the [moment](#animated-materials) an animation is baked at. It doesn't keep which boxes are [bypassed](#bypassing-a-box): bypass is for comparing while you work, and a look that quietly switched boxes off would be easy to forget. It doesn't keep a shape's **Tiling** either: that belongs to each shape and layer.

1. Tick **Shown in the painter** on the settings that make the looks differ (the moss's amount, the cracks' depth), and set them for the first look.
2. Click **Variations** on the bar above the canvas → **Save as a new variation**. It is called **Variation 1**; rename it in Attributes (click the empty canvas so nothing is selected), say **Clean**.
3. Change the settings for the next look, and save it the same way.
4. Flip between them with **Variations** → a variation's name. The bar shows the one the graph looks as now ("Variations: Mossy"); change anything a variation keeps and it no longer matches.

With nothing selected, **Attributes** lists the graph's variations: type in a name to rename it; **Use** makes the graph look as it says (the one in use says **In use**); **Update** keeps the graph's look as it is now in it; **Delete** removes it. Each of these, and each use, is one step to undo. A graph holds up to 100 variations.

**The dice.** Every random pattern in a graph (noise, the tiles' wear, scattered pebbles) comes from a **[seed](concepts.md#seed)**, a number. The **dice** button on the bar (a small die, right of **Variations**) gives every seed in the graph a new number at once: a fresh take on the same look, the same kind of stones laid differently. Not this one? Click again, or undo. To roll one box's pattern only, right-click it → **Roll its seed**. Your own boxes' seeds roll too when a seed is one of their sliders. Like a look you rolled? Save it as a variation.

**In the painter:** a [graph fill](painting-textures.md#graph-fills) layer and a material given the graph each pick a **Variation**, so the same graph shows Clean on one shape and Mossy on another. When you update a variation, every layer using it is made again.

There are no keys for these: the menu, the dice and the buttons in Attributes.

### Your own boxes

When a few boxes always go together (a pattern warped and bevelled into stones, say), you can make them into **one box of your own**, with its own name, its own sliders and its own "What does this do?". The graph gets shorter and easier to read, and the box can be used again, in this graph, in others, or in other projects. Other programs call this a group node or a sub-graph. See [own box](concepts.md#own-box).

**Make one.**

1. Select the boxes (click one, **Ctrl** + click more, or drag round them).
2. Press **Alt + G**. The **Make your own box** window opens:
   - **Name** and **What does this do?**: in your own words. They show on the box, in the box search and on the shelf, so write them for whoever uses it later (likely you, in a month).
   - **Inputs**: one for every wire coming into the selection from outside, **Outputs**: one for every wire going out. (If nothing goes out, the selected boxes at the end give out.) Untick one to leave it out, or give it a clearer name.
   - **Sliders**: every setting of the selected boxes, box by box. Tick the ones you want to change from outside, and name them. Settings you showed in the painter start ticked.
3. Click **Make it**. The boxes become one box, wired as they were, and the picture doesn't change. One **Ctrl + Z** takes it back.

**See inside.** Double-click your box (or click **Open it** in Attributes). The canvas shows the boxes inside, with their live pictures following what is plugged into your box. The bar above says where you are (the graph › your box); click **‹ Back out**, the graph's name, or press **Tab** to go back out, one box at a time. (Tab is the app's "back" key; in the Material mode it never takes you out of the mode, so you can press it freely. Esc is kept for cancelling: it closes the box search or a note and cancels a drag, but doesn't take you out of a box.) Inside, everything works as on any graph, and with nothing selected Attributes shows your box's name, words, picture (**Take its picture again** uses what its first output makes now), plugs and sliders. Select a box inside to tick which of its settings are sliders (**Slider:** ticks), or to make a plug give or take from that box instead.

**Linked: change one, change all.** Every box made from the same own box is **linked**: change the boxes inside, and every one of them in the project changes, because they are one box placed several times. Each keeps its own slider values, so they can still differ where you chose. (This is the same idea as [instances](copies-and-instances.md) of objects.)

- **Alt + C** then **Alt + V** pastes **linked copies** of your own boxes. **Ctrl + V** pastes a copy of its own instead ("Rocky copy"), which you can change on its own. The app's own boxes (Noise, Levels…) have nothing inside to share, so they are always pasted as plain copies.
- **Into another project:** boxes copied in one [tab](working-on-several-projects.md) paste in another, and your own boxes among them come along (with the boxes inside them), becoming that project's own boxes. If that project already has the very same box, it is used as it is; if it has a *different* box that happens to share its inner id (say, one you changed there), it is kept and the one coming in becomes a box of its own, with a number after its name if needed ("Rocky 2"). Links can't reach from one project into another, so **Alt + V** there pastes the same as Ctrl + V, and the status bar says so. A [Palette map](material-boxes.md#colour) among them brings the palette it uses along, unless that project already has a palette by that name: then it colours with that project's palette, and the status bar says so.

**Open it back up.** Select your box and press **Shift + G** (or **Open it back up** in Attributes): its boxes go back into the graph in its place, with its slider values, wired as it was. The box stays on the shelf to use again.

**The shelf: this project and your library.** In the Material mode the **shelf** sits under the canvas; its **Your own boxes** tab (beside **Starter graphs**) has two rows. **In this project** shows the project's own boxes, saved with the project; **Your library** shows boxes you keep for every project, saved with the app's settings. Click a card to place that box in the middle of the canvas. Right-click a card for more: **Place it**, **Open it**, **Keep in your library**, **Export as a box file…**, **Delete from the project** (only when no graph uses it) or **Remove from your library**.

Your library holds *copies*: placing one brings it into the project, where it is linked as above. Changing it in a project doesn't change your library until you **Keep in your library** again (which replaces it). If the project already holds its own changed version, that one is placed, and the status bar says so.

**Share them as files.** **Export as a box file…** writes a box (with any own boxes inside it) to a `.matbox` file, plain text you can send to someone. **Bring in a box file…** (top right of the shelf) adds one to your library.

**What can't go inside, and why.** The app says why when you can't make the selection into a box:

- **Material out** is where the graph's material comes out, so it stays outside.
- **From the model** boxes read the model's own maps in the painter, while a box of yours works on one repeat of the material: keep them outside and plug them into your box.
- A box outside the selection that is fed by it *and* feeds it would make your box feed itself in a loop. Select it too (the note offers **Look at** it).
- A box can't hold itself, even through other boxes of yours, and boxes of yours go at most eight deep inside each other.

## Settings

| Setting | What it does |
|---|---|
| **Name** (Attributes, nothing selected) | The graph's name, as listed above the canvas. |
| **Seed** (Attributes, nothing selected) | Changes every random pattern in the graph at once, keeping their look: try a few for a variation of the same material. |
| **Variations** (the bar above the canvas, and Attributes with nothing selected) | Named looks of the graph to flip between, save, rename, update and delete. See [Variations and the dice](#variations-and-the-dice). |
| **Dice** (the bar above the canvas) and **Roll the dice** (Attributes, nothing selected) | Gives every seed in the graph a new number. |
| **Variation** (Attributes → Materials, a material made by a graph) | Which of the graph's variations the material shows, or **As the graph is**. |
| A box's settings | Each box has its own ([Material boxes](material-boxes.md) lists them); rest the mouse on one for what it does. A box's **Seed** changes only that box's pattern. Sizes are in centimetres on the finished material, through Material out's **Real-world size**. |
| **Title**, **Colour**, **Note** (a frame selected) | The frame's title, colour, and a longer note to remind you what that part of the graph does (shown under the title on the canvas, too). |
| **Starter graphs** (above the canvas) | Opens the window of starter graphs; click one to open a copy of it in the project. |
| **Model** / **Ball** / **Cube** / **Flat** (Preview) | What the preview shows the material on. |
| **Unlit** / **Lit** / **Toon** / **Realistic** (Preview) | How the model, ball and cube are lit. |
| **Mask** / **Height** / **Colour**, resolution, **… under** and **Faint**, **Clear** (the painting view's bar) | What a Painted box paints, how many pixels across its paint is kept at, what shows faintly under the paint, and taking all the paint off. See [Painting inside a graph](#painting-inside-a-graph). |
| **Bring in a picture…**, **Real-world size**, **Make into material** (Attributes, an Image box selected) | Brings a photo or painting in through the open dialog; sets how many centimetres it shows across (Material out's size); adds the boxes turning it into a material. See [From a photo to a material](#from-a-photo-to-a-material). |
| **Seam check** (Preview, Flat) | Repeats the texture three by three, marks the joins and says whether they show. |
| **Tiling** (Attributes → Materials, a material made by a graph) | How many times more often the material repeats on that shape: ×⅛ to ×32, snapping to ×½, ×1, ×2, ×4… The graph doesn't change. See [Changing a material's size](#changing-a-materials-size). |
| **Texture size** (Texture budget) | The size the budget is worked out for, and the size **Export textures…** writes, 256 to 4096 pixels. |
| **Give to the selected shapes** (under the preview) | Makes each selected shape's first material from the graph, without painting. |
| **Export textures…** (under the preview) | Writes the material as seamless picture files, made for the Export window's **Made for**. |
| **Graph…** (Attributes → Materials) | Makes that material from one of the project's graphs, or **No graph** to take it back. |
| **Find in the graph** (the bar above the canvas) | Finds boxes and comment frames by name and goes to them. See [Finding a box](#finding-a-box). |
| **View → Minimap** (the bar above the canvas) | Shows the small map of a big graph in the canvas's corner. See [The minimap](#the-minimap). |
| **Shown in the painter** (a box selected) | Which of the box's settings every [graph fill](painting-textures.md#graph-fills) using the graph shows, to set its own way. |
| **Bypassed** (a box selected) | Switches the box off for now: what goes in passes straight through. See [Bypassing a box](#bypassing-a-box). |
| **Change in the game** (a box selected) | Which of the box's sliders a game should be able to change while it runs; kept for the engine translators, baked for now. See [Animated materials](#animated-materials). |
| **Play** / **Pause**, **Back to the start**, **Time** (Preview, a graph that moves) | Play the moving boxes in the preview, go back to 0 seconds, or see any moment. |
| **Bake this moment** (Preview, a graph that moves) | Makes the player's moment the one exports, the painter and shapes given the material show. |
| **Name**, **What does this do?**, **Picture** (inside your own box, nothing selected) | Your box's name and words, as the box search and shelf show them, and its picture (**Take its picture again**). |
| **Plugs**, **Sliders** (inside your own box, nothing selected) | Its plugs' and sliders' names; **×** takes a slider off. |
| **Slider:** ticks (a box inside your own box selected) | Which of that box's settings are sliders on your box. |

## Shortcuts

These work with the mouse over the canvas.

| Action | Shortcut |
|---|---|
| Add a box (the box search) | `Space`, or right-click on empty canvas |
| Connect | Drag from a dot to a dot |
| Disconnect | Drag a wire's end away from its dot |
| Select / add / take out | Click / `Ctrl` + click / `Shift` + click; drag on empty canvas round several |
| Move boxes | Drag a box |
| Pan / zoom | Middle drag / the wheel |
| Frame the selected boxes (or everything) | `Z` |
| Comment frame round the selected boxes | `C` |
| Remove the selected boxes | `Delete` |
| Remove the selected boxes but keep the wire through them | `Backspace` |
| Copy / paste the selected boxes (with the wires between them) | `Ctrl + C` / `Ctrl + V` |
| Linked copy of your own boxes | `Alt + C` / `Alt + V` (the app's own boxes are pasted as plain copies) |
| Make your own box of the selected boxes / open it back up | `Alt + G` / `Shift + G` |
| See inside your own box | Double-click it |
| Look through a box | Click its picture |
| Bypass a box, or switch it back on | Click the switch on its title, or right-click it → **Bypass** (no key yet) |
| A box's menu (Paint on it, Bypass, Roll its seed, Find others like it, Remove) | Right-click it |
| Open a Painted box's painting view | Right-click it → **Paint on it**, or double-click it |
| Bring a picture in on an Image box | Double-click it, or **Bring in a picture…** in Attributes |
| Use or save a variation | **Variations** above the canvas (no key) |
| Roll every seed / one box's seed | The dice above the canvas / right-click the box → **Roll its seed** (no key) |
| Find a box or frame by name | Type in **Find in the graph** above the canvas (no key yet) |
| Next match | **›**, or `Enter` in the field |
| Previous match | **‹** |
| Empty the find field | `Esc` in the field |
| Move the view on a big graph | Click or drag on the minimap |
| Add a reroute dot to a wire | Click the wire |
| Move a reroute dot | Drag it |
| Remove the selected reroute dot | `Delete` (click a dot to select it) |
| Close the box search, a note, a box's menu or the Make your own box window; cancel a drag | `Esc` |
| Come back out of your own box, one level at a time (Tab never leaves the Material mode: choose the Standard tab or another mode for that) | `Tab` |
| Undo / redo any change | `Ctrl + Z` / `Ctrl + Y` |

On the [painting view](#painting-inside-a-graph), the texture painter's own controls work, with the mouse over the painting:

| Action | Shortcut |
|---|---|
| Paint | Left drag |
| A straight line from where the last stroke ended | `Shift` + click |
| Brush size and hardness / flow | `Ctrl` + right drag (sideways size, up and down hardness) / `Ctrl` + left drag (sideways flow) |
| Brush size smaller / bigger | `[` / `]` |
| Paint brush / eraser / smudge / blur / bucket (fills the whole painting) | `1` / `2` / `5` / `7` / `8` |
| Pick the colour under the pointer | `P` |
| Mirror painting (left and right across the texture's middle) on or off | `L` |
| Smoothing on or off | `D` |
| Pan / zoom | Middle drag / the wheel |
| Cancel the stroke under way | `Esc` |
| Back to the graph | `Tab` |

## Tips and common problems

- **Problem:** Bring in a picture says the file can't be read. **Why:** the app reads PNG, JPEG, TGA and BMP; other kinds (HEIC from some phones, WebP, RAW) or a damaged file can't be read, and the note says which. **Fix:** save it as a JPEG or PNG in any photo app and bring that in.
- **Problem:** the material made from a photo still shows a lighter and a darker side. **Why:** Even out light only takes out patches bigger than its **Light patch size**. **Fix:** lower **Light patch size**, or take the photo in even light.
- **Problem:** Space opened the command search, not the box search. **Why:** the mouse wasn't over the canvas. **Fix:** point at the canvas first. Away from the canvas, Space and Delete keep their usual jobs.
- **Problem:** a box shows "…" instead of a picture. **Why:** its picture is still being worked out; the bar above the canvas says "Working out the pictures…". A big graph can take a moment, and the window keeps working meanwhile.
- **Problem:** a box I dropped jumped to the right. **Why:** it was left of a box that feeds it, and wires always run left to right. Move the box that feeds it further left instead.
- **Tip:** the preview's **Seam check** marks where the copies meet and says whether the joins show. Every box in the app is made to tile, so a join that shows means something is off.
- **Problem:** the material looks tiny (or huge) on my model. **Why:** the preview uses Material out's **Real-world size**, the material's **Tiling** and the model's UVs. **Fix:** set the real-world size to how big one repeat should be in the game; if only one model should show it smaller or bigger, set its material's **Tiling** ([Changing a material's size](#changing-a-materials-size)); if only one model looks wrong for no reason, its UVs may be scaled differently from the rest (check them with the [checker](checker-and-stretch.md)).
- **Problem:** Model shows nothing. **Why:** nothing is selected, or the selected model has no UVs. **Fix:** select a model in Objects; unwrap it in the UV mode if the preview says it has no UVs.
- **Problem:** I gave a shape the material but it still looks plain. **Why:** it is still being made (a big graph takes a moment), or the viewport isn't showing painted textures. **Fix:** wait a moment; check **Display → Surface → Painted**. A shape without UVs has nothing to lay the material by: unwrap it in the UV mode.
- **Problem:** Export textures… is greyed out. **Why:** the export check above it says what stops it (most often, no Material out box). **Fix:** do what the check says.
- **Problem:** Alt + G says a box can't go inside. **Why:** see *What can't go inside, and why* above; the note says which box and what to do.
- **Problem:** I changed my box inside and boxes in another graph changed too. **Why:** boxes of yours are linked across the project. **Fix:** to change just one, copy it with **Ctrl + C / Ctrl + V** (a copy of its own) and change that.
- **Problem:** a box of mine says nothing gives its output any more. **Why:** the box inside that gave it was removed. **Fix:** undo, or go inside, select a box and click **Give … from this box**.
- **Tip:** a starter graph's notes don't show. **Why:** you are zoomed too far out to read them. **Fix:** zoom in (the wheel), or select the frame's title and read its note in Attributes.
- **Problem:** opening a starter made "Bricks 2". **Why:** the project already has a graph called Bricks (maybe the copy you opened before). Each copy is separate; rename it as you like.
- **Problem:** Esc doesn't take me out of my own box. **Why:** in the Material mode **Tab** is the "back" key, and Esc only cancels things (the box search, a note, a drag). **Fix:** press **Tab**, or click **‹ Back out** above the canvas. To leave the Material mode itself, click the Standard tab or another mode.
- **Problem:** my animated material stands still in the viewport and in Godot. **Why:** textures are still pictures; only the preview plays moving boxes, until the engine translators come. **Fix:** choose the moment to show with **Bake this moment**.
- **Problem:** the preview plays jerkily. **Why:** a big graph takes longer to work out than a frame; the player keeps to the real time and shows fewer frames. **Fix:** look through a box nearer the moving one, or make the graph lighter while you tune the movement.
- **Problem:** Delete removed a reroute dot, not my box. **Why:** a dot was selected (it has a ring), and Delete removes the selected dot first. **Fix:** click the box, then press Delete.
- **Problem:** the boxes after a bypassed box show a **!**. **Why:** the bypassed box had nothing of the right kind to pass on (it makes that itself, or nothing of that kind is plugged into it). **Fix:** click the **!** and pick **Switch … back on**, or plug something of that kind into the bypassed box.
- **Problem:** my material looks different in the game from what I expected. **Why:** a box may still be bypassed: exports bake the graph as it is. **Fix:** the export check lists bypassed boxes; switch them back on.
- **Problem:** I can't see the minimap. **Why:** the whole graph fits on screen (it only shows when part of the graph is off screen), the canvas is very small, or it is switched off. **Fix:** zoom in or pan; tick **View → Minimap** above the canvas.
- **Problem:** Find says "No match" for a box I can see. **Why:** every word typed must be in its name, a choice it is set to, or a frame's title or note; numbers and setting names aren't searched. **Fix:** type fewer words, or part of the box's name.
- **Problem:** I painted on a Painted box but the boxes after it didn't change. **Why:** they follow when the stroke ends (you let go), and only the output of the box's **Kind** gives the paint. **Fix:** let go; check the wire comes from the output named like its Kind (or change the Kind on the painting view's bar).
- **Problem:** Tab didn't go back to the graph from the painting view. **Why:** a stroke was still under way. **Fix:** let go of the mouse (or press **Esc** to cancel the stroke), then Tab.
- **Problem:** my strokes look soft in the preview. **Why:** the painting is kept at fewer pixels than the texture is made at. **Fix:** choose a bigger resolution on the painting view's bar (it scales what is painted already, then paint the fine parts again).
- **Tip:** the Keys window (**Keys** on the menu bar) shows the canvas's keys in the Material mode's green.

## Related

- [Material boxes](material-boxes.md): every box, in plain words
- [Concepts: material graphs](concepts.md#material-graphs)
- [Painting textures](painting-textures.md), for painting by hand, and [graph fills](painting-textures.md#graph-fills) for painting with a graph
- [Materials](materials.md), for giving parts of a model their own material
- [Export and import](export-and-import.md#materials-made-by-a-graph): how materials made by a graph go out
- [Interface tour: the Material layout](interface-tour.md#layouts-and-panels)
- [Keyboard shortcuts](keyboard-shortcuts.md)
