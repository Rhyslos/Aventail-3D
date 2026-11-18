# Your first script

> Build a small script step by step, inside the app: one that bevels every hard edge of the selected shapes by 2 mm, first in LustScript, then the same in Lua, with every line explained.

## When to use it

Read this page when you've never written a [script](concepts.md#script), or not for a while. By the end you'll have a script of your own in your [library](concepts.md#script-library) that does in one keypress what would take a dozen clicks with the Bevel tool, and you'll know how to find and fix a mistake. Nothing here needs any programming you don't already have: you'll copy lines and change numbers.

Why a script for this? Bevelling every hard edge by hand means selecting them (Select → Hard edges), picking Bevel and typing the width, shape by shape. A script does it for every selected shape at once, the same way every time, and it's one step to undo if you don't like it.

## How to use it

### 1. Something to bevel

1. **Create → Cube.** A cube's twelve edges are [hard edges](concepts.md#shading-and-normals) already.
2. Leave it selected (click it in the view or in Objects if it isn't).

### 2. A new script

1. Press **F11** (or **Window → Scripts**). The Scripts panel opens at the bottom of the window.
2. Click **+** over **Your scripts**.
3. Type the name `Bevel hard edges`, choose **LustScript**, and click **Make it**.

The script opens in the editor with a few lines to start from. Press **Ctrl + Enter** to try them: the output under the editor says the cube's name. That's a script running.

### 3. The header

Replace everything in the editor with these lines (select all with **Ctrl + A**, then type or paste):

```text
-- name: Bevel hard edges
-- about: Bevels every hard edge of the selected shapes.
-- api: 1
```

Lines starting with `--` are **comments**: the app doesn't run them. These first three are the script's **header**: its name (shown in the output and in **Edit → Undo**), what it does, and the [scripting version](concepts.md#api-version) it's written for. The editor shows comments in grey.

### 4. Bring in what the script uses

Add a blank line, then:

```text
fetch "aventail/scene":[Scene, Shape];
```

A LustScript file says which of the app's parts it uses. `Scene` is the whole scene (every shape, what's selected), and `Shape` is one shape. They're in the app's file `aventail/scene`; the [Script reference](script-reference.md) says which file each part is in. Every line that does something ends with `;`.

### 5. Go through the selected shapes

```text
loop (Shape shape in Scene:selected()) {
}
```

`Scene:selected()` asks the scene for the selected shapes. `loop (… in …) { }` goes through them one at a time, calling each one `shape` while the lines between `{` and `}` run. The `:` means "ask this thing": `Scene:selected()` is "scene, give me what's selected".

### 6. Bevel each one

Between the `{` and `}`, add:

```text
    shape:bevel(shape:hardEdges(), 0.2, 1);
```

- `shape:hardEdges()` asks the shape for its hard edges.
- `shape:bevel(edges, width, segments)` bevels them: `0.2` centimetres (2 mm) into each side, in `1` row. Sizes are in centimetres as you see them in the scene, as the Bevel tool measures.

The four spaces in front are only for reading: they show the line is inside the loop.

### 7. End the file

Every LustScript file ends with a line holding `end`. The whole script now reads:

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

Over the editor it should say **No mistakes found**. The script has been saving itself as you typed.

### 8. Run it

Press **Ctrl + Enter**. The cube's edges are rounded off, and the status bar says how many changes were made, as one step. Press **Ctrl + Z** once: the whole bevel is gone. **Ctrl + Y** brings it back. Look in the cube's [Edit layers](edit-layers.md): the bevel is there, as if you'd made it with the Bevel tool, and you can change its width there.

Select a few more shapes and run it again: all of them are bevelled at once.

### 9. Make a mistake on purpose

Change `0.2` to `"wide"` (with the quotes). A moment after you stop typing, `"wide"` is **underlined in red**, and the mistake is written over the editor:

`Bevel hard edges.lsc:8:36  width is a float, but this is a string. Use .float() to turn text into a number.`

The numbers are the line (8) and the column (36) where it is. A *float* is a number that can have a point (`0.2`), and a *string* is text. LustScript knows `bevel` wants a number for its width, so it finds the mistake **before anything runs**. Click the words over the editor to go to the line, put `0.2` back, and the underline goes.

### 10. Say what it did

Add a line inside the loop, after the bevel, to print something in the output:

```text
    print(shape:name() + ": " + shape:polygonCount().text() + " polygons now");
```

`+` joins pieces of text, and `.text()` turns the count (a number) into text. Run it again: the output says each shape's name and how many polygons it has after the bevel. Printing is the easiest way to see what a script is doing.

### The same in Lua

Make another script with **+**, this time choosing **Lua**, and call it `Bevel hard edges in Lua`. Replace its lines with:

```lua
-- name: Bevel hard edges in Lua
-- about: Bevels every hard edge of the selected shapes.
-- api: 1

for _, shape in ipairs(aventail.Scene:selected()) do
    shape:bevel(shape:hardEdges(), 0.2, 1)
    print(shape:name() .. ": " .. shape:polygonCount() .. " polygons now")
end
```

What's different from LustScript:

- No `fetch`: in Lua the app's parts are always there, under `aventail.` (`aventail.Scene`).
- The loop is `for _, shape in ipairs(…) do … end`: `ipairs` goes through a list in order (the `_` is where Lua puts each shape's place in the list, which this script doesn't need).
- Lines don't end with `;`, the file doesn't end with `end` (the `end` here closes the loop), and `..` joins text, turning numbers into text by itself.
- Lua finds most mistakes only as it runs: change `0.2` to `"wide"` and the editor says nothing until you press Ctrl + Enter; then the output says, in red, at its line: *shape:bevel(edges, width, segments) wants a number for width, and was given text.*

### Try a line at a time

The line under the output is the **[console](concepts.md#console)**. Its language is beside it. With **Lua** chosen, type this and press **Enter**:

```text
print(#aventail.Scene:selected())
```

It prints how many shapes are selected. The console is for trying a call before putting it in a script. **Up** brings the line back to change and run again.

## Settings

The numbers in the `bevel` line are the script's settings for now: change `0.2` for a wider or narrower bevel (in centimetres), and `1` for more rows (a rounder edge). A later update shows a script's settings in Attributes, like a tool's.

## Shortcuts

| Action | Shortcut |
|---|---|
| Open or put away the Scripts panel | `F11` |
| Run the script (or only the selected lines) | `Ctrl + Enter`, in the editor |
| Run the console line | `Enter` |
| Take back everything a run did | `Ctrl + Z`, once |

## Tips and common problems

- **Problem:** *every file ends with the word end on its last line.* **Why:** the `end` line is missing. **Fix:** add it as the last line (LustScript only).
- **Problem:** nothing happens when the script runs. **Why:** nothing was selected, or the selected shapes have no hard edges. **Fix:** select a cube, or mark some edges hard (**Mesh → Shading**).
- **Problem:** *a script is running on this project* when you try to undo. **Why:** a script is still working. **Fix:** wait for it, or press **Esc** to stop it.
- **Tip:** copy a script that works before changing it a lot: right-click it → **Show the folder**, and copy the file.
- **Tip:** prefer your own text editor? Right-click the script → **Open in your own editor**. Save there and the app loads the change.

## Related

- [Scripting](scripts.md): everything scripts can do, the Scripts panel in full, limits and mistakes.
- [Script reference](script-reference.md): every call, in LustScript and Lua.
- [Bevel](bevel.md): the tool the script's bevel is.
- [Concepts](concepts.md#scripting): script, LustScript, Lua, console and the other words.
