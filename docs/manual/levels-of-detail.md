# Levels of detail

> Give an object a few simpler copies of itself for a game to draw when it is far away, each made the way that suits it, and check each one in the viewport. All in the **Levels** panel.

A detailed rock up close might have 5 000 triangles. Fifty metres away it covers a few dozen pixels on screen, and the game still draws all 5 000, though nobody can see the difference. **[Levels of detail](concepts.md#the-game-asset-pipeline)** (LODs for short) fix that: simpler copies of the model that the game swaps in as it moves away, so far-off things cost little. A forest or a city full of props runs much faster with them.

Each level can be made in three ways, and you can mix them:

1. **With its own modifier changes** (usually the best): the level is the model worked out again with some of its [modifiers](modifiers.md) switched off or given simpler settings. A bevelled crate's far level with **Live bevel** at 1 segment instead of 3, or with **Subdivide** off; a scattered rock pile without its **Scatter** copies; a pipe swept with 6 sides instead of 16; an **Array** with fewer copies. It stays live: change the model and every level follows.
2. **Reduced automatically**: triangles taken away with the same careful reduction as [Decimate](decimate.md) (UV seams, hard edges, material borders and open borders kept), as a last step after the level's modifier changes. Levels made from the same shape are reduced together, each using only points of the one before, so the outline doesn't jump when the game swaps them.
3. **By hand**: the level becomes a shape of its own, a copy of what it showed, that you model yourself in the viewport. It no longer follows the model.

The object itself never changes.

## When to use it

- **Props that appear many times, or far away:** rocks, trees, crates, barrels, buildings, kit pieces.
- **Shapes made with modifiers**: switching a modifier off or lowering its count at a level gives a clean, light level with the shape's character, often better than reducing triangles blindly.
- **To check what Godot will do.** Godot makes its own levels when it imports a model (see below), so you don't need these for Godot. Previewing a level here still shows how the shape holds up when simplified.
- **For other engines and tools** that take hand-made levels as meshes named `_LOD1`, `_LOD2` and so on, or to hit a triangle budget per level yourself.

Not for: making the object itself lighter for good. Use [Decimate](decimate.md) for that.

## How to use it

1. Select a shape. Open the **Levels** panel: it's a tab beside [Attributes](attributes.md) (top right). If you don't see it, choose **Window → Levels**, or click **Open the Levels panel** in Attributes' *Levels of detail* line. Switch on **Make levels**. Or press **Space**, type *levels of detail*, and pick **Make levels of detail**: it switches them on for every selected shape and brings the panel to the front.
2. Three levels appear, as standard, each reduced: **Level 1** to 50% of its triangles, **Level 2** to 25%, **Level 3** to 12%, shown from **10 m**, **20 m** and **40 m**. Each level's first line says how many triangles it has (as the game gets them), and the **Full shape** line says how many the model has.
3. To look at a level, click its **Preview** switch. The viewport draws that level in the shape's place, and its line in the panel is lit in the accent colour. Only the view changes: you still select and edit the full shape. Click the Full shape's **Preview** when you're done.
4. Set each level the way you want:
   - **Modifiers at this level** lists the object's modifiers. Untick one to switch it off for this level only. Open **Settings here** under it to give it simpler settings for this level (fewer segments, a lower count); the level's line then says *changed here*, and **Same as the model** takes the change back.
   - **Reduce** (on as standard) takes triangles away as a last step, keeping the share you set of the level's own triangles. Switch it off when the modifier changes are enough.
   - **Budget** is the number of triangles you aim for at this level. The level's count turns amber when it has more. Nothing changes by itself: lower its settings or its share until it fits.
   - **Shown from** is the distance a game starts showing it.
5. To model a level yourself, click **Make this level by hand** under it. A copy of what it showed becomes a shape of its own, called after the object (*Rock LOD2*), inside the object in the [Objects panel](objects.md), and you're editing it straight away: the viewport shows it in the object's place, with a banner across the top saying so. Use any [mode](modes.md) and tool on it, as on any shape. Click **Stop editing the level** on the banner (or select something else) when you're done; **Edit this level** in the panel takes you back in.
6. To put the levels in the exported file, keep **Levels of detail** ticked next to *Includes* in the [Export window](export-and-import.md): it starts ticked when anything going out has levels made, until you set it yourself. [Export kit](kit-export.md) and exports into a [game project folder](export-and-import.md#export-into-a-game-project) use that choice too.

### Levels made by hand

A hand-made level doesn't follow the model: that's the point of making it by hand. When the model has changed since you made it (its shape, its modifiers' settings or its smoothing), the panel says *The model has changed since this level was made.*, with **Start again from the model**: that makes the level again from the model as it is now, throwing your edits on it away (undo brings them back).

Its shape is an ordinary shape inside the object, so you can also give it its own modifiers, move it a little, or rename it. It's drawn only while you edit it, and its name is quieter in the Objects panel meanwhile. It stops being the level if you delete it (or click **Make from the model** in the panel), or drag it out of its object in the Objects panel: the level is then made from the model again, and a shape dragged out becomes an ordinary object. Copying the object copies its hand-made levels with it.

### What goes into the file

With *Levels of detail* ticked, each object with levels goes out under its own name as before (the full shape, level 0), and each level beside it as a mesh of its own, its real shape (with its modifier changes, reduced or not, or as you modelled it), at the same pivot, named after it: `Rock_LOD1`, `Rock_LOD2`, `Rock_LOD3`. In a glTF file, each also carries its distances, in metres, as notes (glTF *extras*): `lod_begin` and `lod_end` (from how far, and up to how far; an end of 0 means no end), and the full shape carries `lod_begin` 0 and `lod_end` up to Level 1's distance. In an OBJ file the levels are just extra objects. A hand-made level's own shape never goes out as an object of its own: only as its level.

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

All in the **Levels** panel.

| Setting | What it does |
|---|---|
| **Make levels** | On: the object has levels of detail. Off (standard): none, and its settings are kept for next time. |
| **Levels** | How many simpler copies, 1 to 8. **3** as standard. Adding one gives it half the share of the last and twice its distance. |
| **Preview** | On each line (Full shape, Level 1, …): which one the viewport draws. The line shown is lit. Saved with the project; an undo step like the other settings. |
| *triangles* | On each level's line: how many triangles it has as exported (… while it's being made), and *of* its budget when it has one, in amber when over. |
| **Shown from** | How far from the camera a game starts showing that level instead of the one before, up to 10 km. **10 m, 20 m, 40 m** as standard. It goes into the file; the app's own viewport doesn't switch by distance. |
| **Budget** | The triangles you aim for at that level; **0** (standard) for none. Only shown beside the count. |
| **Reduce** | On (standard): triangles are taken away as a last step, keeping this share of the level's own triangles (after its modifier changes): **50%, 25%, 12%** as standard. Levels side by side made from the same shape are reduced together, each at most as detailed as the one before. Off: the level is exactly what its modifier changes give. |
| **Modifiers at this level** | Each of the object's modifiers, ticked when it's on at this level. Untick to switch it off here only. **Settings here** gives it its own settings at this level: the level then keeps its own copy of that modifier's settings (it follows every edit of the shape, but no longer the model's later changes to that modifier's settings). **Same as the model** takes the level's change away. Removing a modifier from the object removes its changes too. |
| **Make this level by hand** | Turns the level into a shape of its own to model, a copy of what it showed, and starts editing it. |
| **Edit this level** | For a level made by hand: shows its shape in the object's place and lets you edit it (it selects it). Click again to stop. |
| **Make from the model** | For a level made by hand: deletes its shape, so the level is made from the model again. |
| **Start again from the model** | Shown when the model has changed since a hand-made level was made: makes it again from the model now. |
| **Levels of detail** (Export window, *Includes*) | On when anything going out has levels made, off otherwise, until you tick or untick it yourself; your choice is then remembered with the other export settings. Off: files hold the objects only. On: each object's levels go beside it, `name_LOD1` and on. |

The levels are made from the shape **as exported**: every [modifier](modifiers.md) that is on (with the level's changes), the [live mirror](symmetry.md) and [booleans](booleans.md) included, and at Subdivide's *Levels for export*. So the preview and the counts show what the game gets.

## Shortcuts

| Action | Shortcut |
|---|---|
| Switch levels of detail on for the selected shapes | `Space`, then type *levels of detail* |
| Stop editing a level made by hand | **Stop editing the level** on the viewport's banner, or select something else |

There are no keys for any of this, so no key is taken from future tools. Ask if you'd like one.

## Tips and common problems

- **Problem:** a level has more triangles than its share. **Why:** the reduction keeps UV seams, hard edges, material borders and open borders as lines, and their corners never move, so a shape with many seams or hard edges stops early. **Fix:** fewer seams or a higher Smoothing angle help; for a stronger reduction of the object itself, [Decimate](decimate.md) lets you switch each kind of line off.
- **Problem:** an amber note says *No levels of detail: a point of the shape isn't at a real position*. **Why:** the shape is damaged (a point at no real place). **Fix:** [Clean up](clean-up.md) the shape, or undo what broke it.
- **Problem:** in Godot, the levels are hidden and never appear. **Why:** Godot doesn't use them by itself. **Fix:** add the script above, or delete the `_LOD` meshes and let Godot's own levels do the work.
- **Problem:** the triangle counts show *…* for a while, or the preview shows the last level for a moment. **Why:** a heavy shape's levels are made in the background, so the window never freezes; the count and the level appear when they're ready.
- **Problem:** I edited the model and a level didn't change. **Why:** it's made by hand (it has **Edit this level**), or it has its own **Settings here** for the modifier you changed. **Fix:** **Start again from the model**, or **Same as the model** on that modifier.
- **Problem:** the object vanished from the viewport. **Why:** you're editing one of its hand-made levels: the level is shown in its place (the banner says so). **Fix:** **Stop editing the level**.
- **Tip:** try modifier changes before reduction: a Live bevel at 1 segment or a sweep with fewer sides keeps clean shapes and good shading, where reduction has to guess.
- **Tip:** about each level should halve the triangles and double the distance: on screen, an object twice as far covers a quarter of the area, so half the triangles still look as fine.
- **Tip:** the collision shapes and the exported full shape aren't affected by the levels: physics uses the [collision shape](collision-shapes.md) you chose.

## Related

- [Decimate](decimate.md): the same reduction, as a modifier that changes the object itself.
- [Modifiers](modifiers.md): the modifiers a level can switch off or simplify.
- [Kit export and collision](kit-export.md): each piece in its own file, levels and all.
- [Export and import](export-and-import.md): the Export window's *Includes*.
- [Concepts](concepts.md#the-game-asset-pipeline): level of detail, triangle budget.
