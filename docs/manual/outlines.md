# Outlines

> A dark line round an object for a cartoon look: drawn in the viewport, and exported so any game engine shows it.

## When to use it

- Your game has a cartoon or comic-book style, where objects have ink lines round them.
- You want a prop to stand out against busy backgrounds.

The outline is drawn round the object's silhouette (its outer edge as seen from the camera). Lines *inside* the shape, such as the edge between two faces of a box, come from painting instead: see [Edge highlights](painting-textures.md#edge-highlights) and [Smart masks](painting-textures.md#smart-masks).

## How to use it

1. Select the object.
2. In the **Attributes** panel, tick **Outline**. A dark line appears round it in the viewport.
3. Set its **Width** (how thick, in your units) and **Colour**.
4. Export as usual (**File → Export**, or a game project folder). The outline goes with the object, as explained below.

Each object has its own outline: tick it on the ones that need one.

### How it gets into the game

An outline needs a little trick, because a game draws an object's surface, not its edges. The app uses the most common one, the **inverted hull** (see [Concepts](concepts.md#uvs-and-texturing)):

- **A shell beside each outlined object.** The export adds an object called *Name-outline*: a copy of the shape, a little bigger (pushed out by the width), turned inside out, in the outline's colour. Games leave out the side of a surface facing away from them, so only the rim of the shell, round the object's edge, shows. This works in every engine with no extra setup: Godot, Unity, Unreal, Solis.
- **A Godot shader too.** Exporting for Godot also writes *name-outline.gdshader* beside the file. It draws the same rim without the extra object, and lets you change the width in Godot. To use it instead of the shell: in Godot, select the object's material, set **Next Pass** to a new **ShaderMaterial**, load the shader into it, and delete the *-outline* node.

The shell has as many triangles as the object, so an outlined object costs about twice as much to draw: keep outlines for the objects that need them.

## Settings

| Setting | What it does |
|---|---|
| **Outline** (Attributes) | Gives the selected object an outline. Off as standard. |
| **Width** | How thick the line is, measured on the object (1 cm to start with). A line on a small prop wants to be thinner than on a building: around 0.5–2 cm for props a player holds, more for big objects seen from afar. |
| **Colour** | The line's colour. Near black (the standard, a little warm) reads as ink; a darker shade of the object's own colour gives a softer look. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Undo an outline change | `Ctrl + Z` |

## Tips and common problems

- **Problem:** the outline has gaps or breaks at sharp corners. **Why:** a very thin wall or a shape with holes pushes the shell through itself. **Fix:** lower the width, or close the shape.
- **Problem:** the outline shows *through* a flat shape, such as a plane. **Why:** a plane has no inside, so its shell faces both ways. **Fix:** outlines suit closed shapes; give the plane some thickness with [Shell](shell.md).
- **Problem:** in Godot the outline looks too thick or too thin. **Why:** the width is in the model's units, so a scaled object's outline scales too. **Fix:** change the width here, or use the Godot shader and change its *width* there.
- **Problem:** the object has an extra object called *-outline* in the game. **Why:** that's the shell that draws the outline. **Fix:** leave it as a child of the object, so they move together.

## Related

- [Painting textures](painting-textures.md): edge highlights and the Toon preview go well with outlines.
- [Export and import](export-and-import.md)
- [Concepts](concepts.md)
