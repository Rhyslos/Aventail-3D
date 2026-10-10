# Interface tour

> The app's window: what each area is for, how to rearrange it, and how to change its accent colour.

![The app window with the standard layout](../screenshots/2026-10-10-studio-look.png)

## When to use this page

Start here if you're opening the app for the first time, or if you've moved panels around and want to know how to get the standard layout back.

## The areas of the window

From top to bottom, left to right:

1. **Menu bar.** Every command lives in a menu, in this order: File, Edit, Create, Modes, Select, Tools, Mesh, UV, View, Window, Help. Commands that aren't built yet are greyed out; hover over one to see which phase brings it. The **Mesh** menu's **Make editable** already works: it turns the selected shapes into plain meshes (see [Objects](objects.md#make-editable)). On the right are the **layouts**: arrangements of the window for different kinds of work. *Standard* is the one in use (underlined); *Modelling* and *UV Edit* are greyed out until they arrive.
2. **Command palette** (the row of icons under the menus). One-click access to the tools you use most: undo and redo, the selection and transform tools, axis locks, primitives, groups, snapping and symmetry. Undo, redo, the blue shape buttons (cube, cylinder, plane, sphere) and the group button work now (press **B** anywhere for a list of every shape; see [Primitives](primitives.md)); the others are dimmed, and hovering shows when each arrives.
3. **Mode palette** (the column of icons down the left side). Decides *what kind of thing* your tools work on; see [Modes](#modes) below. At the bottom are switches for the viewport: the grid button steps through three grid levels, from a line every metre (standard) to every 50 cm and every 10 cm, and back (filled with the accent colour while it shows more than the metre lines). See [Navigating the view](navigating-the-view.md#settings).
4. **Viewport** (the large area in the middle). The 3D view of your scene, with its own menu line along the top (View, Cameras, Display, Options, Panel) and buttons on the right of that line that you can drag to pan, zoom and orbit. The view shows a calm grey sky over a slightly darker ground, with the horizon where they meet. A grid lies on the ground around the point you're looking at, a line every metre, fading out at its edges. The red line is the X axis, the blue one the Y axis, and the green line rising from the middle is Z, which points up. It can also show the top, front and right views, one at a time or all four at once (press **F5**). Labels in the corners tell you the view (*Perspective*, *Top*, ...), the camera, which way the axes point, and the grid spacing; the button in the bottom-right corner picks how the mouse moves the view. A new scene is empty; shapes you create appear at its centre, and selected ones are outlined in the accent colour. See [Navigating the view](navigating-the-view.md) and [Primitives](primitives.md).
5. **Attributes** (top right), the Attribute Manager. Shows the settings of whatever is selected: its name and whether it's shown, and for a shape its size, segments and so on. Drag or type a value and the model updates as you go. Sizes show in metres or centimetres, as you choose; the small arrows beside each value nudge it one step for fine tuning. Its menu line has Mode, Edit (copy and paste settings) and Units. See [The Attribute Manager](attributes.md).
6. **Objects** (below Attributes), the Object Manager. A tree of every object in your scene: click to select, double-click to rename, drag to put objects inside each other (see [Hierarchy](concepts.md#objects-and-scenes)), and click the eye to show or hide. Its menu line has Edit, View and Object menus. See [Objects and the Object Manager](objects.md).
7. **Coordinates** (under the viewport). Type exact positions, sizes and rotations for the selection, then press Apply (sizes and positions are in the units chosen in the Attributes panel, metres as standard); **Object (Rel)** above Apply will switch between values relative to the object and to the world. In this app **Z is up**: X and Y are the ground directions and Z is height (see [Axes](concepts.md#objects-and-scenes)). Rotation uses H, P and B: **heading** turns left and right around Z, **pitch** tilts up and down, **bank** rolls sideways. The panel is just tall enough for its rows; drag the gap above it to make it bigger or smaller.
8. **Status bar** (the bottom line). Says which mode you're in and how many objects are selected, reminds you which navigation controls are in use, and shows short messages, for example what was just undone or why something couldn't be done.

## Modes

The mode palette sets what you're working on. The button filled with the accent colour is the active mode.

| Mode | You work on |
|---|---|
| Model | Whole objects, with everything inside them |
| Object | Whole objects, including moving their axis (the [pivot](concepts.md#objects-and-scenes)) without moving the object |
| Points | The [vertices](concepts.md#mesh-basics) of a mesh |
| Edges | The edges of a mesh |
| Polygons | The faces of a mesh |
| UV | The [UV map](concepts.md#uvs-and-texturing), in the UV Editor and on the model |

You can also switch modes from the **Modes** menu. Until meshes exist (Phase 3), switching modes only changes the status bar.

## Rearranging panels

Attributes, Objects and Coordinates are **panels** you can rearrange around the viewport:

- **Move a panel:** drag it by its tab (the name at its top-left) and drop it where the highlight shows. Dropping it onto another panel's tab area stacks them as tabs.
- **Resize:** drag the gap between two panels, including the gaps around the viewport. A panel can be made as small as its tab and a little room.
- **Get the standard layout back:** **View → Reset panel layout** on the main menu bar.

The viewport has no tab (its menu line takes that place), so it stays where it is while the others move around it. Panels can't be closed or pulled out into separate windows, so you can't lose one by accident. Your arrangement and the window size are remembered the next time you open the app.

## Accent colour

One colour marks what's active: the current mode (and later the current tool), the chosen option in a list, the layout in use, and selected text. It's red unless you pick another: **Window → Accent colour** offers Red, Blue, Marine green, Plum, Olive and Petrol blue. The whole app changes at once, and your choice is remembered.

## Shortcuts

| Action | Shortcut |
|---|---|
| Quit | `Ctrl + Q` (`Cmd + Q` on macOS) |
| Undo / redo | `Ctrl + Z` / `Ctrl + Y` (or `Ctrl + Shift + Z`) |
| Delete the selection | `Delete` or `Backspace` |
| Frame everything / the selection | `H` / `F` |
| Perspective, top, right, front view; all four | `F1`, `F2`, `F3`, `F4`; `F5` |
| Orbit, pan, zoom the view | Depends on your navigation controls; as standard right drag to orbit, middle drag to pan and the mouse wheel to zoom (`Alt` + left, middle or right drag work too). See [Navigating the view](navigating-the-view.md) |

## Tips and common problems

- **Colours:** the app is always dark, whatever your system prefers. The accent colour marks what's active. Palette icons are soft white, except the shapes you can create (cube, cylinder, plane, sphere), which are blue. The axes are always X red, Y blue, Z green, in the viewport, the palette's axis locks and next to the Coordinates fields; their letters are always shown too, so colour is never the only clue. Slightly dimmed icons are commands that aren't built yet.
- **Problem:** the panels are in a strange arrangement after an update. **Why:** a saved layout from an older version didn't fit. **Fix:** the app replaces it with the standard layout automatically and says so in the status bar; if it still looks wrong, use **View → Reset panel layout**.
- **Problem:** the viewport shows a tab after you dropped another panel onto it. **Why:** when two panels share the space, both keep their tabs so neither gets lost. **Fix:** drag the other panel back out, or use **View → Reset panel layout**.
- **Problem:** on Linux, the app says a library "could not be loaded". **Fix:** install the library it names with your package manager (on Ubuntu, for example, `libxkbcommon-x11-0` and `mesa-vulkan-drivers`).

## Related

- [Navigating the view](navigating-the-view.md): orbit, pan and zoom, the navigation controls, and the grid settings.
- [Objects and the Object Manager](objects.md): selecting, naming, grouping, hiding, deleting and undo.
- [Concepts](concepts.md): the ideas behind the tools.
