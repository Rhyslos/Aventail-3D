# Navigating the view

> Turn around, slide across and move closer to your model in the 3D viewport, with Alt and the mouse, as in Cinema 4D.

## When to use it

All the time: looking at a model from different sides is how you judge its shape. Moving the view never changes the model itself, so experiment freely. If you get lost, **View → Reset view** brings back the opening view.

## How to use it

The camera always looks at one point, the **pivot** (see [Orbit, pan and zoom](concepts.md#viewing-the-scene)). Orbiting turns around it, zooming moves towards it, and panning carries it along with the view.

1. **Orbit (turn around the model):** hold **Alt**, press the **left** mouse button in the viewport and drag. The model follows your hand: drag right and its front turns to the right; drag down and its top tips towards you. The view never flips upside down: it stops just short of looking straight down or straight up.
2. **Pan (slide the view):** hold **Alt** and drag with the **middle** mouse button (press the wheel down). The grid stays under the mouse as you drag, as if you were pushing a sheet of paper.
3. **Zoom (move closer or farther):** turn the **mouse wheel** (away from you to move closer), or hold **Alt** and drag with the **right** button (right or up to move closer, left or down to move away). Each step moves the same share of the distance, so zooming feels the same whether you are close or far.

You only need to hold Alt when the drag starts; letting go of it during the drag doesn't stop the movement. The mouse pointer changes shape while you navigate. The status bar at the bottom of the window shows these controls as a reminder.

**Coming next:** orbiting and zooming around the exact point under the mouse (as in 3ds Max), framing a selected object, and top, front and side views.

## Settings

| Setting | What it does |
|---|---|
| **Fine grid lines** (the grid button at the bottom of the left palette, or **View → Fine grid lines**) | Off (the standard): the grid shows a line every metre, plus stronger 10 m lines in the distance, for a calm ground. On: adds thin lines every 10 cm, useful when you're placing things precisely up close. The button is orange while the fine lines are on. Your choice is remembered next time you open the app. |
| **View → Reset view** | Puts the camera back where it starts when the app opens. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Orbit | `Alt` + left drag |
| Pan | `Alt` + middle drag |
| Zoom | Mouse wheel, or `Alt` + right drag |

On macOS, Alt is the `Option` key.

## Tips and common problems

- **Problem:** nothing happens when you drag in the viewport. **Why:** without Alt, dragging is kept free for the selection tools (Phase 3). **Fix:** hold Alt as you start the drag.
- **Problem:** on Linux, Alt + drag moves the whole window instead of the view. **Why:** some Linux desktops use Alt + drag for moving windows. **Fix:** in your desktop's window settings, change that key (often called the "window action key" or "modifier key") to Super, the Windows-logo key, or switch it off.
- **Problem:** zooming has stopped getting closer. **Why:** the camera stops 2 cm from the pivot, so it can't pass through it. **Fix:** zoom out a little and pan, or orbit, to come at it from another side. Zooming towards the point under the mouse arrives in the next update.
- **Problem:** the grid looks busy or flickers in the distance. **Fix:** switch the fine grid lines off; the remaining lines fade out on their own when they would get too close together.

## Related

- [Interface tour](interface-tour.md): the viewport and the rest of the window.
- [Concepts](concepts.md): [orbit, pan and zoom](concepts.md#viewing-the-scene), [axes](concepts.md#objects-and-scenes), and the [origin and grid](concepts.md#objects-and-scenes).
