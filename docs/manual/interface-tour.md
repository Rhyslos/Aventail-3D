# Interface tour

> The app's window: what each area is for, and how to rearrange it.

![The app window with the standard layout](../screenshots/2026-10-10-navigation.png)

## When to use this page

Start here if you're opening the app for the first time, or if you've moved panels around and want to know how to get the standard layout back.

## The areas of the window

From top to bottom, left to right:

1. **Menu bar.** Every command lives in a menu, in this order: File, Edit, Create, Modes, Select, Tools, Mesh, UV, View, Window, Help. Commands that aren't built yet are greyed out; hover over one to see which phase brings it.
2. **Command palette** (the row of icons under the menus). One-click access to the tools you use most: undo and redo, the selection and transform tools, axis locks, primitives, snapping and symmetry. For now all of them are dimmed; hover to see when each arrives.
3. **Mode palette** (the column of icons down the left side). Decides *what kind of thing* your tools work on; see [Modes](#modes) below. At the bottom are switches for the viewport: the grid button turns the fine 10 cm grid lines on and off (orange while on).
4. **Viewport** (the large area in the middle). The 3D view of your scene. The grid lies on the ground, with a line every metre (switch on the fine 10 cm lines with the grid button at the bottom of the mode palette). The red line is the X axis and the blue one the Y axis; Z points up. Shapes are lit softly from above and in front, a little darker where they meet the ground, so you can read their form without harsh contrast. For now it shows a 2 m test cube standing at the origin. Hold **Alt** and drag to look around it: left button to orbit, middle to pan, right (or the mouse wheel) to zoom. See [Navigating the view](navigating-the-view.md).
5. **Attributes** (top right). Shows the settings of whatever is selected, or of the tool you're using. Change a value here and the model updates.
6. **Objects** (below Attributes). A tree of every object in your scene. Objects can be grouped by putting them inside each other (see [Hierarchy](concepts.md#objects-and-scenes)).
7. **Coordinates** (under the viewport). Type exact positions, sizes and rotations for the selection, then press Apply; **Object (Rel)** above Apply will switch between values relative to the object and to the world. In this app **Z is up**: X and Y are the ground directions and Z is height (see [Axes](concepts.md#objects-and-scenes)). Rotation uses H, P and B: **heading** turns left and right around Z, **pitch** tilts up and down, **bank** rolls sideways. The panel is just tall enough for its rows; drag the gap above it to make it bigger or smaller.
8. **Status bar** (the bottom line). Says which mode you're in, reminds you how to move around the 3D view, and shows short messages, for example after you reset the layout.

## Modes

The mode palette sets what you're working on. The orange button is the active mode.

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

The four middle areas (Viewport, Attributes, Objects, Coordinates) are **panels** you can rearrange:

- **Move a panel:** drag it by its tab (the label at its top-left) and drop it where the highlight shows. Dropping it onto another panel's tab area stacks them as tabs.
- **Resize:** drag the gap between two panels. A panel can be made as small as its tab bar and a little room.
- **Get the standard layout back:** **View → Reset panel layout**.

Panels can't be closed or pulled out into separate windows, so you can't lose one by accident. Your arrangement and the window size are remembered the next time you open the app.

## Shortcuts

| Action | Shortcut |
|---|---|
| Quit | `Ctrl + Q` (`Cmd + Q` on macOS) |
| Orbit, pan, zoom the view | `Alt` + left, middle or right drag; mouse wheel to zoom |

## Tips and common problems

- **Colours:** the app is always dark, whatever your system prefers. Orange marks what's active (the current mode, and later the current tool); blue-grey marks what's selected. Palette icons are soft white, except the shapes you can create (cube, cylinder, plane, sphere), which are blue. The axes are always X red, Y blue, Z green, in the palette's axis locks and next to the Coordinates fields. Slightly dimmed icons are commands that aren't built yet.
- **Problem:** the panels are in a strange arrangement after an update. **Why:** a saved layout from an older version didn't fit. **Fix:** the app replaces it with the standard layout automatically and says so in the status bar; if it still looks wrong, use **View → Reset panel layout**.
- **Problem:** on Linux, the app says a library "could not be loaded". **Fix:** install the library it names with your package manager (on Ubuntu, for example, `libxkbcommon-x11-0` and `mesa-vulkan-drivers`).

## Related

- [Navigating the view](navigating-the-view.md): orbit, pan and zoom, and the grid settings.
- [Concepts](concepts.md): the ideas behind the tools.
