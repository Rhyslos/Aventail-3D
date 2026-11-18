# Script reference

> Every call a script can make, what it takes and gives back, in LustScript and Lua.

<!-- Made from the scripting crate's table (crates/script/src/describe.rs): a test fails when it's out of date. Write it again with UPDATE_REFERENCE=1 cargo test -p app --test script_reference. -->

Scripts are explained in [Scripting](scripts.md); words like *array* and *state* are in [Concepts](concepts.md#scripting). This page lists **scripting version 1**: a script says the version it was written for with an `-- api: 1` line at its top.

In LustScript, fetch what you use from the app's own files first, such as `fetch "aventail/scene":[Scene, Shape];`. In Lua everything is under `aventail.`, such as `aventail.Scene:selected()`; a shape's own calls are written the same in both (`shape:name()`).

Calls that change the scene change the script's own copy of it, so the script reads its own changes as it goes. When it ends well, everything it did lands on the project at once as **one undo step** named after the script; when it stops on a mistake, is stopped, or runs out of time or memory, nothing it did is kept. A run may make at most 100,000 changes.

## Scene

The scene the script runs on: every shape in the Objects panel.

LustScript: `fetch "aventail/scene":Scene;`

| LustScript | Lua | Gives | What it does | Since |
|---|---|---|---|---|
| `Scene:shapes()` | `aventail.Scene:shapes()` | Shape array | Every object in the scene, in the Objects panel's order (top to bottom, the ones inside a group after it). | 1 |
| `Scene:selected()` | `aventail.Scene:selected()` | Shape array | The selected objects, in the Objects panel's order. | 1 |
| `Scene:find(name)` | `aventail.Scene:find(name)` | Shape | The first object with this name, in the Objects panel's order, or nothing. | 1 |
| `Scene:add(kind)` | `aventail.Scene:add(kind)` | Shape | Makes a new shape of this kind with its standard settings, at the top of the Objects panel, and selects it, as the Create menu does. Its settings stay editable. | 1 |
| `Scene:build(name, points, polygons)` | `aventail.Scene:build(name, points, polygons)` | Shape | Makes a new shape called name from points (where each is, in centimetres) and polygons (only each Polygon's corners are read: its points, named by their place in points counting from 0, in order around the side it faces). Each point's id in the new shape is its place in points. Refused with the reason when a polygon has fewer than three corners, names a point that isn't there or the same point twice, a point isn't a real number, or there are more than 100,000 points or polygons. The new shape is put at the top of the Objects panel and selected. | 1 |
| `Scene:group(shapes)` | `aventail.Scene:group(shapes)` | Shape | Puts the shapes into a new group, which takes the place of the first of them, and selects the group. | 1 |

## Shape

One object in the Objects panel (a shape, a group, a curve…). Its `id` is a whole number that names it for the whole run.

LustScript: `fetch "aventail/scene":Shape;`

| LustScript | Lua | Gives | What it does | Since |
|---|---|---|---|---|
| `shape:name()` | `shape:name()` | string | Its name, as the Objects panel shows it. | 1 |
| `shape:position()` | `shape:position()` | vec3 | Where its pivot is, in centimetres, relative to the group it's in (as the Coordinates panel shows it). | 1 |
| `shape:rotation()` | `shape:rotation()` | vec3 | How it's turned, in degrees: heading (around Z) in x, pitch (around X) in y and bank (around Y) in z. | 1 |
| `shape:scale()` | `shape:scale()` | vec3 | Its size factor along each axis (1 is as made). | 1 |
| `shape:parent()` | `shape:parent()` | Shape | The group it's in, or nothing at the top of the scene. | 1 |
| `shape:children()` | `shape:children()` | Shape array | The objects directly inside it, in order. | 1 |
| `shape:shown()` | `shape:shown()` | bool | Whether its own switch shows it (its groups may still hide it). | 1 |
| `shape:pointCount()` | `shape:pointCount()` | int | How many points its final shape has (0 for a group). | 1 |
| `shape:points(from, count)` | `shape:points(from, count)` | Point array | Up to count of its points (at most 100,000 at a time), starting at the one numbered from (from 0), in a fixed order: read a heavy shape a chunk at a time. | 1 |
| `shape:polygonCount()` | `shape:polygonCount()` | int | How many polygons its final shape has (0 for a group). | 1 |
| `shape:polygons(from, count)` | `shape:polygons(from, count)` | Polygon array | Up to count of its polygons (at most 100,000 at a time), starting at the one numbered from (from 0), in a fixed order. | 1 |
| `shape:hardEdges()` | `shape:hardEdges()` | Edge array | The edges marked hard (where shading breaks). | 1 |
| `shape:seams()` | `shape:seams()` | Edge array | The edges marked as UV seams. | 1 |
| `shape:materials()` | `shape:materials()` | string array | The names of its materials, in slot order. | 1 |
| `shape:setName(name)` | `shape:setName(name)` | nothing | Renames it. Spaces around the name are trimmed; an empty name is refused. | 1 |
| `shape:setPosition(position)` | `shape:setPosition(position)` | nothing | Moves its pivot here, in centimetres, relative to the group it's in. | 1 |
| `shape:setRotation(rotation)` | `shape:setRotation(rotation)` | nothing | Turns it, in degrees: heading (around Z) in x, pitch (around X) in y and bank (around Y) in z. | 1 |
| `shape:setScale(scale)` | `shape:setScale(scale)` | nothing | Sets its size factor along each axis (1 is as made). | 1 |
| `shape:show()` | `shape:show()` | nothing | Shows it (its own switch: its groups may still hide it). | 1 |
| `shape:hide()` | `shape:hide()` | nothing | Hides it, and so everything inside it. | 1 |
| `shape:delete()` | `shape:delete()` | nothing | Deletes it and everything inside it. Calls on it afterwards are refused. | 1 |
| `shape:bevel(edges, width, segments)` | `shape:bevel(edges, width, segments)` | nothing | Rounds off these edges, width centimetres into each side as seen in the scene (as the Bevel tool measures), in segments rows (1 to 64; 1 is a flat cut). Recorded in its edit layers, like the Bevel tool's. | 1 |
| `shape:extrude(polygons, distance)` | `shape:extrude(polygons, distance)` | nothing | Pushes these polygons out by distance centimetres as seen in the scene (less than 0 pushes them in), neighbours together as one piece. Recorded in its edit layers. | 1 |
| `shape:inset(polygons, amount)` | `shape:inset(polygons, amount)` | nothing | Makes a smaller polygon inside each of these, amount centimetres in from their border as seen in the scene, neighbours together as one piece. Recorded in its edit layers. | 1 |
| `shape:markHard(edges)` | `shape:markHard(edges)` | nothing | Marks these edges hard, so shading breaks there. Recorded in its edit layers. | 1 |
| `shape:markSeams(edges)` | `shape:markSeams(edges)` | nothing | Marks these edges as UV seams, where its UVs are cut apart. Recorded in its edit layers. | 1 |
| `shape:unwrap()` | `shape:unwrap()` | nothing | Unwraps all of its polygons into UVs along its seams, with the UV settings the app has now, as Unwrap in the UV Editor does. Recorded in its edit layers. | 1 |
| `shape:setMaterial(polygons, name)` | `shape:setMaterial(polygons, name)` | nothing | Gives these polygons the material called name. A material it doesn't have yet is added to its materials, with the colour another shape's material of that name has, or the standard grey. Recorded in its edit layers. | 1 |
| `shape:makeEditable()` | `shape:makeEditable()` | nothing | Bakes its settings and recorded edits into a plain mesh, as Make editable does. A shape with nothing to bake is left as it is. | 1 |

## Selection

What's selected, and the mode the app was in when the script started.

LustScript: `fetch "aventail/selection":Selection;`

| LustScript | Lua | Gives | What it does | Since |
|---|---|---|---|---|
| `Selection:mode()` | `aventail.Selection:mode()` | Mode | The mode the app was in when the script started. | 1 |
| `Selection:points(shape)` | `aventail.Selection:points(shape)` | int array | The ids of the shape's selected points. | 1 |
| `Selection:edges(shape)` | `aventail.Selection:edges(shape)` | Edge array | The shape's selected edges. | 1 |
| `Selection:polygons(shape)` | `aventail.Selection:polygons(shape)` | int array | The ids of the shape's selected polygons. | 1 |
| `Selection:set(shapes)` | `aventail.Selection:set(shapes)` | nothing | Selects exactly these shapes. | 1 |
| `Selection:setPoints(shape, points)` | `aventail.Selection:setPoints(shape, points)` | nothing | Selects these points of the shape (by id), in place of the ones selected before. Its selected edges and polygons stay. | 1 |
| `Selection:setEdges(shape, edges)` | `aventail.Selection:setEdges(shape, edges)` | nothing | Selects these edges of the shape, in place of the ones selected before. Its selected points and polygons stay. | 1 |
| `Selection:setPolygons(shape, polygons)` | `aventail.Selection:setPolygons(shape, polygons)` | nothing | Selects these polygons of the shape (by id), in place of the ones selected before. Its selected points and edges stay. | 1 |
| `Selection:clear()` | `aventail.Selection:clear()` | nothing | Selects no shapes. | 1 |

## Project

The open project.

LustScript: `fetch "aventail/project":Project;`

| LustScript | Lua | Gives | What it does | Since |
|---|---|---|---|---|
| `Project:name()` | `aventail.Project:name()` | string | The project's name, as its tab shows it. | 1 |
| `Project:folder()` | `aventail.Project:folder()` | string | The folder the project is saved in, or empty text if it hasn't been saved yet. | 1 |
| `Project:readText(path)` | `aventail.Project:readText(path)` | string | The text in the file at path, inside the project's folder (or the folder you chose, for a script that asks for one), such as notes/names.txt. Needs a permission: the script's header asks to read the project folder (or more), and you allow it when it first runs. At most 16 MB; a file this run wrote reads as written. | 1 |
| `Project:writeText(path, text)` | `aventail.Project:writeText(path, text)` | nothing | Writes text to the file at path, inside the project's folder (or the folder you chose), making folders on the way. Needs a permission: the header asks to read and write the project folder, or a folder you choose. Files are written when the script ends well, each whole; a script that fails or is stopped writes nothing. | 1 |
| `Project:exportShapes(shapes, path, preset)` | `aventail.Project:exportShapes(shapes, path, preset)` | nothing | Exports the shapes (and what's inside groups among them, if shown) to the file at path, as the Export window does with the preset: a .glb for the game engines, an .obj for Substance Painter, an .stl for 3D printing. Needs the same permission as writeText, and writes when the script ends well. | 1 |

## Script

The script itself: what it says in the output, and how far it has got.

LustScript: `fetch "aventail/script":Script;`

| LustScript | Lua | Gives | What it does | Since |
|---|---|---|---|---|
| `Script:api()` | `aventail.Script:api()` | int | The scripting version this app has. | 1 |
| `Script:say(text)` | `aventail.Script:say(text)` | nothing | Writes a line in the output. print does the same. | 1 |
| `Script:warn(text)` | `aventail.Script:warn(text)` | nothing | Writes a warning in the output, marked so it stands out. | 1 |
| `Script:progress(done, what)` | `aventail.Script:progress(done, what)` | nothing | Says how far the script has got, from 0 (just started) to 1 (finished), and what it's doing: the status bar shows it. | 1 |

## What calls hand back

- **Point** (`fetch "aventail/scene":Point;`): A point of a shape: its id, and where it is in the shape's own space, in centimetres. Fields: `int id`, `vec3 position`.
- **Polygon** (`fetch "aventail/scene":Polygon;`): A polygon of a shape: its id, the ids of its corner points in order, its material's name, and which way it faces (one unit long, in the shape's own space). Fields: `int id`, `int array corners`, `string material`, `vec3 facing`.
- **Edge** (`fetch "aventail/scene":Edge;`): An edge of a shape, named by the ids of the two points at its ends. Fields: `int a`, `int b`.
- **Primitive** (`fetch "aventail/scene":Primitive;`): The ready-made shapes, as the Create menu lists them. One of `Primitive.Cube`, `Primitive.Plane`, `Primitive.Cylinder`, `Primitive.Sphere`, `Primitive.Disc`, `Primitive.Tube`, `Primitive.Cone`, `Primitive.Icosphere`, `Primitive.Torus`, `Primitive.Capsule`, `Primitive.Dome`, `Primitive.Elbow`, `Primitive.Sector`, `Primitive.Gear`, `Primitive.HexGrid`, `Primitive.HexSphere`, `Primitive.Stairs`, `Primitive.Arch`. In Lua it's the option's name as text (`"Cube"`), also kept as `aventail.Primitive.Cube`.
- **Mode** (`fetch "aventail/selection":Mode;`): The app's modes, as the mode palette lists them. One of `Mode.Model`, `Mode.Object`, `Mode.Points`, `Mode.Edges`, `Mode.Polygons`, `Mode.Uv`, `Mode.Paint`, `Mode.Material`. In Lua it's the option's name as text (`"Model"`), also kept as `aventail.Mode.Model`.
- **Preset** (`fetch "aventail/project":Preset;`): The export presets, as the Export window lists them. One of `Preset.Godot`, `Preset.Unity`, `Preset.UnrealEngine`, `Preset.Solis`, `Preset.SubstancePainter`, `Preset.Printing`. In Lua it's the option's name as text (`"Godot"`), also kept as `aventail.Preset.Godot`.

## Related

- [Scripting](scripts.md): running a script, the output, mistakes and limits.
- [Concepts](concepts.md#scripting): script, LustScript, Lua, sandbox and the other words.
