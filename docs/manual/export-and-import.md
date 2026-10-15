# Export and import

> Write your models to a file for a game engine or Substance Painter, and bring models in from other programs.

![The Export window: the kind of file, what to export, metres or centimetres, Y up or Z up, and cutting polygons into triangles](../screenshots/2026-10-15-export-window.png)

## When to use it

Your project file (`.avtl`) keeps everything the app needs to go on working: settings, recorded edits, undo-ready shapes. Other programs can't read it. To use a model elsewhere, **export** it to a common file:

- **OBJ** (`.obj`): plain text, read by almost every 3D program, including Godot, Unity, Unreal Engine, Blender and Substance Painter. It holds the shapes (points, polygons, normals and UVs), not where each object's pivot is.

More kinds of file (glTF for Godot, with each object's pivot) and ready-made settings for Godot and Substance Painter arrive in the next updates of Phase 5.

**Import** does the reverse: it brings shapes in from an OBJ file made elsewhere, as plain meshes you can edit.

## How to use it

### Export

1. Choose **File → Export…**. The Export window opens.
2. Pick the **kind of file** (OBJ for now).
3. Pick **What**: **Everything shown** (hidden objects stay out), or **The selected objects** (and what's inside them).
4. Check **Size and direction** for where the file is going:
   - **Metres** for Godot, Unity and Substance Painter; **Centimetres** for Unreal Engine.
   - **Y up** for nearly every program. The app is Z up, so it turns the model on the way out and it arrives standing upright. **Z up** keeps the app's own axes.
5. Tick **Cut polygons into triangles** if the model will be baked in Substance Painter: the file then holds exactly the triangles the game will draw, cut the same way the viewport cuts them, so the bake matches. Game engines cut polygons themselves otherwise, not always the same way.
6. Click **Export…**, choose where to save, and the status bar says how many objects and triangles went out.

**File → Export again** repeats the last export, to the same file, the same way: handy while you go back and forth between the app and the engine.

**Shading goes out as you see it.** With **Automatic hard edges** on (Mesh menu, the standard), edges sharper than 30° go out crisp and the rest smooth; with it off, the edges you marked [hard](hard-and-soft-edges.md). Each corner gets the [normal](concepts.md#shading-and-normals) the viewport shows, so a model shades the same in the engine.

Each object's points stay joined in the file: a smooth surface is one connected piece, as Substance Painter wants for baking. Every edit and setting is applied; the file holds the shape as it is now.

### Import

1. Choose **File → Import…** and pick an OBJ file.
2. Each object in the file (named by its `o` lines, or `g` lines if it has none) becomes a plain mesh at the top of the Objects panel, selected, and the view frames it. One **Ctrl + Z** takes the whole import back.

The app reads an OBJ as metres with Y up, the usual way, and turns it to centimetres with Z up; a file this app exported says how it was written, and is read back that way. Where the file gives two polygons different normals along an edge they share, that edge is made **hard**, so a model shaded crisp elsewhere stays crisp. UVs come along too, ready for the UV tools in Phase 8. Lines the app doesn't use (curves, material files) are skipped, and broken polygons are left out and counted in the status bar.

## Settings

| Setting | What it does |
|---|---|
| **Kind of file** | OBJ (`.obj`) for now. |
| **What** | Everything shown (standard), or the selected objects and what's inside them. Groups have no shape of their own; only shapes are exported. |
| **Metres / Centimetres** | The file's units. 1 unit = 1 m for Godot, Unity and Substance Painter (standard); 1 cm for Unreal Engine. |
| **Y up / Z up** | Y up (standard) turns the model so its height runs along Y, as nearly every program expects. Z up keeps the app's own axes. |
| **Cut polygons into triangles** | Off (standard): polygons go out as they are. On: every polygon is cut into triangles the same way the viewport cuts them. |

The Export window remembers its choices while the app is open.

## Shortcuts

None yet: use the File menu.

## Tips and common problems

- **Problem:** the model arrives lying on its back. **Why:** the file was written Z up, and the program expected Y up (or the other way round). **Fix:** export again with the other choice.
- **Problem:** the model is 100 times too big or too small. **Why:** metres and centimetres were mixed up. **Fix:** Godot, Unity and Substance Painter want **Metres**; Unreal Engine wants **Centimetres**.
- **Problem:** the status bar says there is nothing to export. **Why:** everything is hidden, or *The selected objects* was chosen with only groups selected. **Fix:** show the objects, or choose Everything shown.
- **Problem:** an imported model looks faceted all over. **Why:** the file had no normals, so the app couldn't tell which edges were meant to be hard. **Fix:** keep **Automatic hard edges** on (Mesh menu), or mark edges with [Hard and soft edges](hard-and-soft-edges.md).
- **Tip:** an imported object is a plain mesh: there are no settings to change, but every modelling tool works on it.

## Related

- [Hard and soft edges, and normals](hard-and-soft-edges.md)
- [Pivots and the Object mode](pivots.md) (the units of the project)
- [Concepts: triangulation, glTF and the game-asset pipeline](concepts.md#the-game-asset-pipeline)
- [Saving and opening projects](saving-and-opening.md)
