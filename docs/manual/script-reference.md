# Script reference

> Every call a script can make, what it takes and gives back, in LustScript and Lua.

<!-- Made from the scripting crate's table (crates/script/src/describe.rs): a test fails when it's out of date. Write it again with UPDATE_REFERENCE=1 cargo test -p app --test script_reference. -->

Scripts are explained in [Scripting](scripts.md); words like *array* and *state* are in [Concepts](concepts.md#scripting). This page lists **scripting version 1**: a script says the version it was written for with an `-- api: 1` line at its top.

In LustScript, fetch what you use from the app's own files first, such as `fetch "aventail/scene":[Scene, Shape];`. In Lua everything is under `aventail.`, such as `aventail.Scene:selected()`; a shape's own calls are written the same in both (`shape:name()`).

Version 1 only reads: no call changes the scene yet.

## Scene

The scene the script runs on: every shape in the Objects panel.

LustScript: `fetch "aventail/scene":Scene;`

| LustScript | Lua | Gives | What it does | Since |
|---|---|---|---|---|
| `Scene:shapes()` | `aventail.Scene:shapes()` | Shape array | Every object in the scene, in the Objects panel's order (top to bottom, the ones inside a group after it). | 1 |
| `Scene:selected()` | `aventail.Scene:selected()` | Shape array | The selected objects, in the Objects panel's order. | 1 |
| `Scene:find(name)` | `aventail.Scene:find(name)` | Shape | The first object with this name, in the Objects panel's order, or nothing. | 1 |

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

## Selection

What's selected, and the mode the app was in when the script started.

LustScript: `fetch "aventail/selection":Selection;`

| LustScript | Lua | Gives | What it does | Since |
|---|---|---|---|---|
| `Selection:mode()` | `aventail.Selection:mode()` | Mode | The mode the app was in when the script started. | 1 |
| `Selection:points(shape)` | `aventail.Selection:points(shape)` | int array | The ids of the shape's selected points. | 1 |
| `Selection:edges(shape)` | `aventail.Selection:edges(shape)` | Edge array | The shape's selected edges. | 1 |
| `Selection:polygons(shape)` | `aventail.Selection:polygons(shape)` | int array | The ids of the shape's selected polygons. | 1 |

## Project

The open project.

LustScript: `fetch "aventail/project":Project;`

| LustScript | Lua | Gives | What it does | Since |
|---|---|---|---|---|
| `Project:name()` | `aventail.Project:name()` | string | The project's name, as its tab shows it. | 1 |
| `Project:folder()` | `aventail.Project:folder()` | string | The folder the project is saved in, or empty text if it hasn't been saved yet. | 1 |

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
- **Mode** (`fetch "aventail/selection":Mode;`): The app's modes, as the mode palette lists them. One of `Mode.Model`, `Mode.Object`, `Mode.Points`, `Mode.Edges`, `Mode.Polygons`, `Mode.Uv`, `Mode.Paint`, `Mode.Material`. In Lua it's the option's name as text (`"Model"`), also kept as `aventail.Mode.Model`.

## Related

- [Scripting](scripts.md): running a script, the output, mistakes and limits.
- [Concepts](concepts.md#scripting): script, LustScript, Lua, sandbox and the other words.
