# Scripting

> Run a small program of your own (a script) on your project: it can read every shape, its points and polygons and what's selected, and print what it finds. Scripts are written in LustScript or Lua, and run safely.

## When to use it

A [script](concepts.md#script) does by itself what would take many clicks, or what the app has no button for:

- list every shape with its polygon count, to check a model fits a game's budget;
- find the shapes still called "Cube" or "Cylinder" before you export a kit;
- count the hard edges and seams on each shape, or which polygons use which material.

**For now scripts only read.** They can't change the scene yet: making and changing shapes, scripts with their own settings in Attributes, and an editor inside the app come with the next updates.

Scripts come in two languages, and the app runs both the same way, with the same calls:

- **[LustScript](concepts.md#lustscript)** (`.lsc` files), the app's own. The app checks the whole script before running any of it, so mistakes are found with their line before anything happens.
- **[Lua](concepts.md#lua)** (`.lua` files), which many games and programs use for scripts. Lua finds its mistakes only as it runs.

## How to use it

1. Write a script in any text editor (Notepad will do) and save it with `.lsc` or `.lua` at the end of its name. The examples below are a good start.
2. Choose **Scripts → Run a script file…** and pick the file.
3. The **Scripts** panel opens beside Coordinates: what the script prints shows there as it runs, and how it ended (finished, stopped, or stopped on a mistake). The status bar says the script is running, with **Stop**.

Scripts run on a copy of your project, beside the app, so the app never freezes while one runs: you can keep looking around the view.

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

The Scripts panel, beside Coordinates at the bottom, holds the [output](concepts.md#script-output) of every script run in this project, newest at the bottom:

- **Running** and the script's name, when one starts;
- what it printed (with `print` or `Script:say`), in a fixed-width font;
- **Warning:** lines from `Script:warn`, in a warning colour;
- notes from the app, greyed;
- **mistakes**, in red, with the file and line they're at (`tidy.lua:5  attempt to index nil with 'x'`);
- how the run ended.

**Clear** empties it. Show or put it away with **Scripts → Scripts panel**; it shows by itself whenever a script runs. A script can also say how far it has got with `Script:progress(0.5, "Reading shapes")`: the status bar shows a bar while it runs.

## Stopping a script

Press **Esc**, or click **Stop** in the status bar (or **Scripts → Stop the script**). The script stops at once, and the output says it was stopped. Esc stops a script only while one is running, and not while you're typing in a text box; the rest of the time it does its usual jobs.

## What scripts can and can't do

Scripts run in a [sandbox](concepts.md#sandbox), so a script from someone else can't harm your computer or your work:

- **They read the project through the app's own calls**: shapes, their points, polygons, hard edges, seams and materials, what's selected and the mode you're in. The [Script reference](script-reference.md) lists every call.
- **They can't open files, reach the internet or start other programs.** Lust's own `Files` and `Web` calls, and Lua's `io` and `os`, aren't there; using them is a mistake that says so.
- **They change nothing** for now: a script works on a copy of the project.
- **A run may take 10 seconds and 256 MB of memory.** Past either, the script is stopped with a message saying which, so a loop that never ends can't hold the app.
- **A script prints at most 1,000 lines** of up to 4 KB each; the rest are left out, with a note.
- **Heavy shapes are read a chunk at a time:** `shape:points(from, count)` and `shape:polygons(from, count)` give at most 100,000 at once.

## Mistakes and what they mean

Every mistake says where it is and what's wrong, in words:

- **Before running** (LustScript): `list.lsc:7:5  there's no method called nme.` Nothing has run; fix the line and run it again.
- **While running:** the line of your script that failed. `aventail.Scene:find(name) wants text for name, and was given nothing (nil). Is a value missing, its name misspelt, or the colon before the call's name written as a dot?` (in Lua, `aventail.Scene.find(…)` with a dot is a common slip).
- **Ran out of time:** *List shapes was stopped after 10 seconds, the longest a script may run. Is there a loop that never ends?*
- **Ran out of memory:** *… it used more than 256 MB of memory, the most a script may use.*

## Shortcuts

| Action | Shortcut |
|---|---|
| Run a script file | **Scripts → Run a script file…** (no key yet) |
| Stop a running script | `Esc` (only while one runs), or **Stop** in the status bar |

## Tips and common problems

- **Problem:** *every file ends with the word end on its last line.* **Why:** a LustScript file must finish with `end`. **Fix:** add a last line with `end`. Lua files don't need it.
- **Problem:** *there's no file aventail/scene.lsc.* **Why:** the path in `fetch` is misspelt. **Fix:** write it as in the [Script reference](script-reference.md), without `.lsc`.
- **Problem:** a Lua script fails with *attempt to index nil*. **Why:** something you asked for was nothing: a shape `find` didn't find, or a misspelt name. **Fix:** check it before using it: `if shape then … end`.
- **Tip:** counting in LustScript starts at 0 (`shapes[0]` is the first); in Lua at 1. `#list` is how many in both.
- **Tip:** the same shape always gives the same value, so `==` tells whether two are the same shape.
- **Tip:** a `-- name:` line makes the output and the status bar easier to read than the file's name.

## Related

- [Script reference](script-reference.md): every call, in LustScript and Lua.
- [Concepts](concepts.md#scripting): script, LustScript, Lua, sandbox, API version and the other words.
- [The Objects panel](objects.md): the order `Scene:shapes()` follows.
- [Points, Edges and Polygons modes](modes.md): what `Selection` reads.
