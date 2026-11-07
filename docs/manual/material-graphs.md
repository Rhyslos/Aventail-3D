# Material graphs

> Build a seamless material from **boxes joined by wires** on the node canvas, in the **Material** mode, and see it beside it on your model, a ball, a cube or tiled flat.

A [material graph](concepts.md#material-graphs) makes a texture from nothing: a box makes a pattern (noise, tiles), the next shapes it, another colours it, and the last turns it into a whole material (colour, bumps, shine) ready for a game. Every box shows a small live picture of what it makes, so you can see each step. The textures it makes **tile seamlessly**: their left edge matches their right, and their top their bottom.

This is the start of the app's material graphs (Phase 12). The first set of boxes is in: patterns (Noise, Clouds, Cells, Bricks, Tiles, Planks and more), boxes that shape them (Warp, Bevel, Blur…), colour them (Gradient map, Hue and saturation) and finish them (Material from height, Mix materials). [Material boxes](material-boxes.md) lists every one in plain words. Starter graphs and using a graph in the painter follow.

## When to use it

- You want a material that repeats over a big surface without seams: stone tiles, bricks, ground, wood.
- You want to change a material later by moving a slider rather than painting it again.
- For one-off details on one model (a scratch here, a logo there), [painting](painting-textures.md) is quicker.

## How to use it

1. Click **Material** on the mode palette on the left (under Paint), or the **Material** tab at the top right. The window changes to the Material layout: the **Material graph** canvas in the middle, the **Preview** at the top right and **Attributes** under it. The viewport is still there, as a tab behind the canvas.
2. Click **New graph** (above the canvas). A new graph starts with one box, **Material out**: whatever is plugged into it is the graph's material. Graphs are saved in the project, and the list above the canvas switches between them; **Rename** and **Delete graph** are beside it.
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

### Comment frames

A comment frame is a coloured box with a title, for grouping boxes ("Stones", "Moss in the gaps"), so a big graph reads as its parts.

- Select some boxes and press **C**: a frame goes round them, its title ready to type (press **Enter** when done). With nothing selected, **C** puts an empty frame at the pointer.
- **Drag its title** to move the frame with every box inside it. A box is inside a frame when its top left corner is.
- **Drag its edges** (left, right, bottom) to make it bigger or smaller.
- **Double-click its title** to rename it.
- Click its title to select it: its **colour** and a longer **note** are in Attributes. **Delete** removes the frame, not its boxes.
- Frames can sit inside frames, and their titles stay readable however far out you zoom.

### Looking through a box

Click a box's **picture** to show that box in the preview on its own, whatever comes after it: handy to see what a box adds. The picture gets an outline while you look through it. Click it again, or **Back to the material** under the preview, to go back.

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

The camera is fixed, looking from the front right a little above. Without a graphics card (or when it can't be used), the ball is drawn more simply and says so, and the model and cube can't be shown.

### The seam check

A material [tiles](concepts.md#seamless-tiling) when its left edge matches its right and its top matches its bottom, so copies laid side by side join without a line. Choose **Flat** and switch on **[Seam check](concepts.md#seam-check)**: the texture is repeated three times each way and the joins between the copies are marked with thin lines. Under it, the check says in plain words whether the joins show. It compares how much the picture jumps across each join with how much it changes between neighbouring pixels everywhere else; a join that jumps much more shows as a line in the game. A join that shows is marked in yellow.

Every box in the app tiles, so a graph's material passes. Even then, look at the three-by-three for **repeats that are easy to spot**: one large stone or dark patch that comes back in a grid. More variation, or a larger real-world size, hides it.

### The texture budget

Under the preview, the **[texture budget](concepts.md#texture-budget)** says what the material will cost in a game. Choose the **Texture size** the material will be made at (256 to 4096 pixels across; it starts at the project's texture size). The budget then says:

- **Which maps** it goes out as, for the export as it is set up now (Export → Made for and its texture settings). For Godot that is the colour, an ORM (occlusion, roughness and metal in one picture) and a normal map. It counts exactly as the Export window's texture budget does.
- **What they take on the graphics card**: four bytes a pixel, and a third more for the smaller copies ([mipmaps](concepts.md#mipmaps)) games make so far-away surfaces don't shimmer.
- **How sharp it is:** pixels of texture per metre of surface at the real-world size, against the [texel density](concepts.md#texel-density) the project aims for (set in the UV Editor's **Texture** menu; see [Texture size and texel density](texture-and-density.md)).

It warns, in yellow, when one material takes more than 128 MB, and when it is much blurrier or sharper than the project aims for, naming the size that would match. Why: a texture twice as wide takes four times the memory, so a size that matches the rest of the game keeps it both sharp and light.

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

### Exporting the textures on their own

**Export textures…** writes the material as a set of **seamless picture files** you can tile anywhere: on a floor in Godot, in Blender, in another texturing app. Pick a folder; the files are made at the **Texture size** above, for the engine the Export window is **Made for**:

| Made for | Files (for a graph called Stone) |
|---|---|
| Godot | *Stone.png* (colour), *Stone_ORM.png*, *Stone_Normal.png*, *Stone_Height.png*, and *Stone.tres*: a Godot material using them, ready to drop on a mesh |
| Unity | the colour, *Stone_MaskMap.png* (Unity's [mask map](concepts.md#the-game-asset-pipeline)), the normal map and the height |
| Unreal Engine | the colour, the ORM, the normal map with green pointing down (as Unreal wants) and the height |
| Solis, Substance Painter | the colour, the ORM, the normal map and the height |
| Custom settings with **Separate maps beside it** | the colour, roughness, metal, height, normal map and occlusion each on their own |

The ORM packs the material's own **occlusion** (dark in its dips), roughness and metal into one picture's red, green and blue. PNG is used unless the Export window's **Textures** row says TGA. Nothing is padded: the textures repeat seamlessly, so their edges already lead into each other. The status bar says when the files are written.

**The export check.** Above the button, anything that won't come out as the preview shows it is said in yellow, with what to do:

- A graph with no **Material out** box has no material to export: add one.
- **From the model** boxes follow a model, and a texture set on its own has none, so their stand-in of rounded bumps is baked. To bake them on the model, give the graph to the shape and export the model instead.
- Boxes that move over time (coming in a later milestone) are baked as they are at that moment: exported textures can't move.
- Boxes made by a newer version of the app can't be worked out here.



### Using a graph in the painter

A graph can paint a model as a layer in the painter: a [graph fill](painting-textures.md#graph-fills). In Paint mode, click **···** → **+ Graph fill** in the Paint layers panel and choose the graph, or click the graph's card in the shelf's **Materials** tab. The layer is made from the graph on the model, at the graph's real-world size, and made again whenever you change the graph here: come back to Paint mode and it has followed.

Graphs using the **From the model** boxes ([Material boxes](material-boxes.md#from-the-model-following-the-model-you-paint)) follow the model they paint: moss in its crevices, light on its edges. Here they show a stand-in of rounded bumps.

### Showing settings in the painter

Some settings are worth changing per layer: the moss's colour, how big the stones are. Select a box, and under **Shown in the painter** in Attributes, tick those settings. Every graph fill using the graph then shows them under its own settings, where each layer can set them its own way without changing the graph (or the other layers using it). Ticking one is a step to undo.

## Settings

| Setting | What it does |
|---|---|
| **Name** (Attributes, nothing selected) | The graph's name, as listed above the canvas. |
| **Seed** (Attributes, nothing selected) | Changes every random pattern in the graph at once, keeping their look: try a few for a variation of the same material. |
| A box's settings | Each box has its own ([Material boxes](material-boxes.md) lists them); rest the mouse on one for what it does. A box's **Seed** changes only that box's pattern. Sizes are in centimetres on the finished material, through Material out's **Real-world size**. |
| **Title**, **Colour**, **Note** (a frame selected) | The frame's title, colour, and a longer note to remind you what that part of the graph does. |
| **Model** / **Ball** / **Cube** / **Flat** (Preview) | What the preview shows the material on. |
| **Unlit** / **Lit** / **Toon** / **Realistic** (Preview) | How the model, ball and cube are lit. |
| **Seam check** (Preview, Flat) | Repeats the texture three by three, marks the joins and says whether they show. |
| **Texture size** (Texture budget) | The size the budget is worked out for, and the size **Export textures…** writes, 256 to 4096 pixels. |
| **Give to the selected shapes** (under the preview) | Makes each selected shape's first material from the graph, without painting. |
| **Export textures…** (under the preview) | Writes the material as seamless picture files, made for the Export window's **Made for**. |
| **Graph…** (Attributes → Materials) | Makes that material from one of the project's graphs, or **No graph** to take it back. |
| **Shown in the painter** (a box selected) | Which of the box's settings every [graph fill](painting-textures.md#graph-fills) using the graph shows, to set its own way. |

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
| Linked copy | `Alt + C` / `Alt + V`: for now these paste plain copies, and say so; linked boxes come with your own boxes |
| Make your own box / open it | `Alt + G` / `Shift + G`: these come later in Phase 12; for now the status bar says so |
| Look through a box | Click its picture |
| Close the box search or a note; cancel a drag | `Esc` |
| Undo / redo any change | `Ctrl + Z` / `Ctrl + Y` |

## Tips and common problems

- **Problem:** Space opened the command search, not the box search. **Why:** the mouse wasn't over the canvas. **Fix:** point at the canvas first. Away from the canvas, Space and Delete keep their usual jobs.
- **Problem:** a box shows "…" instead of a picture. **Why:** its picture is still being worked out; the bar above the canvas says "Working out the pictures…". A big graph can take a moment, and the window keeps working meanwhile.
- **Problem:** a box I dropped jumped to the right. **Why:** it was left of a box that feeds it, and wires always run left to right. Move the box that feeds it further left instead.
- **Tip:** the preview's **Seam check** marks where the copies meet and says whether the joins show. Every box in the app is made to tile, so a join that shows means something is off.
- **Problem:** the material looks tiny (or huge) on my model. **Why:** the preview uses Material out's **Real-world size** and the model's UVs. **Fix:** set the real-world size to how big one repeat should be in the game; if only one model looks wrong, its UVs may be scaled differently from the rest (check them with the [checker](checker-and-stretch.md)).
- **Problem:** Model shows nothing. **Why:** nothing is selected, or the selected model has no UVs. **Fix:** select a model in Objects; unwrap it in the UV mode if the preview says it has no UVs.
- **Problem:** I gave a shape the material but it still looks plain. **Why:** it is still being made (a big graph takes a moment), or the viewport isn't showing painted textures. **Fix:** wait a moment; check **Display → Surface → Painted**. A shape without UVs has nothing to lay the material by: unwrap it in the UV mode.
- **Problem:** Export textures… is greyed out. **Why:** the export check above it says what stops it (most often, no Material out box). **Fix:** do what the check says.
- **Tip:** the Keys window (**Keys** on the menu bar) shows the canvas's keys in the Material mode's green.

## Related

- [Material boxes](material-boxes.md): every box, in plain words
- [Concepts: material graphs](concepts.md#material-graphs)
- [Painting textures](painting-textures.md), for painting by hand, and [graph fills](painting-textures.md#graph-fills) for painting with a graph
- [Materials](materials.md), for giving parts of a model their own material
- [Export and import](export-and-import.md#materials-made-by-a-graph): how materials made by a graph go out
- [Interface tour: the Material layout](interface-tour.md#layouts-and-panels)
- [Keyboard shortcuts](keyboard-shortcuts.md)
