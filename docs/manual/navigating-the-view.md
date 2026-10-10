# Navigating the view

> Turn around, slide across and move closer to your model in the 3D viewport, with the mouse controls that suit you.

## When to use it

All the time: looking at a model from different sides is how you judge its shape. Moving the view never changes the model itself, so experiment freely. If you get lost, **View → Reset view** (in the viewport's own menu line) brings back the opening view.

## How to use it

Every movement happens around one point, the **pivot** (see [Orbit, pan and zoom](concepts.md#viewing-the-scene)): orbiting turns around it, zooming moves towards it, and panning carries it along with the view. The grid on the ground is drawn around it too, so it follows you.

**The pivot is whatever you point at.** When you start to orbit, the view turns around the exact spot on the model under the mouse, and that spot stays still on screen while everything turns around it; a small ring marks it while you drag. When you zoom, the view moves towards whatever is under the mouse, which stays under the mouse as you get closer: point at a corner and roll the wheel to get a close look at that corner. Over empty background, orbiting keeps turning around the last pivot, and zooming heads for the ground under the mouse (or, at the sky, straight into the view). Rolling the wheel while you hold a button to orbit or pan zooms towards the point you're already turning around, so nothing jumps. Prefer turning around the middle of the view? Switch off **Orbit and zoom around the point under the mouse** (see [Settings](#settings)).

As standard, the view moves with **the mouse alone**, no keys needed:

1. **Orbit (turn around the model):** press the **right** mouse button in the viewport and drag. The model follows your hand: drag right and its front turns to the right; drag down and its top tips towards you. It turns around the spot you grabbed. The view never flips upside down: it stops just short of looking straight down or straight up.
2. **Pan (slide the view):** drag with the **middle** mouse button (press the wheel down). The grid stays under the mouse as you drag, as if you were pushing a sheet of paper.
3. **Zoom (move closer or farther):** turn the **mouse wheel** (away from you to move closer). Each step moves the same share of the distance, so zooming feels the same whether you are close or far.

The Alt + mouse moves work too, unless you switch them off (see [Settings](#settings)): **Alt** + left drag orbits, **Alt** + middle drag pans, and **Alt** + right drag zooms (right or up to move closer). You only need a key when the drag starts; letting go of it during the drag doesn't stop the movement. The mouse pointer changes shape while you navigate. A right *click* without dragging is left free for menus.

**Without the mouse buttons:** the three buttons on the right of the viewport's menu line (four arrows, an up-and-down arrow, a circular arrow) pan, zoom and orbit when you drag them. Handy on a laptop touchpad.

**Coming next:** framing a selected object or the whole scene, and top, front and side views.

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

Whatever you pick, **a plain left drag never moves the view**: it's kept free for selecting and for the modelling tools (from Phase 3). A right *click* without dragging stays free for menus too.

## Settings

| Setting | What it does |
|---|---|
| **Orbit and zoom around the point under the mouse** (in the controls list, or **Options**; on by standard) | Orbiting turns around, and zooming moves towards, whatever is under the mouse. Off: they always work around the middle of the view instead. |
| **Alt + mouse works in every scheme** (in the controls list, on by standard) | Keeps the Alt + mouse moves working whichever controls you pick, unless those controls use the same keys for something else. Useful while you get used to new controls. |
| **Invert orbit direction** (in the controls list, or **Options**) | Orbiting turns the other way: the model moves against your hand instead of with it. |
| **Fine grid lines** (the grid button at the bottom of the left palette, **Display → Fine grid lines**, or **View → Fine grid lines** on the main menu bar) | Off (standard): a line every metre, plus stronger 10 m lines further out, for a calm ground. On: adds thin lines every 10 cm, useful when placing things precisely up close. The bottom-left corner of the view shows the spacing. Remembered next time. |
| **View → Reset view** | Puts the camera back where it starts when the app opens. |

## Shortcuts

As standard (Mouse only controls, with Alt + mouse switched on):

| Action | Shortcut |
|---|---|
| Orbit | Right drag, or `Alt` + left drag |
| Pan | Middle drag, or `Alt` + middle drag |
| Zoom | Mouse wheel, or `Alt` + right drag |

On macOS, Alt is the `Option` key. The other controls are in the table above.

## Tips and common problems

- **Problem:** nothing happens when you drag in the viewport. **Why:** a plain left drag is kept free for the selection tools, and some controls need a key. **Fix:** check the controls in use in the bottom-right corner (or the status bar), and hold the key it lists as you start the drag.
- **Problem:** on Linux, Alt + drag moves the whole window instead of the view. **Why:** some Linux desktops use Alt + drag for moving windows. **Fix:** use the standard **Mouse only** controls (right drag orbits, middle drag pans), or in your desktop's window settings change that key (often called the "window action key" or "modifier key") to Super, the Windows-logo key.
- **Problem:** zooming has stopped getting closer. **Why:** the camera stops 2 cm from the pivot, so it can't pass through it. **Fix:** point the mouse at something a little further away and zoom again: the new point becomes the pivot. Or zoom out and come at it from another side.
- **Problem:** the grid has disappeared. **Why:** the grid is drawn around the point you're looking at (at least 18 m around it) and fades out at its edges; if you've panned far away or are looking at the sky, there may be none in view. **Fix:** **View → Reset view**, or zoom out: the grid reaches farther the farther away you are.

## Related

- [Interface tour](interface-tour.md): the viewport and the rest of the window.
- [Concepts](concepts.md): [orbit, pan and zoom](concepts.md#viewing-the-scene), [axes](concepts.md#objects-and-scenes), and the [origin and grid](concepts.md#objects-and-scenes).
