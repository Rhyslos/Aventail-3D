# Levels of detail

> Give an object a few simpler copies of itself, each with fewer triangles, for a game to draw when it is far away, and check each one in the viewport.

A detailed rock up close might have 5 000 triangles. Fifty metres away it covers a few dozen pixels on screen, and the game still draws all 5 000, though nobody can see the difference. **[Levels of detail](concepts.md#the-game-asset-pipeline)** (LODs for short) fix that: simpler copies of the model that the game swaps in as it moves away, so far-off things cost little. A forest or a city full of props runs much faster with them.

The app makes the copies for you, from the object's finished shape, with the same careful reduction as [Decimate](decimate.md): UV seams, hard edges, material borders and open borders are kept, and each simpler level uses only points of the one before, so the outline doesn't jump when the game swaps them. The object itself never changes.

## When to use it

- **Props that appear many times, or far away:** rocks, trees, crates, barrels, buildings, kit pieces.
- **To check what Godot will do.** Godot makes its own levels when it imports a model (see below), so you don't need these for Godot. Previewing a level here still shows how the shape holds up when simplified.
- **For other engines and tools** that take hand-made levels as meshes named `_LOD1`, `_LOD2` and so on, or to tune each level's triangles yourself.

Not for: making the object itself lighter for good. Use [Decimate](decimate.md) for that.

## How to use it

1. Select a shape. In the [Attributes panel](attributes.md), find **Levels of detail** (below the modifiers and materials) and switch on **Make levels**. Or press **Space**, type *levels of detail*, and pick **Make levels of detail**: it switches them on for every selected shape.
2. Three levels appear, as standard: **Level 1** keeps 50% of the triangles, **Level 2** 25%, **Level 3** 12%, shown from **10 m**, **20 m** and **40 m**. Under each, **Triangles** says how many it has, and a line below says how many the full shape has.
3. To look at a level, choose it in **Preview**. The viewport draws that level in the shape's place. Only the view changes: you still select and edit the full shape, and it still exports whole. Set Preview back to **Full shape** when you're done.
4. Change **Levels** for more or fewer (1 to 8), and each level's share and distance to taste.
5. To put the levels in the exported file, tick **Levels of detail** next to *Includes* in the [Export window](export-and-import.md). [Export kit](kit-export.md) and exports into a [game project folder](export-and-import.md#export-into-a-game-project) use that choice too.

### What goes into the file

With *Levels of detail* ticked, each object with levels goes out under its own name as before (the full shape, level 0), and each level beside it as a mesh of its own, at the same pivot, named after it: `Rock_LOD1`, `Rock_LOD2`, `Rock_LOD3`. In a glTF file, each also carries its distances, in metres, as notes (glTF *extras*): `lod_begin` and `lod_end` (from how far, and up to how far; an end of 0 means no end), and the full shape carries `lod_begin` 0 and `lod_end` up to Level 1's distance. In an OBJ file the levels are just extra objects.

In an export **for Godot** the levels are written **hidden**, so the imported scene shows the rock once, not four times on top of each other.

### Godot: its own levels, or yours

Godot doesn't read hand-made levels of detail from a glTF file: there's no name ending or file setting it looks for. Instead, it **makes its own when it imports** a model, from the full shape, and picks one each frame by how big the model is on screen. This is on as standard: select the model file in Godot's FileSystem dock, open the **Import** tab, and see **Meshes → Generate LODs**. So for most Godot games, export without *Levels of detail* and let Godot do it.

Use the app's levels in Godot when you want to choose each level's triangles and distance yourself. Godot can show one mesh between two distances (its **Visibility Range**, under GeometryInstance3D in the Inspector). This short script sets those from the distances in the file, and shows the levels, every time the model is imported:

1. In Godot, make a new script file called `lod_ranges.gd` and paste this into it:

   ```gdscript
   @tool
   extends EditorScenePostImport

   # Turns the levels of detail exported from the app into visibility
   # ranges: each node with "lod_begin" and "lod_end" in its glTF
   # extras is shown between those distances, and shown at all.


   func _post_import(scene: Node) -> Object:
   	_set_ranges(scene)
   	return scene


   func _set_ranges(node: Node) -> void:
   	if node is GeometryInstance3D and node.has_meta("extras"):
   		var extras: Dictionary = node.get_meta("extras")
   		if extras.has("lod_begin") and extras.has("lod_end"):
   			node.visibility_range_begin = extras["lod_begin"]
   			node.visibility_range_end = extras["lod_end"]
   			node.visible = true
   	for child in node.get_children():
   		_set_ranges(child)
   ```

2. Select the model file, open the **Import** tab, set **Import Script → Path** to `lod_ranges.gd`, and click **Reimport**. (To use it for every model, choose **Preset → Set as Default for 'Scene'** in the same tab.)
3. Optionally, switch off **Meshes → Generate LODs** for that file: otherwise Godot also makes its own levels inside each of yours (harmless, just extra work at import).

## Settings

| Setting | What it does |
|---|---|
| **Make levels** | On: the object has levels of detail. Off (standard): none, and its settings are kept for next time. |
| **Levels** | How many simpler copies, 1 to 8. **3** as standard. Adding one gives it half the triangles of the last and twice its distance. |
| **Level 1, 2, …** | That level's share of the full shape's triangles. **50%, 25%, 12%** as standard. A level with a larger share than the one before is made the same as that one: each level is at most as detailed as the one before. |
| **Shown from** | How far from the camera a game starts showing that level instead of the one before, up to 10 km. **10 m, 20 m, 40 m** as standard. It goes into the file; the app's own viewport doesn't switch by distance. |
| **Triangles** | How many triangles the level has, once it's made (… until then). Kept seams, hard edges and borders can stop a level short of its share. |
| **Preview** | Which level the viewport draws: **Full shape** (standard) or a level. Saved with the project. |
| **Levels of detail** (Export window, *Includes*) | Off (standard): files hold the objects only. On: each object's levels go beside it, `name_LOD1` and on. Remembered with the other export settings. |

The levels are made from the shape **as exported**: every [modifier](modifiers.md) that is on, the [live mirror](symmetry.md) and [booleans](booleans.md) included. The Triangles counts and the preview use the shape the viewport draws, which is the same unless a modifier has a different viewport setting (Subdivide's levels, say).

## Shortcuts

| Action | Shortcut |
|---|---|
| Switch levels of detail on for the selected shapes | `Space`, then type *levels of detail* |

There's no key for it, so no key is taken from future tools. Ask if you'd like one.

## Tips and common problems

- **Problem:** a level has more triangles than its share. **Why:** the reduction keeps UV seams, hard edges, material borders and open borders as lines, and their corners never move, so a shape with many seams or hard edges stops early. **Fix:** fewer seams or a higher Smoothing angle help; for a stronger reduction of the object itself, [Decimate](decimate.md) lets you switch each kind of line off.
- **Problem:** an amber note says *No levels of detail: a point of the shape isn't at a real position*. **Why:** the shape is damaged (a point at no real place). **Fix:** [Clean up](clean-up.md) the shape, or undo what broke it.
- **Problem:** in Godot, the levels are hidden and never appear. **Why:** Godot doesn't use them by itself. **Fix:** add the script above, or delete the `_LOD` meshes and let Godot's own levels do the work.
- **Problem:** the triangle counts show *…* for a while. **Why:** a heavy shape's levels are made in the background; the count appears when they're ready.
- **Tip:** about each level should halve the triangles and double the distance: on screen, an object twice as far covers a quarter of the area, so half the triangles still look as fine.
- **Tip:** the collision shapes and the exported full shape aren't affected by the levels: physics uses the [collision shape](collision-shapes.md) you chose.

## Related

- [Decimate](decimate.md): the same reduction, as a modifier that changes the object itself.
- [Kit export and collision](kit-export.md): each piece in its own file, levels and all.
- [Export and import](export-and-import.md): the Export window's *Includes*.
- [Concepts](concepts.md#the-game-asset-pipeline): level of detail, triangle budget.
