# Scripting

> Run a small program of your own (a script) on your project: it can read every shape, its points and polygons and what's selected, and change them: make shapes, build them from points, move, rename, bevel, extrude, mark seams, unwrap, give materials and select. Everything a script does is one step you can [undo](concepts.md#objects-and-scenes). Scripts are written in LustScript or Lua, and run safely: one can reach your files only in a folder it asks for and you allow. Keep scripts with a project, or share them as files, and let some run by themselves as a project opens, is saved or is exported.

## When to use it

A [script](concepts.md#script) does by itself what would take many clicks, or what the app has no button for:

- list every shape with its polygon count, to check a model fits a game's budget;
- find the shapes still called "Cube" or "Cylinder" before you export a kit;
- count the hard edges and seams on each shape, or which polygons use which material;
- bevel every hard edge of the selected shapes at once, or give every polygon facing up a "Grass" material;
- build a shape from numbers: a gear, a staircase, a fence, as many as you like with one click;
- tidy up: rename shapes by a pattern, hide the helpers, group the pieces of a kit;
- check every export by itself, and stop one with a shape far too heavy for a game.

Six [example scripts](#example-scripts) do some of these: add one from **Scripts → Example scripts** and read it, run it, change it.

A script can become a **command** of your own, with settings you change in Attributes, a place on the command palette and a key, like the app's own tools ([Scripts as commands](#scripts-as-commands)). New to scripts? [Your first script](your-first-script.md) builds one step by step.

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

A script can also make a **command**: a piece with its own settings (a width, a number of segments) and a `run` part that does the work. When you run the file from the Scripts panel, each command runs once with its settings as they're written. Run from the **Scripts** menu, a command runs on its own, and its settings show in Attributes so you can change them: see [Scripts as commands](#scripts-as-commands).

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

## Scripts as commands

Every script in your library becomes a [command](concepts.md#script-command) of its own: something you can run from the **Scripts** menu, find with **command search** (Space), put on the command palette or give a key, like the app's own tools. The app reads your scripts' commands by itself, a moment after you save one: no restart.

- **A script that makes commands** (LustScript `component`s with a `run`, or Lua's `command(…)`) gives one menu entry for each. With one command, it's named after the script's `-- name:` line; with several, "Script name: Command name" (`CountHardEdges` reads as "Count hard edges").
- **A script that makes no commands** is one command of its own, which runs the whole script.
- **A script with a mistake** has no commands until it's mended: the [script manager](#the-script-manager) says what's wrong.

### Running one, and its settings in Attributes

Choose the command in the **Scripts** menu (under **Script manager…**). It runs with the settings you last used (the first time, those written in the script), and lands as one step named after it, like any script. Its settings then show **at the top of Attributes**, by their names in words (`least` reads "Least"):

- a `bool` is a tick box, an `int` and a `float` are numbers you drag or type, a `string` is a text box, a [state](concepts.md#state)'s options are a drop-down, a `vec3` is three numbers and a `colour` a colour button. Settings of other kinds keep what the script gives them.
- **Change one and the command runs again** with it, in place of the run before: it stays **one step**, so one Ctrl + Z takes it all back however many times you changed it. While you drag a number, the newest value wins.
- If the command fails with the new settings, the last run that worked comes back, with its settings, and the status bar says so (the output says why).
- The settings stay in Attributes until you change, undo or select something else, as a tool's last edit does. Run the command again to bring them back.
- **Ctrl + wheel** in the view changes the setting the script's `-- count:` line names (a whole-number one, such as `segments`), one step a notch, from 1 up: the command runs again each time.

In Lua, a setting is a whole number when it starts at one (`segments = 1`) and has decimals when it starts with them (`width = 0.2`); start a decimal setting with decimals (`2.5`, not `2`). Lua settings show in Attributes by name, A to Z; LustScript's in the order written.

### On the command palette, and keys

Right-click a script in **Your scripts** (or use the [script manager](#the-script-manager)):

- **Add to the command palette** puts the command on the row of icons under the menus, after the app's own, with a page icon; point at it for what it does, click to run it. Right-click it there and **Take it off the command palette**.
- **Give it a key…** opens a small window: press the keys you want (with Ctrl, Shift or Alt if you like). A key the app already uses anywhere, or one another script command has, is refused, saying what uses it; **F6** to **F10** and **F12** are free, as are most **Ctrl + Shift** letters. **Take its key away** clears it; **Esc** cancels. A command's key works when you're not typing and no window is open.

Scripts get **no keys as standard**: only those you give them.

### The script manager

**Scripts → Script manager…** lists every script in your library: its name and file, what it says it does, what it asks to reach, and its mistakes if it has any. For each:

- **On**: switch a script off and its commands leave the Scripts menu, command search, the palette and their keys (it still runs from the Scripts panel). Handy for a script you're still writing.
- Each of its commands, with **Give it a key…** (or its key) and **On the command palette**.
- **Edit** opens it in the Scripts panel; **Reload** reads its commands again now; **Rename…** gives it another name (its keys, palette place and settings follow it); **Remove…** deletes its file from your library, after asking (it can't be brought back); **Show in its folder** opens your library's folder.

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
| `-- about: …` (shown) | Also shown in command search, on the palette button and in the script manager. |
| `-- count: segments` | The setting **Ctrl + wheel** changes while the command's settings show in Attributes ([Scripts as commands](#running-one-and-its-settings-in-attributes)). |
| `-- asks: read the project folder` | What the script asks to reach: `read the project folder`, `read and write the project folder`, or `read and write a folder you choose` (one line each). It's shown before you install a script and asked about before it first runs ([Files and permissions](#files-and-permissions)). A script without one reaches no files. |

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

## Keeping scripts with a project

Your library's scripts are yours, in every project. To give a script to whoever opens a project (a teammate, or you on another computer), keep it **with the project**: right-click it in **Your scripts** → **Keep with this project**. A copy goes into the project file, and travels with it when you save. It's one step Ctrl + Z takes back; keeping the script again later updates the copy.

A project's own scripts show in the Scripts panel under **In this project**, below your own. They can be **looked at but not changed** there (double-click one: it shows coloured, with line numbers). Right-click one for:

- **Look at it**, and **Run** once the project's scripts are on;
- **Copy to your library**, to change it: edit your copy, then **Keep with this project** again to update the project's;
- **Export…**, to save it as a file ([Sharing a script](#sharing-a-script));
- **Take out of this project** (Ctrl + Z puts it back).

### Scripts in a project from someone else

A script can change your scene, so **a project's scripts never run by themselves**: not when it opens, and not even to read their commands, until you turn them on, as Blender does. When a project has scripts you haven't turned on, a bar under the menus says *This project has 2 scripts. They won't run until you turn them on*, with:

- **Look at them**: opens the Scripts panel with the first one showing, so you can read what they do first;
- **Turn them on**: lets them run on this computer, and puts their commands in the Scripts menu and command search (a command named like one of yours says "(this project's)");
- **Not now**: puts the bar away; they stay off.

Changed your mind? **Scripts → Script manager…** lists them under **In this project**, with **Turn them off**: they're off on this computer again until you turn them on.

Your computer remembers the scripts you turned on, by what's written in them: the same script in another project is on too, and a script that's **changed in any way** is off again until you turn it on. Scripts you keep with a project yourself are on already, so your own projects open with their scripts working. Turning scripts on doesn't let them reach your files: that's asked separately ([Files and permissions](#files-and-permissions)). A project's commands get no keys or palette places: those are yours, the same in every project.

## Sharing a script

A script is shared as **one file** (`.lsc` or `.lua`):

- **Scripts → Export a script** lists your scripts and the project's: pick one and choose where to save it. **Export…** on a script's right-click menu in the Scripts panel does the same.
- **Scripts → Install a script…** adds a script someone gave you to your library. First a window shows its name, what it says it does, the [scripting version](concepts.md#api-version) it was written for, what it asks to reach, and **its code**: read it before clicking **Install it**. Installing runs nothing and allows nothing. A script with the name of one you already have is added with a number ("Bevel 2.lsc"), never over yours.

## Scripts that run by themselves

Some jobs belong at a moment rather than on a button: tidying names **every time you save**, or checking a model **every time you export it**. A script can run by itself at three [moments](concepts.md#moment):

| Moment | When it runs | What it can do |
|---|---|---|
| **A project opens** (`ProjectOpened`) | just after the project opens | anything a script can do; what it changes is one step you can undo |
| **A project is saved** (`ProjectSaved`) | just before the file is written | anything; what it changes lands first as one step, so it's **in the saved file** |
| **An export starts** (`ExportStarting`) | before anything is written, or you're asked where | read what goes in, warn, or **stop the export with a reason**; it can't change the scene |

### Writing one

In LustScript, a component reacts to a moment with `on` and the moment's name, fetched from `aventail/moments` like the calls:

```
-- name: Tidy names
-- api: 1

fetch "aventail/scene":[Scene, Shape];
fetch "aventail/moments":ProjectSaved;

component TidyNames {
    on ProjectSaved() {
        loop (Shape shape in Scene:shapes()) {
            shape:setName(shape:name().trim());
        }
    }
}

end
```

In Lua, a command gets a function called `on_` and the moment's name:

```lua
-- name: Tidy names
local Tidy = command("TidyNames", {})

function Tidy:on_ProjectSaved()
    for _, shape in ipairs(aventail.Scene:shapes()) do
        shape:setName(shape:name():match("^%s*(.-)%s*$"))
    end
end
```

A component (or Lua command) can have a `run()` too, so you can also run it by hand from the Scripts menu; one with only an `on` isn't in the menu. It reacts with its **settings as you last used them** there.

### Switching it on

A script never runs by itself until you say so. Open **Scripts → Script manager…**: under each script that reacts to a moment, **Runs by itself when** has a switch for each one it reacts to (**a project opens**, **a project is saved**, **an export starts**). Tick it, and from then on the script runs at that moment, in every project. Untick it to stop. A project's own scripts get their switches under **In this project** once they're [turned on](#scripts-in-a-project-from-someone-else); until then they never run.

### What happens when one runs

- Opening, saving or exporting **waits** while its scripts run, **one after another in name order**, each for **at most 2 seconds**. The status bar says which is running (**Esc** stops it), then which ran: *Saved crate.avtl. Ran by itself as the project was saved: Tidy names.*
- A script that stops on a mistake, is stopped, or runs past its 2 seconds is said in the Scripts panel's output and the status bar, nothing it did is kept, and **the save or export goes on** anyway. A script can't hold up your work.
- A script that asks to reach files you haven't allowed yet **doesn't run** at the moment (nobody may be there to answer): the output says so. Allow it in the script manager first.
- What a script changes is one step, named after it: Ctrl + Z takes it back.

### Checking an export

While an export starts, a script reads what goes in, and can stop it:

| Call | What it does |
|---|---|
| `Export:shapes()` | the objects going into the file, in the Objects panel's order (every shown one, or the selected ones and what's inside them) |
| `Export:format()` | the kind of file: `Format.Glb`, `Format.Obj` or `Format.Stl` |
| `Export:cancel(reason)` | stops the export: nothing is written, and the status bar says *The export didn't go ahead: Check before export stopped it (Crate has more than 100000 polygons).* |

Every export counts: the Export window's (and **Export again**), a kit export, **Export to game project**, and the export into your game project after every save. A script stopping that last one lets the **save** go on without the export. While an export starts a script **can't change the scene** (the export is made from it as it is): trying stops the script with a note. Files the script writes are only written if the export goes ahead.

## Example scripts

Six scripts to learn from, run and change. **Scripts → Example scripts** copies one into your library (with a number added if you have one of the same name, so nothing of yours is written over) and opens it in the editor. They're in LustScript, with comments saying what each part does.

- **Rename by pattern**: renames the selected shapes (every shape, when none is selected), changing one piece of each name for another (`Cube` to `Crate`, as standard) and putting a word in front (`prefix`, such as `SM_`). Run it from the Scripts menu, then change **Find**, **Replace with** and **Prefix** in Attributes: it runs again with them, one step to undo. *Shows:* reading and changing names, settings in Attributes.
- **Gear**: builds a gear from points and polygons with `Scene:build`: twelve teeth, 10 cm across, as standard. Change **Teeth** with **Ctrl + wheel** over the view, or any setting in Attributes, and it's built again. *Shows:* making a shape from numbers, polygons facing the right way.
- **Check before export**: warns about shapes still called by a standard name ("Cube 3") and shapes with more polygons than **Most polygons** (100,000). Run by hand it only warns. Switch it on for **an export starts** in the script manager, and it **stops** an export holding a shape that's too heavy, saying which. *Shows:* `on ExportStarting()`, `Export:shapes()` and `Export:cancel`.
- **Export each shape**: exports each selected shape (every shape, when none is selected) to a `.glb` file of its own named after it, for Godot, in an `exports` folder beside the project. It asks to read and write the project's folder, so it asks you the first time; save the project first. *Shows:* permissions and `Project:exportShapes`.
- **Select polygons facing up**: in each selected shape, selects the polygons facing up within **Most tilt** (10°), reading a heavy shape 100,000 polygons at a time. Switch to Polygons mode to see them, then give them a material or extrude them. "Up" is the shape's own up, as it was modelled. *Shows:* reading polygons in chunks, selecting.
- **Tidy names**: takes spaces off the ends of every shape's name and turns double spaces into one. Run it by hand, or switch it on for **a project is saved** so every save is tidy. *Shows:* `on ProjectSaved()`.

## Files and permissions

Scripts reach no files unless they ask, and you allow it. A script asks with a line at its top ([The header](#the-header-a-scripts-first-lines)) for one of three [permissions](concepts.md#permission):

- `-- asks: read the project folder`: read files in the folder the project is saved in;
- `-- asks: read and write the project folder`: read and write them;
- `-- asks: read and write a folder you choose`: read and write files in one folder you pick, and nowhere else.

The first time such a script runs, a window says what it asks for: **Allow and run**, or **Don't run it**. For a folder you choose, click **Choose…** first. Your answer is remembered for that script; if a later version **asks for more**, you're asked again. **Scripts → Script manager…** shows what each script (yours, and the project's) was allowed, with **Take back**, and **Allow…** to allow it before it runs. The console line never reaches files.

With permission, a script uses three calls ([Script reference](script-reference.md#project)):

```lua
-- asks: read and write the project folder
local names = aventail.Project:readText("notes/names.txt")
aventail.Project:writeText("notes/count.txt", "Shapes: " .. #aventail.Scene:shapes())
aventail.Project:exportShapes(aventail.Scene:selected(), "exports/chair.glb", aventail.Preset.Godot)
```

- **Paths are written from inside the folder**, with `/` (or `\`) between folders: `notes/names.txt`. Folders are made as needed. A path that climbs out with `..`, a whole path (`C:\…`, `/home/…`), a link (shortcut) to somewhere else, or a name Windows can't keep (`CON`, `what?.txt`) is refused, saying why.
- **What a script writes is written when it finishes well**, each file whole. A script that stops on a mistake, is stopped or runs out of time writes nothing, as it changes nothing in the scene. A script reading a file it wrote reads what it wrote.
- **`exportShapes`** exports as the Export window does with that preset (`Preset.Godot`, `Unity`, `UnrealEngine`, `Solis`, `SubstancePainter` or `Printing`): the path ends in the preset's ending (`.glb`, `.obj` or `.stl`), and files that go beside it (an OBJ's `.mtl`, textures) go next to it. Shapes inside a group given to it go too; hidden ones don't.
- **An unsaved project has no folder**: save it first.
- At most 16 MB is read from one file, and a run writes at most 1,000 files and 256 MB.

## Stopping a script

Press **Esc**, or click **Stop** in the status bar (or **Scripts → Stop the script**). The script stops at once, and the output says it was stopped. Esc stops a script only while one is running, and not while you're typing in a text box; the rest of the time it does its usual jobs.

## What scripts can and can't do

Scripts run in a [sandbox](concepts.md#sandbox), so a script from someone else can't harm your computer or your work:

- **They read the project through the app's own calls**: shapes, their points, polygons, hard edges, seams and materials, what's selected and the mode you're in. The [Script reference](script-reference.md) lists every call.
- **They can't reach the internet or start other programs, and reach files only as you allow** ([Files and permissions](#files-and-permissions)). Lust's own `Files` and `Web` calls, and Lua's `io` and `os`, aren't there; using them is a mistake that says so.
- **A project's own scripts don't run until you turn them on** ([Scripts in a project from someone else](#scripts-in-a-project-from-someone-else)).
- **They change the scene only with the app's own edits**, on a copy of the project, landing as one step you can undo; nothing lands from a run that fails or is stopped.
- **A run may make at most 100,000 changes.** A shape built with `Scene:build` may have at most 100,000 points and 100,000 polygons, and a run may build at most 5,000,000 points and polygons in all. A list given to one call (edges to bevel, say) may hold at most 100,000 items.
- **A run may take 10 seconds and 256 MB of memory** as standard; change both in **Edit → Settings → Scripts** (from 1 second to 10 minutes, and from 32 MB to 2 GB). Past either, the script is stopped with a message saying which, so a loop that never ends can't hold the app.
- **Reading a script's commands runs its start** (the lines outside its commands) once, in the sandbox, against an empty project: nothing it does there is kept. A start that takes longer than 2 seconds gives no commands; keep slow work inside a command's `run`.
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
- **Not allowed:** *aventail.Project:writeText(path, text) can't write out.txt: this script isn't allowed to write files. A script asks for it with a line at its top (-- asks: read and write the project folder), and you allow it when it first runs.* Add the line, run it again and allow it.
- **A path refused:** *… can't write ../out.txt: a path can't climb out of the folder with ..* Write the path from inside the folder.

## Shortcuts

| Action | Shortcut |
|---|---|
| Show or put away the Scripts panel | `F11` (or **Window → Scripts**) |
| Run the script you're editing (only the selected lines, when some are) | `Ctrl + Enter`, in the editor |
| Run the console line | `Enter`, on the console line |
| A new line on the console line | `Shift + Enter` |
| Earlier console lines | `Up` and `Down`, on the console line |
| Undo and redo typing | `Ctrl + Z` and `Ctrl + Y`, in the editor or on the console line |
| Run a script file | **Scripts → Run a script file…** (no key) |
| Run a script's command | **Scripts** menu, command search (`Space`), the command palette, or a key you give it |
| Change a command's count | `Ctrl + wheel`, while its settings show in Attributes |
| A command's menu (palette, key) | Right-click the script in **Your scripts**, or its palette button |
| Stop a running script | `Esc` (only while one runs), or **Stop** in the status bar |
| Take back everything a script did | `Ctrl + Z`, once, after it has ended |
| Keep a script with the project, export it | Right-click it in **Your scripts** |
| Look at a project's script | Double-click it under **In this project** (right-click for more) |
| Install or export a script | **Scripts → Install a script…**, **Scripts → Export a script** (no keys) |
| Add an example script | **Scripts → Example scripts** (no key) |
| Let a script run by itself at a moment | **Scripts → Script manager…** → **Runs by itself when** (no key) |

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
- **Problem:** my script isn't in the Scripts menu. **Why:** it has a mistake, its start took longer than 2 seconds, or it's switched off. **Fix:** open **Scripts → Script manager…**: it says which.
- **Problem:** a Lua setting only goes up in whole steps. **Why:** it starts at a whole number. **Fix:** start it with decimals (`width = 0.5`).
- **Problem:** *F… is taken* when giving a key. **Why:** the app or another script command uses it. **Fix:** try F6 to F10, F12 or a Ctrl + Shift letter.
- **Problem:** a project's commands aren't in the Scripts menu. **Why:** its scripts are off on this computer. **Fix:** click **Turn them on** in the bar under the menus (or right-click one under **In this project**), after looking at them.
- **Problem:** *the project hasn't been saved yet, so it has no folder.* **Why:** a script asking for the project's folder needs one. **Fix:** save the project, then run the script again.
- **Problem:** I edited a project's script and it's off again. **Why:** a changed script is a different script, so it waits to be turned on. **Fix:** turn it on again; scripts you keep with the project yourself are on already.
- **Problem:** the script finished but nothing changed. **Why:** it was given empty lists (no hard edges, nothing selected), or what it asked for was already so (edges already hard): those change nothing. **Fix:** print what it's working on to check.
- **Problem:** my script didn't run when I saved. **Why:** it isn't switched on for that moment, its script is switched off, it's a project's script that isn't turned on, or it asks for files you haven't allowed (the output says so). **Fix:** open **Scripts → Script manager…** and check its **Runs by itself when** switches.
- **Problem:** *… can't change the scene while an export is starting.* **Why:** an export is made from the scene as it is. **Fix:** warn with `Script:warn`, or stop the export with `Export:cancel(reason)` and change the scene afterwards.
- **Problem:** *Export:shapes() only works while an export is starting.* **Why:** the `Export` calls only mean something then. **Fix:** use them inside `on ExportStarting()` (Lua: `on_ExportStarting`).
- **Problem:** a LustScript `check` over `Selection:mode()` says an option is missing. **Why:** the modes include `Sculpt`. **Fix:** add `when Sculpt { … }`.

## Related

- [Your first script](your-first-script.md): a script built step by step, in LustScript and Lua.
- [Script reference](script-reference.md): every call, in LustScript and Lua.
- [Concepts](concepts.md#scripting): script, LustScript, Lua, sandbox, API version and the other words.
- [The Objects panel](objects.md): the order `Scene:shapes()` follows.
- [Points, Edges and Polygons modes](modes.md): what `Selection` reads and sets.
- [Edit layers](edit-layers.md): where a script's bevels, extrudes and other edits are recorded.
- [Command search](command-search.md) and the [Attributes panel](attributes.md): where commands are found and their settings changed.
- [Settings](settings.md#settings): the time and memory limits.
- [Export and import](export-and-import.md): the presets `exportShapes` uses.
