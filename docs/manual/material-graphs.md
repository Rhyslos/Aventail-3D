# Material graphs

> Build a seamless material from **boxes joined by wires** on the node canvas, in the **Material** mode, and see it on a ball or tiled flat beside it.

A [material graph](concepts.md#material-graphs) makes a texture from nothing: a box makes a pattern (noise, tiles), the next shapes it, another colours it, and the last turns it into a whole material (colour, bumps, shine) ready for a game. Every box shows a small live picture of what it makes, so you can see each step. The textures it makes **tile seamlessly**: their left edge matches their right, and their top their bottom.

This is the start of the app's material graphs (Phase 12). The first set of boxes is in: patterns (Noise, Clouds, Cells, Bricks, Tiles, Planks and more), boxes that shape them (Warp, Bevel, Blur…), colour them (Gradient map, Hue and saturation) and finish them (Material from height, Mix materials). [Material boxes](material-boxes.md) lists every one in plain words. Starter graphs and using a graph in the painter follow.

## When to use it

- You want a material that repeats over a big surface without seams: stone tiles, bricks, ground, wood.
- You want to change a material later by moving a slider rather than painting it again.
- For one-off details on one model (a scratch here, a logo there), [painting](painting-textures.md) is quicker.

## How to use it

1. Click **Material** on the mode palette on the left (under Paint), or the **Material** tab at the top right. The window changes to the Material layout: the **Material graph** canvas in the middle, the **Preview** at the top right and **Attributes** under it. The viewport is still there, as a tab behind the canvas.
2. Click **New graph** (above the canvas). A new graph starts with one box, **Material out**: whatever is plugged into it is the graph's material. Graphs are saved in the project, and the list above the canvas switches between them; **Rename** and **Delete graph** are beside it.
3. **Add a box:** point at the canvas and press **Space**, or right-click on an empty part of it. The box search opens at the pointer. Type a few letters ("noi"), or pick from the list, and click a box or press **Enter**. Boxes are listed in the order experts work in: **Make** (patterns from nothing), **Shape**, **Detail**, **Colour**, then **Finish** (whole materials).
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

The **Preview** shows the graph's material (what goes into Material out) on a lit **Ball**, or **Flat**, repeated twice each way so you can check that it tiles. Under it, it says which box it shows. With no Material out, it shows the selected box.

## Settings

| Setting | What it does |
|---|---|
| **Name** (Attributes, nothing selected) | The graph's name, as listed above the canvas. |
| **Seed** (Attributes, nothing selected) | Changes every random pattern in the graph at once, keeping their look: try a few for a variation of the same material. |
| A box's settings | Each box has its own ([Material boxes](material-boxes.md) lists them); rest the mouse on one for what it does. A box's **Seed** changes only that box's pattern. Sizes are in centimetres on the finished material, through Material out's **Real-world size**. |
| **Title**, **Colour**, **Note** (a frame selected) | The frame's title, colour, and a longer note to remind you what that part of the graph does. |
| **Ball** / **Flat** (Preview) | What the preview shows the material on. |

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
- **Tip:** the preview's **Flat** view shows the texture twice each way: if you can see a line where the copies meet, the material doesn't tile there. Every box in the app is made to tile, so this only happens if something is off.
- **Tip:** the Keys window (**Keys** on the menu bar) shows the canvas's keys in the Material mode's green.

## Related

- [Material boxes](material-boxes.md): every box, in plain words
- [Concepts: material graphs](concepts.md#material-graphs)
- [Painting textures](painting-textures.md), for painting by hand
- [Materials](materials.md), for giving parts of a model their own material
- [Interface tour: the Material layout](interface-tour.md#layouts-and-panels)
- [Keyboard shortcuts](keyboard-shortcuts.md)
