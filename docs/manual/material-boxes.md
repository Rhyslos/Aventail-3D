# Material boxes

> Every box you can add to a [material graph](material-graphs.md), in plain words, in the order experts use them: **Make**, **Shape**, **Colour**, **Finish**.

Each box does one small job. A graph usually reads left to right in the same order as this page: a **Make** box draws a pattern from nothing, **Shape** boxes change it, a **Colour** box colours it, and a **Finish** box turns it into a whole material. Every box tiles seamlessly, so the material repeats without lines.

## When to use it

- Use this page to find the right box for a job ("I want the edges rounded": **Bevel**; "I want it to look less neat": **Warp**).
- In the app, the same words are in Attributes under **What does this do?** when a box is selected, and **Usually comes next** lists the boxes most often used after it.

## How to use it

1. In the **Material** mode, point at the canvas and press **Space** (or right-click on empty canvas) to open the box search. Type part of a box's name, or pick from the groups.
2. Connect it by dragging from an output dot (right edge) to an input dot (left edge). Wires are coloured by what they carry: grey for a [shape](concepts.md#material-graphs), amber for a [height](concepts.md#material-graphs), blue for a colour, green for a [whole material](concepts.md#material-graphs).
3. Select the box to change its settings in Attributes.

A simple stone-tile material, to see how they fit together:

**Tiles** → **Warp** (the tiles wobble) → **Bevel** (rounded edges) → **Blend** with **Noise** (a bumpy surface) → **Gradient map** (colours) and **Material from height** → **Mix materials** with moss in the gaps → **Material out**.

### Sizes in centimetres

Settings that are a size (a bevel's width, a blur's reach, how far Warp pushes) are given in **centimetres on the finished material**. The graph's [real-world size](concepts.md#real-world-size), set on **Material out**, says how many centimetres the whole texture covers (100 cm to start with). So a bevel 2 cm wide stays 2 cm wide whatever size the texture is worked out at, and if you make the stones bigger by raising the real-world size, the bevel stays the same real width. Numbers that must be whole for the texture to tile (tiles across, cells across, repeats) stay counts.

## The boxes

### Make: patterns from nothing

| Box | What it makes | Settings |
|---|---|---|
| **Noise** | Soft, cloudy greys: the start of stone, ground, rust and most natural surfaces. | **Scale** (blobs across), **Detail** (finer layers on top), **Roughness** (how strong the finer layers are), **Seed**. |
| **Clouds** | Big, soft, billowing greys, for weathering, dirt and patches of colour. **Billowy** puffs it up like cotton wool; **Ridged** gives sharp ridges like crumpled paper or veins. | **Scale**, **Detail**, **Roughness**, **Look** (Soft, Billowy, Ridged), **Seed**. |
| **Cells** | Irregular cells: cobbles, scales, cracked mud, crystals. **Random** gives each cell its own grey. | **Scale** (cells across), **Jitter** (0 is a neat grid), **Look** (Distance: dark middles; Bumps: domes; Edges: the lines between; Flat: one grey per cell), **Seed**. |
| **Gradient** | Greys running smoothly dark to light: stripes, waves, fading something in, or a soft spot in each repeat. | **Style** (Up and down: no sharp step; Ramp: dark to light then a sharp step; Round: a spot), **Direction** (Across, Down, Diagonal), **Repeats**. |
| **Bricks** | Bricks in rows, every other row moved along, mortar the same width all round. **Random** gives each brick its own grey. | **Rows** (an odd number is made one more, so it tiles), **Bricks per row**, **Mortar**, **Shift**, **Seed**. |
| **Tiles** | A grid of tiles with gaps: floors, walls. **Random** gives each tile its own grey. | **Across**, **Down**, **Gap**, **Shift** (keep Down even with a shift), **Seed**. |
| **Planks** | Rows of boards, each row's ends at a random place. **Random** gives each board a grey; **Grain** gives streaks along each board. | **Boards down**, **Boards per row**, **Gap**, **Grain size**, **Seed**. |
| **Basic shape** | A circle, square, polygon or star, once or in a grid: studs, rivets, holes, dots. | **Shape**, **Size** (how much of each repeat it fills), **Sides** (polygon sides or star points), **Star depth**, **Turn** (degrees), **Repeats**, **Soft edge**. |

### Shape: changing a shape or height

| Box | What it does | Settings |
|---|---|---|
| **Blur** | Softens a height, smoothing sharp edges and small bumps. | **Radius** (cm). |
| **Warp** | Pushes a height about so straight lines wobble: neat tiles become hand-cut stone. With a height in **By**, things move the way it slopes upwards; with nothing there, its own noise moves them. | **Amount** (cm), **Scale** (its own wobbles across), **Seed**. |
| **Levels** | Changes how dark and light a height is: crisper or softer edges, or turned upside down. | **Darkest**, **Lightest**, **Middle**, **Turn upside down**. |
| **Edge finder** | Draws a shape's outline as a soft white line: worn edges, grout, seams. | **Width** (cm), **Side** (Inside, Outside, Both). |
| **Bevel** | Turns a black-and-white shape into a height with rounded edges, as if each tile were cut with a rounded, sloped or stepped edge. | **Width** (cm: how far in the edge reaches), **Roundness** (softens the edge's corners), **Height** (how high the top is), **Edge** (Round, Sharp, Steps), **Steps**. |
| **Transform** | Moves, turns or flips a height; what goes off one edge comes back on the other. | **Move across**, **Move down** (shares of the texture), **Turn** (quarter turns only: other angles would break the tiling), **Flip left to right**, **Flip top to bottom**. |
| **Repeat** | Copies a height in a grid, smaller: one stud becomes a sheet of studs. | **Across**, **Down**, **Shift** (every other row; keep Down even), **Vary** (each copy a little darker at random), **Seed**. |
| **Blend** | Mixes two heights, Top over Bottom, through a shape in **Where** if given. | **Mode** (Mix, Add, Subtract, Multiply, Lighter, Darker), **Amount**. |

### Colour

| Box | What it does | Settings |
|---|---|---|
| **Colour fill** | One colour everywhere. | **Colour**. |
| **Gradient map** | Colours a height: each grey takes the colour at the same place along a gradient, black at its left end, white at its right. The quickest way to make colour follow shape: dark in the cracks, light on top. | **Colours**: the gradient, drawn as a strip. Each colour has a swatch to pick it, a slider for where it sits (0 for black, 1 for white) and **×** to take it out; **Add a colour** puts one in the widest gap (2 to 8 colours). |
| **Hue and saturation** | Changes a colour picture's hues, how colourful it is, and how light. With a shape in **Where**, only the white parts change. | **Hue** (degrees round the colour wheel), **Saturation** (0 grey, 1 as it is, 2 twice as colourful), **Lightness**. |
| **Colour to shape** | Turns a colour picture into a black-and-white shape, so boxes that need a shape (Bevel, Edge finder, a Where plug) can use it. A colour on its own has no inside or outside: this box decides one. **Height** gives the same greys as a height. | **By** (Lightness, or Close to a colour), **Colour**, **Threshold**, **Softness**. |

### Finish: whole materials

| Box | What it does | Settings |
|---|---|---|
| **Material from height** | Turns a height into a whole material: works out the normal (how light catches the bumps), occlusion (darker in dips) and curvature (lighter on ridges), and adds colour, roughness and metal. | **Colour** (when nothing is plugged into Colour), **Depth**, **Roughness**, **Metal**, **Occlusion**. |
| **Mix materials** | Lays one whole material over another, mixing every map at once. **By shape**: Top shows where Where is white (everywhere with nothing there). **By height**: Top shows where it stands higher than Bottom, as moss fills cracks or snow sits on stones. | **By** (Shape, Height), **Amount**, **Rise** (lifts Top, so more or less of it shows), **Softness**. |
| **Material out** | The graph's result: the material plugged in here is what the graph makes. One per graph. | **Real-world size** (cm one repeat covers). |

## Settings

Each box's settings are in the tables above. Two settings appear on many boxes:

| Setting | What it does |
|---|---|
| **Seed** | Another number gives another pattern of the same look. It is mixed with the graph's own seed, so changing the graph's seed changes every pattern at once. |
| A size in **cm** | A length on the finished material, through Material out's **Real-world size**. |

## Shortcuts

The canvas's keys are on the [Material graphs](material-graphs.md#shortcuts) page. Boxes add no keys of their own.

## Tips and common problems

- **Problem:** Bevel makes everything one flat white. **Why:** the gaps in its shape are thinner than a pixel at the size being worked out, so there is no outside to round towards. **Fix:** widen the gaps (Tiles' **Gap**, Bricks' **Mortar**), or look at a bigger picture; the canvas works pictures out at 256 pixels or more.
- **Problem:** a wire from a colour won't go into Bevel. **Why:** Bevel needs to know what is inside and outside, and a colour has neither. **Fix:** the note offers **Put a Colour to shape box between them**: click it.
- **Problem:** after Tiles or Repeat with a **Shift**, there is a line at the top of the texture. **Why:** with an odd number of rows, the first and last rows are both unshifted. **Fix:** keep **Down** even (Bricks makes its rows even for you).
- **Tip:** to make each tile a little different, plug Tiles' (or Bricks', Planks', Cells') **Random** into a Blend with a small Amount, or into a Gradient map for a different colour per tile.
- **Tip:** Warp before Bevel makes hand-cut stones; Warp after Bevel makes worn, soft ones.
- **Tip:** try Gradient map with dark colours on the left and warm, light ones on the right for a hand-painted look.

## Related

- [Material graphs](material-graphs.md): the canvas, connecting boxes, the preview
- [Concepts: material graphs](concepts.md#material-graphs), [real-world size](concepts.md#real-world-size), [gradient map](concepts.md#gradient-map), [seamless tiling](concepts.md#seamless-tiling)
- [Painting textures](painting-textures.md)
