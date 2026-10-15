# Navigating the view

> Turn around, slide across and move closer to your model in the 3D viewport, with the mouse controls that suit you, and look at it from the top, front and side.

## When to use it

All the time: looking at a model from different sides is how you judge its shape. Moving the view never changes the model itself, so experiment freely. If you get lost, press **H** to fit everything back in view, or use **View → Reset view** (in the viewport's own menu line) to go back to the opening view.

## How to use it

Every movement happens around one point, the **pivot** (see [Orbit, pan and zoom](concepts.md#viewing-the-scene)): orbiting turns around it, zooming moves towards it, and panning carries it along with the view. The grid on the ground is drawn around it too, so it follows you.

**The pivot is whatever you point at.** When you start to orbit, the view turns around the exact spot on the model under the mouse, and that spot stays still on screen while everything turns around it; a small ring marks it while you drag. (With this switched off, the view turns around the middle and no ring is shown.) When you zoom, the view moves towards whatever is under the mouse, which stays under the mouse as you get closer: point at a corner and roll the wheel to get a close look at that corner. Over empty background, orbiting keeps turning around the last pivot, and zooming heads for the ground under the mouse (or, at the sky, straight into the view). Rolling the wheel while you hold a button to orbit or pan zooms towards the point you're already turning around, so nothing jumps. Prefer turning around the middle of the view? Switch off **Orbit and zoom around the point under the mouse** (see [Settings](#settings)).

As standard, the view moves with **the mouse alone**, no keys needed:

1. **Orbit (turn around the model):** press the **right** mouse button in the viewport and drag. The model follows your hand: drag right and its front turns to the right; drag down and its top tips towards you. It turns around the spot you grabbed. The view never flips upside down: it stops just short of looking straight down or straight up.
2. **Pan (slide the view):** drag with the **middle** mouse button (press the wheel down). The grid stays under the mouse as you drag, as if you were pushing a sheet of paper.
3. **Zoom (move closer or farther):** turn the **mouse wheel** (away from you to move closer). Each step moves the same share of the distance, so zooming feels the same whether you are close or far.

The Alt + mouse moves work too, unless you switch them off (see [Settings](#settings)): **Alt** + left drag orbits, **Alt** + middle drag pans, and **Alt** + right drag zooms (right or up to move closer). You only need a key when the drag starts; letting go of it during the drag doesn't stop the movement. The mouse pointer changes shape while you navigate. A right *click* without dragging is left free for menus.

**Without the mouse buttons:** the three buttons on the right of the viewport's menu line (four arrows, an up-and-down arrow, a circular arrow) pan, zoom and orbit when you drag them. Handy on a laptop touchpad.

**Framing: fit things in the view.** Press **H** (*frame all*) to fit the whole scene in the view, or **Z** (*frame selected*) to fit what you've selected. The view keeps looking the same way and glides forwards or back until everything fits, with a little room around it, so you never lose your bearings. Both are also in the **View** menus. With nothing selected, Z frames everything too. (Frame selected used to be **F**; F is [Loop cut](loop-cut.md) now.) Hidden objects are left out; with nothing shown at all, both go back to the middle of the scene. The keys work anywhere in the app, except while you're typing in a field.

## Look along an axis

The small **X**, **Y** and **Z** arrows in the bottom-left corner of a view show which way the [axes](concepts.md#objects-and-scenes) point. In the perspective view, their round letters are buttons:

- **Click a letter** and the view swings round to look straight along that axis, towards the point you're turning around, from that axis's side, at the same distance. **Z** looks straight down from above, **X** looks from the right (the +X side), and **Y** from the back (the +Y side).
- **Click the same letter again** to look from the other side: Z again looks up from below.
- Looking straight down or up, the view lines up like the Top view: X to the right, Y up the screen.

It's a quick way to check a shape square-on without switching to a flat view. You can orbit away again at once. Pointing at a letter rings it, so you know what you'll click. The letters work in flat views too, and the view stays flat: from the Top view, clicking X turns it into the Right view.

![After clicking Z: the view looks straight down at a crate, with X to the right and Y up.](../screenshots/2026-10-14-look-along.png)

## Views and the four-view layout

The viewport can show four views: **Perspective** (the ordinary 3D view), **Top**, **Front** and **Right**. Top, front and right are [flat views](concepts.md#viewing-the-scene): no perspective, so sizes and lines read true, which makes them handy for lining things up.

![The four views side by side](../screenshots/2026-10-10-four-views.png)

- **See all four at once:** press **F5**, click the square button at the right end of the viewport's menu line, or use **Panel → Four views**. Perspective sits top left, Top top right, Front bottom left and Right bottom right.
- **Show one view large:** press **F1** (perspective), **F2** (top), **F3** (right) or **F4** (front), or click the square button again (it shows the active view on its own).
- **Middle click to switch:** a middle click (press the wheel without moving the mouse) on a view shows just that view; another middle click goes back to all four. Dragging with the middle button still pans as usual.
- **The active view:** the last view you clicked is the active one, marked with a thin line in the accent colour. The viewport's menus (View, Cameras) and keys such as **H** act on it.
- **Change what a view shows:** **Cameras** lists the kinds (Perspective, Orthographic, Top, Front, Right); picking one turns the active view into that kind, starting from its opening camera. Between Perspective and Orthographic the view keeps looking where it looked, as **P** does.

Each view keeps its own camera, so moving around in one never disturbs the others. In a flat view you can pan and zoom as usual, and **orbiting turns it into a free orthographic view**: the Top view becomes *Orthographic*, still flat, from wherever you turn it. Press **F2** (or F3, F4) to get the exact Top (or Right, Front) view back. Zooming a flat view shows more or less of the scene (the spot under the mouse stays put), and **H** fits everything in it. Each flat view's grid stands facing you: the top view shows the ground grid from above, the front view an upright grid along X and Z, and the right view one along Y and Z. The view's name is in its top-left corner. Whether you last used one view or four is remembered.

### Orthographic: no perspective, from any direction

Press **P** to switch the active view between **perspective** and **orthographic** (or pick **Cameras → Orthographic**). An orthographic view is [flat](concepts.md#viewing-the-scene): parallel lines stay parallel and nothing shrinks with distance, so you can judge whether edges line up, whether parts are the same height, and how a shape reads as a game sprite or an isometric asset. Unlike Top, Front and Right, it can be orbited freely, panned and zoomed like the perspective view.

The switch keeps the view looking the same way, at the same point, with things at that point the same size, so nothing jumps. Press **P** again to go back. The view's name in its top-left corner says *Orthographic*. In the Top view, P gives a perspective view looking straight down.

![An orthographic view of a cube, orbited: its parallel edges stay parallel, and the grid lines don't converge](../screenshots/2026-10-15-orthographic.png)

## Navigation controls

Different people come with different habits, so you can choose how the mouse moves the view. Click the button in the **bottom-right corner of the viewport** (it shows the name of the controls in use), or use **Options → Navigation controls…** in the viewport's menu line, and pick one:

| Controls | Orbit | Pan | Zoom |
|---|---|---|---|
| **Mouse only** (standard) | right drag | middle drag | wheel |
| **Alt + mouse** | `Alt` + left drag | `Alt` + middle drag | `Alt` + right drag, or wheel |
| **Mouse only, middle zooms** | right drag | `Shift` + middle drag | middle drag, or wheel |
| **Middle pans, Alt orbits** | `Alt` + middle drag | middle drag | wheel |
| **Middle orbits** | middle drag | `Shift` + middle drag | wheel |

Try them for a while and keep the one that feels natural. The status bar shows the controls in use as a reminder, and your choice is remembered.

Whatever you pick, **a plain left drag never moves the view**: it's kept free for selecting and for the gizmos that move, rotate and scale. A right *click* without dragging stays free for menus too.

## Settings

Every setting here is also in **Edit → Settings… → Viewport** (see [Settings](settings.md)), next to the app's other preferences.

| Setting | What it does |
|---|---|
| **Orbit and zoom around the point under the mouse** (in the controls list, or **Options**; on by standard) | Orbiting turns around, and zooming moves towards, whatever is under the mouse. Off: they work around the middle of the view, exactly as the app's original rotation did. |
| **Alt + mouse works in every scheme** (in the controls list, on by standard) | Keeps the Alt + mouse moves working whichever controls you pick, unless those controls use the same keys for something else. Useful while you get used to new controls. |
| **Invert orbit direction** (in the controls list, or **Options**) | Orbiting turns the other way: the model moves against your hand instead of with it. |
| **Grid lines** (the grid button at the bottom of the left palette, **Display → Grid lines**, or **View → Grid lines** on the main menu bar) | Three levels. **Every 1 m** (standard): a line every metre, plus stronger 10 m lines further out, for a calm ground. **Every 50 cm (quarters)**: each metre square is cut into four by fainter lines, handy for halves and quarters of a metre. **Every 10 cm (fine)**: thin lines every 10 cm, for placing things precisely up close. Each click on the grid button moves to the next level, and from the finest back to the calmest; the menus let you pick one directly. The bottom-left corner of the view shows the spacing, above the axis arrows. Remembered next time. |
| **Display → Hard edges**, **Display → Normals** | Mark hard edges in light cyan while editing points, edges or polygons (on by standard), and show which way each polygon of the selected objects faces. See [Hard and soft edges](hard-and-soft-edges.md#seeing-them). Remembered next time. |
| **View → Frame all** (`H`), **Frame selected** (`Z`) | Fit everything shown, or the selected objects, in the view, keeping the direction you're looking in. |
| **View → Reset view** | Puts the active view's camera back where it starts when the app opens. |
| **Cameras → Perspective, Top, Front, Right** | Turns the active view into that kind of view. |
| **Panel → Four views** (`F5`) | Shows all four views at once; again for just the active one. Remembered next time. |

## Shortcuts

As standard (Mouse only controls, with Alt + mouse switched on):

| Action | Shortcut |
|---|---|
| Orbit | Right drag, or `Alt` + left drag |
| Pan | Middle drag, or `Alt` + middle drag |
| Zoom | Mouse wheel, or `Alt` + right drag |
| Frame all | `H` |
| Frame selected | `Z` |
| Perspective, top, right, front view on its own | `F1`, `F2`, `F3`, `F4` |
| Switch between perspective and orthographic | `P` |
| All four views | `F5` |
| Switch between one view and four | Middle click on a view |
| Look along X, Y or Z (again: from the other side) | Click its letter in the bottom-left corner (perspective view) |

On macOS, Alt is the `Option` key. The other controls are in the table above.

## Tips and common problems

- **Problem:** nothing happens when you drag in the viewport. **Why:** a plain left drag is kept free for the selection tools, and some controls need a key. **Fix:** check the controls in use in the bottom-right corner (or the status bar), and hold the key it lists as you start the drag.
- **Problem:** on Linux, Alt + drag moves the whole window instead of the view. **Why:** some Linux desktops use Alt + drag for moving windows. **Fix:** use the standard **Mouse only** controls (right drag orbits, middle drag pans), or in your desktop's window settings change that key (often called the "window action key" or "modifier key") to Super, the Windows-logo key.
- **Problem:** zooming has stopped getting closer. **Why:** the camera stops 2 cm from the pivot, so it can't pass through it. **Fix:** point the mouse at something a little further away and zoom again: the new point becomes the pivot. Or zoom out and come at it from another side.
- **Problem:** I orbited the Top view and now it says *Orthographic*. **Why:** orbiting a flat view frees it, as in Blender. **Fix:** press **F2** for the exact Top view again (F3 Right, F4 Front).
- **Problem:** far things don't look smaller, and the view feels odd. **Why:** the view is orthographic (its name says so). **Fix:** press **P** for perspective.
- **Problem:** a function key does something else, like changing the screen brightness. **Why:** on many laptops the F keys control the hardware unless you hold **Fn**. **Fix:** hold **Fn** with the key, or use the square button on the viewport's menu line, a middle click, or the **Panel** and **Cameras** menus.
- **Problem:** the grid has disappeared. **Why:** the grid is drawn around the point you're looking at (at least 18 m around it) and fades out at its edges; if you've panned far away or are looking at the sky, there may be none in view. **Fix:** press **H** to frame everything, or zoom out: the grid reaches farther the farther away you are.

## Related

- [Interface tour](interface-tour.md): the viewport and the rest of the window.
- [Concepts](concepts.md): [orbit, pan and zoom](concepts.md#viewing-the-scene), [axes](concepts.md#objects-and-scenes), and the [origin and grid](concepts.md#objects-and-scenes).
