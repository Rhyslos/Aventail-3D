# Scripting

> Run a small program of your own (a script) on your project: it can read every shape, its points and polygons and what's selected, and change them: make shapes, build them from points, move, rename, bevel, extrude, mark seams, unwrap, give materials and select. Everything a script does is one step you can [undo](concepts.md#objects-and-scenes). Scripts are written in LustScript or Lua, and run safely.

## When to use it

A [script](concepts.md#script) does by itself what would take many clicks, or what the app has no button for:

- list every shape with its polygon count, to check a model fits a game's budget;
- find the shapes still called "Cube" or "Cylinder" before you export a kit;
- count the hard edges and seams on each shape, or which polygons use which material;
- bevel every hard edge of the selected shapes at once, or give every polygon facing up a "Grass" material;
- build a shape from numbers: a gear, a staircase, a fence, as many as you like with one click;
- tidy up: rename shapes by a pattern, hide the helpers, group the pieces of a kit.

Scripts with their own settings in Attributes come with a later update. New to scripts? [Your first script](your-first-script.md) builds one step by step.

Scripts come in two languages, and the app runs both the same way, with the same calls:

- **[LustScript](concepts.md#lustscript)** (`.lsc` files), the app's own. The app checks the whole script before running any of it, so mistakes are found with their line before anything happens.
- **[Lua](concepts.md#lua)** (`.lua` files), which many games and programs use for scripts. Lua finds its mistakes only as it runs.

## How to use it

1. Open the **Scripts** panel: **Window → Scripts**, or press **F11**. It opens beside Coordinates at the bottom of the window.
2. Click **+** over **Your scripts**, give the script a name, choose **LustScript** or **Lua**, and click **Make it**. The new script opens in the editor with a few lines to start from; the examples below are good to copy in.
3. Press **Ctrl + Enter** in the editor (or click **Run** over it). What the script prints shows in the **Output** under the editor as it runs, and how it ended (finished, stopped, or stopped on a mistake). The status bar says the script is running, with **Stop**.

You can also write a script in any text editor (Notepad will do), save it with `.lsc` or `.lua` at the end of its name, and run it with **Scripts → Run a script file…**.

Scripts run on a copy of your project, beside the app, so the app never freezes while one runs: you can keep looking around the view. What a script changes [lands](#changing-the-scene) on your project when it ends, as one step.

### A first script in LustScript

```
-- name: List shapes
-- about: Prints every shape's name and how many polygons it has.
-- api: 1

fetch "aventail/scene":[Scene, Shape];

loop (Shape shape in Scene:shapes()) {
    print(shape:name() + ": " + shape:polygonCount().text() + " polygons");
}
end
```

- The lines starting `--` are comments. The first few are the script's [header](#the-header-a-scripts-first-lines).
- `fetch "aventail/scene":[Scene, Shape];` brings in the parts of the app the script uses: the scene, and shapes. The [Script reference](script-reference.md) says which file each part is in.
- `loop (… in Scene:shapes())` goes through every object in the Objects panel, top to bottom.
- `shape:name()` and `shape:polygonCount()` ask a shape about itself; `.text()` turns a number into text, and `+` joins texts.
- Every LustScript file ends with `end`.

### The same in Lua

```lua
-- name: List shapes
-- about: Prints every shape's name and how many polygons it has.
-- api: 1

for _, shape in ipairs(aventail.Scene:shapes()) do
    print(shape:name() .. ": " .. shape:polygonCount() .. " polygons")
end
```

In Lua the app's parts are always there under `aventail.`, with no `fetch`; `..` joins texts.

### Reading what's selected

```
-- name: Selected polygons
-- api: 1

fetch "aventail/scene":[Scene, Shape];
fetch "aventail/selection":Selection;

loop (Shape shape in Scene:selected()) {
    int count = #Selection:polygons(shape);
    print(shape:name() + " has " + count.text() + " polygons selected");
}
end
```

### Commands: a script with settings

A script can also make a **command**: a piece with its own settings (a width, a number of segments) and a `run` part that does the work. When you run the file, each command runs once with its settings as they're written; later updates show the settings in Attributes so you can change them.

```
-- name: Hard edge count
-- api: 1

fetch "aventail/scene":[Scene, Shape];

component CountHardEdges {
    has {
        int least = 1;
    }

    method run() {
        loop (Shape shape in Scene:selected()) {
            int hard = #shape:hardEdges();
            is hard >= least? then {
                print(shape:name() + ": " + hard.text() + " hard edges");
            }
        }
    }
}

pass CountHardEdges;
end
```

In Lua, a command is made with `command`:

```lua
-- name: Hard edge count
-- api: 1

local Count = command("CountHardEdges", { least = 1 })

function Count:run()
    for _, shape in ipairs(aventail.Scene:selected()) do
        local hard = #shape:hardEdges()
        if hard >= self.least then
            print(shape:name() .. ": " .. hard .. " hard edges")
        end
    end
end
```

## Changing the scene

Scripts change the scene with the same edits the app's tools make: a script's bevel is the Bevel tool's bevel, recorded in the shape's [edit layers](edit-layers.md) where you can change it or switch it off afterwards, and a new shape is made as the Create menu makes it. Nothing a script does is hidden from you, and everything can be undone.

### Bevel every hard edge

```
-- name: Bevel hard edges
-- about: Bevels every hard edge of the selected shapes.
-- api: 1

fetch "aventail/scene":[Scene, Shape];

loop (Shape shape in Scene:selected()) {
    shape:bevel(shape:hardEdges(), 0.2, 1);
}
end
```

`shape:bevel(edges, width, segments)` rounds off the edges it's given: here every hard edge, 0.2 cm (2 mm) into each side, in one row. Sizes are in centimetres **as you see them in the scene**, as the Bevel tool measures, even on a shape you've scaled.

The same in Lua:

```lua
-- name: Bevel hard edges
-- api: 1

for _, shape in ipairs(aventail.Scene:selected()) do
    shape:bevel(shape:hardEdges(), 0.2, 1)
end
```

### Build a shape from points

`Scene:build(name, points, polygons)` makes a new shape from where its points are and which points each polygon goes through. Here, a flat triangle:

```
-- name: Triangle
-- api: 1

fetch "aventail/scene":[Scene, Shape, Polygon];

vec3 array points = [[0, 0, 0], [100, 0, 0], [0, 100, 0]];
Polygon array polygons = [new Polygon { corners = [0, 1, 2] }];
Shape triangle = Scene:build("Triangle", points, polygons);
triangle:setPosition([0, 0, 50]);
end
```

- Points are in centimetres, in the new shape's own space.
- Each polygon lists its **corners**, the points it goes through, **by their place in the list of points, counting from 0** (the first point is 0), in order around the side the polygon faces (counter-clockwise, seen from that side). That place is also each point's id in the new shape.
- A polygon needs at least three corners, each a point in the list, none twice; the script stops with the reason when one isn't right, and nothing is made.

In Lua the points are `vector.create(x, y, z)` (or tables `{ x = 0, y = 0, z = 0 }`) and each polygon is a table with its corners, still counted from 0:

```lua
local tri = aventail.Scene:build("Triangle",
    { vector.create(0, 0, 0), vector.create(100, 0, 0), vector.create(0, 100, 0) },
    { { corners = { 0, 1, 2 } } })
```

### Everything a script can change

| Change | Call |
|---|---|
| Make a ready-made shape, as the Create menu does | `Scene:add(Primitive.Cube)` (`aventail.Primitive.Cube` in Lua) |
| Build a shape from points and polygons | `Scene:build(name, points, polygons)` |
| Put shapes in a group | `Scene:group(shapes)` |
| Rename, move, turn and size a shape | `shape:setName("Wall")`, `setPosition`, `setRotation` (degrees), `setScale` |
| Show, hide or delete a shape | `shape:show()`, `shape:hide()`, `shape:delete()` |
| Bevel, extrude and inset | `shape:bevel(edges, width, segments)`, `shape:extrude(polygons, distance)`, `shape:inset(polygons, amount)` |
| Mark hard edges and seams | `shape:markHard(edges)`, `shape:markSeams(edges)` |
| Unwrap its UVs | `shape:unwrap()`, with the UV settings you have now |
| Give polygons a material | `shape:setMaterial(polygons, "Steel")`: the material is added to the shape if it hasn't got it |
| Bake it into a plain mesh | `shape:makeEditable()` |
| Select | `Selection:set(shapes)`, `setPoints`, `setEdges`, `setPolygons` (one shape's), `Selection:clear()` |

The [Script reference](script-reference.md) has each one in full. A new shape is selected when it's made, as when you make one yourself.

### One step to undo

While a script runs, it works on its own copy of your project, and it reads its own changes as it goes: after `shape:bevel(…)`, `shape:polygonCount()` counts the new polygons. When it **ends well**, everything it did lands on your project at once, as **one step** named after the script: one **Ctrl + Z** takes the whole run back, and **Ctrl + Y** brings it again. The status bar says how many changes it made.

When a script **stops on a mistake, is stopped, or runs out of time or memory, nothing it did is kept**: your project is exactly as it was. You never get half a script.

### While a script runs

While a script runs, its project is **held**: changes to it wait until the script ends. If you try one (deleting a shape, undo, a tool), it's turned away with a note saying a script is running; press **Esc** to stop the script if you don't want to wait. You can still look around the view, read the panels, and work in your other projects' tabs, which are free. A script keeps running in a tab you've left, and lands what it did when it ends; that tab doesn't go to sleep while it runs.

### When the scene says no

Some changes can't be made: renaming to an empty name, bevelling a group (a group has no shape), extruding a polygon the shape hasn't got. The script stops there with the scene's own reason (`shape:setName(name) couldn't be done: a name can't be empty.`), and nothing it did is kept. Even if a Lua script catches the mistake with `pcall`, nothing it does afterwards is kept: once a change has been refused, the run can't land.

Points, edges and polygons are named by their ids, as a shape gives them back. Edits change a shape's ids, so read them again after an edit rather than keeping old ones.

## The header: a script's first lines

A few comment lines at the very top say what a script is. The app reads them **without running anything**, so you can see what a script is before trusting it. All of them are optional.

| Line | What it says |
|---|---|
| `-- name: List shapes` | The script's name, shown in the panel and the status bar. Without it, the file's name is used. |
| `-- about: …` | What it does, in a sentence. |
| `-- api: 1` | The [scripting version](concepts.md#api-version) it was written for. Without it the app takes version 1 and says so in the output. A script for a newer version than the app has is refused, with a note that it needs a newer app. |
| `-- count: segments` | The setting **Ctrl + wheel** will change, once commands show their settings in Attributes. |
| `-- asks: read the project folder` | What the script will ask to reach (also `read and write the project folder`, or `read and write a folder you choose`). Nothing can be reached outside the project yet; asking and approving come with a later update. |

A line the app can't read (an `api` that isn't a whole number, say) is noted in the output with its line number; the script still runs.

## The Scripts panel

Open or put away the Scripts panel with **Window → Scripts**, **Scripts → Scripts panel** or **F11**; it also shows by itself whenever a script runs. It sits beside Coordinates, and showing it makes that strip at the bottom of the window taller so there's room to work (drag its edge to make it bigger or smaller; putting the panel away gives Coordinates its slim strip back). It has four parts.

### Your scripts

On the left: your **[script library](concepts.md#script-library)**, a folder of your own that's the same in every project.

- **+** makes a new script: type its name, choose **LustScript** or **Lua**, and click **Make it** (or press Enter). It starts with a header and a few lines that say the name of every selected shape, and opens in the editor. A name a file can't have (with `/`, `:` or `?` in it, say) or one already used is refused, saying why.
- **Double-click** a script to edit it.
- **Right-click** a script for **Edit**, **Run**, **Open in your own editor**, and **Show the folder** (your library's folder, in your computer's file browser).

Scripts added to the folder outside the app show in the list within a couple of seconds. The app also keeps a folder called `aventail` in your library, with its **description files** (`scene.lsc` and the others): they list every call a script can make, so an editor outside the app (the Lust editor, say) can check your LustScript too. The app writes them again when they change, so don't edit them.

### The editor

In the middle: the script you're editing, its lines numbered and coloured: the language's own words, names of the app's parts (`Scene`, `Shape`), numbers, text in quotes and comments each in their own colour.

- **Mistakes are found as you type.** A moment after you stop typing, the app checks the script, without running any of it, and **underlines** each mistake it finds in red. The first one is written over the editor (`bevel.lsc:9:13  n is an int, but this is text.`): click it to go to its line. "No mistakes found" means the check found none. Lua scripts are checked for their grammar only; LustScript is checked fully, calls and all.
- **It saves itself.** A second after you stop typing, the script is saved to its file; it's saved too when you open another script, run it, or close the panel or the app. **Ctrl + S** still saves your project, not the script.
- **Ctrl + Enter** runs the script. With some lines selected, it runs **only those lines** (whole lines, even if you selected part of one), keeping the script's header and its `fetch` lines; the output names the run with its lines ("Bevel hard edges (lines 7 to 9)"). Handy for trying a piece, but a few lines from the middle of a `loop` or a `component` may not make sense on their own: the mistake says so.
- **Run** over the editor runs the whole script. **×** closes it (it's saved).
- **Ctrl + Z** and **Ctrl + Y** undo and redo your typing while you're typing in the editor; your project's undo is untouched.

**Open in your own editor** opens the script in the program your computer uses for `.lsc` or `.lua` files (set that up as for any kind of file, for example with **Open with** in your file browser). When you save it there, the app **loads it again** within a second, and the output says so. If you were typing in the app's editor too and hadn't stopped yet, a bar asks which to keep: **Load theirs** (the other program's) or **Keep mine** (yours, written over theirs).

### The output

Under the editor: the [output](concepts.md#script-output) of every script run in this project, newest at the bottom:

- **Running** and the script's name, when one starts;
- what it printed (with `print` or `Script:say`), in a fixed-width font;
- **Warning:** lines from `Script:warn`, in a warning colour;
- notes from the app, greyed (a script loaded again, say);
- **mistakes**, in red, with the file and line they're at (`tidy.lua:5  attempt to index nil with 'x'`): a mistake in the script you're editing is underlined; click it to go to its line;
- how the run ended;
- what you typed on the console line, after `>`.

**Clear** empties it. A script can also say how far it has got with `Script:progress(0.5, "Reading shapes")`: the status bar shows a bar while it runs.

### The console line

At the bottom: the **[console](concepts.md#console)** line, for trying one line at a time without making a script. Type a line and press **Enter**: it runs on your project at once, and what it changes is one step you can undo (called "Console line"). Its mistakes show at its own line.

- **Shift + Enter** adds a line, to run a few lines together.
- **Up** brings back the lines you ran before, newest first (the last 100); **Down** goes forward again, and past the newest gives back what you were typing.
- The language is chosen beside the line; it follows the script you open. In LustScript every part of the app is there without a `fetch`, and `end` is added for you.
- Each line runs on its own: a variable made on one line is gone on the next (type them together with Shift + Enter instead).

```text
Shape box = Scene:add(Primitive.Cube);
```

(LustScript wants what a call gives back kept, here in `box`; in Lua `aventail.Scene:add(aventail.Primitive.Cube)` is enough.)

## Stopping a script

Press **Esc**, or click **Stop** in the status bar (or **Scripts → Stop the script**). The script stops at once, and the output says it was stopped. Esc stops a script only while one is running, and not while you're typing in a text box; the rest of the time it does its usual jobs.

## What scripts can and can't do

Scripts run in a [sandbox](concepts.md#sandbox), so a script from someone else can't harm your computer or your work:

- **They read the project through the app's own calls**: shapes, their points, polygons, hard edges, seams and materials, what's selected and the mode you're in. The [Script reference](script-reference.md) lists every call.
- **They can't open files, reach the internet or start other programs.** Lust's own `Files` and `Web` calls, and Lua's `io` and `os`, aren't there; using them is a mistake that says so.
- **They change the scene only with the app's own edits**, on a copy of the project, landing as one step you can undo; nothing lands from a run that fails or is stopped.
- **A run may make at most 100,000 changes.** A shape built with `Scene:build` may have at most 100,000 points and 100,000 polygons, and a run may build at most 5,000,000 points and polygons in all. A list given to one call (edges to bevel, say) may hold at most 100,000 items.
- **A run may take 10 seconds and 256 MB of memory.** Past either, the script is stopped with a message saying which, so a loop that never ends can't hold the app.
- **A script prints at most 1,000 lines** of up to 4 KB each; the rest are left out, with a note.
- **Heavy shapes are read a chunk at a time:** `shape:points(from, count)` and `shape:polygons(from, count)` give at most 100,000 at once.

## Mistakes and what they mean

Every mistake says where it is and what's wrong, in words:

- **Before running** (LustScript): `list.lsc:7:5  there's no method called nme.` Nothing has run; fix the line and run it again.
- **While running:** the line of your script that failed. `aventail.Scene:find(name) wants text for name, and was given nothing (nil). Is a value missing, its name misspelt, or the colon before the call's name written as a dot?` (in Lua, `aventail.Scene.find(…)` with a dot is a common slip).
- **Ran out of time:** *List shapes was stopped after 10 seconds, the longest a script may run. Is there a loop that never ends?*
- **Ran out of memory:** *… it used more than 256 MB of memory, the most a script may use.*
- **A change the scene refused:** *shape:setName(name) couldn't be done: a name can't be empty.* Nothing the script did is kept.
- **Too many changes:** *… would be more than 100000 changes, the most a script may make in one run.* Nothing is kept; split the work into smaller runs.

## Shortcuts

| Action | Shortcut |
|---|---|
| Show or put away the Scripts panel | `F11` (or **Window → Scripts**) |
| Run the script you're editing (only the selected lines, when some are) | `Ctrl + Enter`, in the editor |
| Run the console line | `Enter`, on the console line |
| A new line on the console line | `Shift + Enter` |
| Earlier console lines | `Up` and `Down`, on the console line |
| Undo and redo typing | `Ctrl + Z` and `Ctrl + Y`, in the editor or on the console line |
| Run a script file | **Scripts → Run a script file…** (no key yet) |
| Stop a running script | `Esc` (only while one runs), or **Stop** in the status bar |
| Take back everything a script did | `Ctrl + Z`, once, after it has ended |

## Tips and common problems

- **Problem:** *every file ends with the word end on its last line.* **Why:** a LustScript file must finish with `end`. **Fix:** add a last line with `end`. Lua files don't need it.
- **Problem:** *there's no file aventail/scene.lsc.* **Why:** the path in `fetch` is misspelt. **Fix:** write it as in the [Script reference](script-reference.md), without `.lsc`.
- **Problem:** a Lua script fails with *attempt to index nil*. **Why:** something you asked for was nothing: a shape `find` didn't find, or a misspelt name. **Fix:** check it before using it: `if shape then … end`.
- **Tip:** counting in LustScript starts at 0 (`shapes[0]` is the first); in Lua at 1. `#list` is how many in both.
- **Tip:** the same shape always gives the same value, so `==` tells whether two are the same shape.
- **Tip:** a `-- name:` line makes the output and the status bar easier to read than the file's name, and it names the step in Edit → Undo.
- **Problem:** *shape:bevel(…) was given the edge from point 0 to 1, which the shape hasn't got.* **Why:** an earlier edit changed the shape, and its ids with it. **Fix:** read the edges or polygons again after each edit.
- **Problem:** the editor says *No mistakes found* but the Lua script fails when it runs. **Why:** Lua is only checked for its grammar before running; misspelt names are found as it runs. **Fix:** read the mistake in the output and click it to go to its line. LustScript finds those before running.
- **Problem:** Ctrl + Enter on a few lines says something like *there's no name called width*. **Why:** the lines use something made on lines you didn't select. **Fix:** select those lines too, or run the whole script with **Run**.
- **Problem:** the script finished but nothing changed. **Why:** it was given empty lists (no hard edges, nothing selected), or what it asked for was already so (edges already hard): those change nothing. **Fix:** print what it's working on to check.

## Related

- [Your first script](your-first-script.md): a script built step by step, in LustScript and Lua.
- [Script reference](script-reference.md): every call, in LustScript and Lua.
- [Concepts](concepts.md#scripting): script, LustScript, Lua, sandbox, API version and the other words.
- [The Objects panel](objects.md): the order `Scene:shapes()` follows.
- [Points, Edges and Polygons modes](modes.md): what `Selection` reads and sets.
- [Edit layers](edit-layers.md): where a script's bevels, extrudes and other edits are recorded.
