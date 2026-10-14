# Changelog

Every change a user would notice, newest first, in plain words.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **The Keys window draws your keyboard.** Every key the app uses is lit, with coloured marks for where it works (anywhere, Model mode, Points/Edges/Polygons, while dragging, menus). Point at a key to see its jobs, click it to keep them shown, and click the coloured chips to hide a situation. The search outlines matching keys. The keyboard follows your system language (US, UK, Norwegian, Danish, Swedish / Finnish, German, French), and you can pick another from the **Keyboard** list.
- **Symmetry for every edit.** With Symmetry on, extrude, inset, bevel, loop cut, slit, connect, bridge, weld, delete, dissolve, fill hole, flip and hard or soft edges are made on the other side too, mirrored, as one undo step; the Last edit changes both sides. Projects move to file format 14.
- **Symmetry menu.** A small arrow beside the Symmetry button opens its settings: X, Y and Z (moved here from the Attributes panel), where the mirror goes through (the object's pivot, its middle, or the world centre) and how far it is moved from there. **Weld the middle now** joins the halves once, when clicked (Ctrl + Z undoes it): points near the middle go onto it and points meeting there become one. **Don't cross the middle** stops points at the middle.
- **Closed slits.** With Slit (X), click three or more points and then the first point again: the line closes into a loop. Drawn inside a polygon, the loop cuts its shape into it as a polygon of its own, like a window in a wall, ready to extrude or delete for a hole. Clicking a line point without moving the mouse no longer nudges it. Lines that touch a polygon's edge from inside and carry on now keep cutting.
- **Tab steps back.** Press **Tab** anywhere (not while typing) to go straight back to the Model mode: a Slit line is finished and a modelling tool put away, so a click selects objects again.
- **Fill hole has a key: J.** Select a hole's rim and press J. To cap both ends of an open tube, select both rims and press J (C still bridges them with a tube).
- **Dissolve asks before making a bent polygon.** Dissolving where polygons meet at a sharp corner (over 30°) asks first, with an "I know what I'm doing, proceed" tickbox and a "Don't warn me again" choice; switch the question back on in Settings → Interface → Warnings.
- **Weld snaps.** While dragging a point with the Weld tool (Ctrl + M), the line snaps onto the point under the mouse and rings it, so you see where it will go. Bridge's drag snaps onto edges the same way.
- **Keyboard shortcuts window.** Click **Keys** on the menu bar, after Help, for every key and mouse action in one place, grouped, with a search field.
- **Slide.** In the Points, Edges and Polygons modes, hold **Alt** and drag the Move gizmo: the points slide along their edges, staying on the surface. Handy for moving a loop cut after making it.
- **Split off and join.** **Mesh → Split off** moves the selected polygons into a new object of their own; **Join objects** (E menu in the Model mode, or the Mesh menu) makes the selected shapes one.
- **Hide and isolate.** **Shift + H** hides the selection, **Ctrl + H** hides everything else, and **Alt + H** shows everything again: whole objects in the Model mode, polygons in the Points, Edges and Polygons modes (to see inside a shape). The left palette's **Show only the selection** button works now.
- **Copies and instances.** **Ctrl + C** then **Ctrl + V** pastes copies of the selected objects; **Alt + V** pastes instances, which show their original's shape and change with it. **Duplicate** and **Instance** are in the E menu and the Edit menu too.
- **Symmetry.** Click the **Symmetry** button at the right end of the top palette (or Tools → Symmetry) and choose **X**, **Y** or **Z** in the Attributes panel: moving, rotating or scaling points, edges or polygons now moves their mirror on the other side of the shape too, and points on the middle stay there.
- **Hard and soft edges.** Pick **Hard edges** or **Soft edges** from the E menu or the Mesh menu to make the selected edges (or the selected polygons' edges) crisp or smooth. **Mesh → Hard edges by angle** does a whole object at once, with an **Angle** you can change afterwards, and **Select → Select hard edges** selects them. The viewport's **Display** menu marks hard edges in light cyan (on as standard) and can show **Normals**: which way each polygon faces.
- **Clean-up.** In the Points, Edges and Polygons modes, **Delete** deletes the selected points, edges or polygons (leaving a hole) and **Backspace** dissolves them: they go, but the polygons around them merge so the surface stays closed. **Fill hole** closes holes with a polygon, and **Flip normals** turns polygons inside out (both in the E menu and the Mesh menu). **Mesh → Fix facing** turns every polygon facing the wrong way round, and the Select menu gains **Select flipped** and **Select similar facing**.
- **Connect, Bridge and Weld.** **J** joins the selected points (or the middles of the selected edges) across their polygons. **C** bridges two selected open borders or two polygons with a band of new polygons (Segments and Twist in the Attributes panel); with nothing to bridge selected, C lets you drag from one open edge to another. **M** welds the selected points into one at their middle, and **Ctrl + M** lets you drag a point onto another to weld it there.
- **Pipe bends.** An extrude can turn a corner and keep its full width: after extruding (D), set **Angle**, **Direction**, **Bend radius** and **Segments** under Last edit (or before dragging) for a smooth elbow, or a sharp mitre joint with no radius. Ctrl + wheel changes Segments while Extrude is in use.
- **Slit.** In Points, Edges or Polygons mode, press **X** and click points over a shape: it is cut along the line as you go, across any polygons, turning wherever a point sits inside one. Drag a point to move it, click the line to add one, Ctrl + click to remove one, and hold Shift for 15° steps. Esc or Enter finishes the line, and Ctrl + Z takes it back. As standard only what you can see is cut; **Cut through** in the Attributes panel cuts the far side too.
- **Ctrl + mouse wheel changes a tool's count**: Bevel's Segments and Loop cut's Cuts, before, while dragging, and on the edit you just made. With every other tool it still grows and shrinks the selection.
- **Loop cut.** In Points, Edges or Polygons mode, press **F** (or pick Loop cut from the E menu or the Mesh menu) and point at an edge: the new loop shows all the way around the shape. Click to cut it, or press and drag along the edge to slide it into place first. Hold **Ctrl** and turn the mouse wheel (or set **Cuts** in the Attributes panel) to make several evenly spaced loops at once; afterwards the cut's Cuts and Position can still be changed under Last edit. The loop stops cleanly at triangles, caps and open borders.
- **Hollow bevels.** The bevel's Profile now goes from 100% down to -100%: 100% is round, 0% is flat, and below 0 the bevel curves inward instead, like a cove moulding.
- **Bevel.** In Edges mode, select edges, press **V** (or pick Bevel from the E menu or the Mesh menu) and drag away from them: the edges are cut away and replaced by a strip, flat (a chamfer) or rounded. In Points mode, V cuts corners off. **Segments** and **Profile** (round to flat) are set in the Attributes panel before the drag, and the last bevel's Width, Segments and Profile can be changed afterwards. Where bevelled edges meet, a corner polygon closes the gap.
- **Extrude and inset.** In Polygons mode, press **D** (Extrude) or **I** (Inset), or pick them from the E menu or the Mesh menu, and drag anywhere in the viewport. Extrude follows your drag: drag the way the polygons face to bring them out, the other way to push them in, and it builds walls behind them. Start the drag on another polygon to extrude that one without clicking it first. Inset follows your pull too: drag towards the middle of the polygons and they shrink. Inset shrinks them inward with a ring of new polygons around them. Shift gives fine steps, Esc or a right-click cancels, and each drag is one undo step. The polygons stay selected, so you can drag again to carry on.
- **Extrude border edges**: in Edges mode, D grows a flat strip of polygons from edges on an open border, like extending a plane; the selection moves to the new edge.
- **Keep together**: a switch in the Attributes panel decides whether touching polygons move as one piece or each on its own.
- **Change the last extrude or inset afterwards**: its Distance and Keep together switch show at the top of the Attributes panel until you make another edit.
- The Mesh menu's built tools can now be clicked.

- **The tool menu: press E.** It opens at the mouse with the tools for what you're working on: object tools (move, rotate, scale, make editable, group, delete and more) in Model mode, and the modelling tools while editing points, edges or polygons. Point towards a tool and click, tap **E** to step through and press **Enter** or **Space**, or hold **E**, point and let go. Every entry shows its key; tools still to come are dimmed and say when they arrive. The ring holds icons only and never changes size or moves its buttons between modes; the middle names the tool you point at, and a click there cancels. **Settings → Interface → Radial menu** offers four ring looks (Continuous ring, the standard; Inner edge; Outlined; Solid tiles) or a list, **Show keys around the ring**, and **Tap E to open**, which can be switched off so the menu shows only while E is held.
- **New icons for every tool**, designed with the project lead and the same everywhere: the tool menu, the palette and the menus. **Move, rotate and scale** (T) is now called **Transform**.
- The **Mesh** menu lists every modelling tool, with its key, and when it arrives. Extrude, inset and bevel will be **D**, **I** and **V**.
- **Backup copies and crash recovery.** While you work, the app saves a backup copy of your scene in the background every 5 minutes (only when something changed), keeping the newest 5. If the app ever closes without saving, a crash or a power cut, it asks **Recover unsaved work?** the next time it starts: **Restore** a copy, **Open as saved**, or **Discard**. The copies never replace your project file and are deleted when the app closes normally. Change how often, how many, or switch them off in **Settings → Autosave**.
- **Heavy shapes stay quick.** A shape of millions of polygons, like a detailed model someone might import, can now be selected, hovered over and edited without the window stalling. Work that takes a moment happens in the background, with a quiet note in the view ("Preparing a heavy shape for drawing…") or in the Attributes panel ("Checking the mesh…").
- **Help → Add a heavy test shape** adds a bumpy ball of 3 million polygons, to see how your computer copes. Undo takes it away again.
- **Edits follow a shape's settings.** Pull up a cube's top polygon, then give the cube more segments: the whole top stays lifted, because edits now remember where their points, edges and polygons were and find them again on the changed shape. The Attributes panel notes when that happened.
- **Scale along two axes at once**: the Scale gizmo has squares between its boxes, like Move. The direction you drag decides how much each axis scales: drag mostly along X and it scales mostly along X.
- **Move, rotate and scale.** Press **W**, **R** or **S** (or **T** for all three) with something selected, and drag the gizmo's arrows, squares, rings or boxes. It works on whole shapes in Model mode, and on points, edges and polygons in their modes, where the shape stretches to follow. Each drag is one undo step.
  - Hold **Shift** for ten times finer steps.
  - **Esc** or a right-click cancels a drag.
  - **X**, **Y** and **Z** lock axes, and **L** switches between the world's and the object's own axes. The palette buttons and the **Tools** menu do the same.
  - **Q** goes back to selecting.
- **Snapping**: the magnet button turns it on. Moves snap to points, edges, polygons or the grid lines shown, turns to 15° steps, and scaling to 10% steps. Holding **Ctrl** while dragging flips snapping for that drag; Settings can make it button only.
- **The Coordinates panel works**: it shows the selection's position, size and rotation. Type a value and press Enter, or drag the number.

- **Selecting in the viewport.** Click a shape to select it, **Ctrl + click** to add one (or take it out if it's selected), **Shift + click** to take one out, and click empty space to unselect. In Points, Edges and Polygons mode the same works on points, edges and polygons of the selected shapes. Every selection, including in the Objects panel, can be undone with **Ctrl + Z**.
- **Box, lasso and paint select brush**: drag to select (Ctrl + drag adds, Shift + drag removes), and press **Q** to switch between them. They select through the shape as standard, so the far side is caught too; the new **Select only visible** button on the left palette (also in the Select menu and Settings) limits them to what you can see.
- **One pattern of keys for every way of selecting**, with three others to choose from in **Settings → Viewport → Selection keys**: Shift adds and Ctrl removes; Ctrl flips; or Ctrl adds and Alt removes.
- **Loops and rings**: double-click for a loop, Alt + double-click for a ring. **Ctrl + mouse wheel** grows or shrinks the selection (with the brush, it changes the brush size). **Ctrl + A** selects all, **Ctrl + I** inverts, and **Esc** in Points, Edges or Polygons mode unselects the parts but keeps the shape. The **Select** menu has all of these.

- **Points, Edges and Polygons modes** now do something: press **1**, **2** or **3** (or use the Modes menu or the left palette) with a shape selected to see its edges, and in Points mode its points. The point, edge or polygon under the mouse lights up in the accent colour, and the status bar names it. Press the same key again to go back to Model mode. Selecting by clicking comes next.

### Changed

- **1, 2 and 3 put a tool away.** With a modelling tool in use (Extrude, Slit, Weld…), pressing a mode key (or clicking a mode button) puts the tool away and you're selecting in that mode, without leaving it. With no tool in use, the keys work as before.
- **Connect moved from J to A**, within easy reach of the left hand.
- **A click selects while Bridge or Weld is the tool**, with the usual red highlight under the mouse, and a drag from empty space draws a selection box. Only a drag that starts on an open edge (Bridge) or a point (Weld) is the tool's.
- **The bend Angle goes back to 0** once an extrude is done or another tool is picked, so the next extrude starts straight. The finished extrude keeps its bend under Last edit.
- **The bend settings are number fields with arrows**, like the rest of the Attributes panel, under a small Bend heading below Distance.
- **New corners come out crisp.** Where a Bridge band or a Fill hole cap meets the shape at a sharp corner (over 30°), the new edges are hard, so a band between two boxes no longer looks smeared.
- Projects are saved in **file format version 13**, which can hold slid points. Older projects still open.
- Projects are saved in **file format version 12**, which can hold instances. Older projects still open.
- Projects are saved in **file format version 11**, which can hold hard and soft edges. Older projects still open.
- Projects are saved in **file format version 10**, which can hold the clean-up edits. Older projects still open.
- In the Points, Edges and Polygons modes, Delete and Backspace now work on the selected points, edges or polygons instead of deleting the whole object. In the Model mode they still delete objects.
- Projects are saved in **file format version 9**, which can hold Connect, Bridge and Weld. Older projects still open.
- Projects are saved in **file format version 8**, which can hold extrudes with a bend. Older projects still open.
- While Extrude is in use, Ctrl + wheel changes the bend's Segments instead of growing the selection (the Select menu still grows it).
- Projects are saved in **file format version 7**, which can hold slits. Older projects still open; a project saved now won't open in an older version of the app.
- **New keys:** **Z** frames the selection (it was F, which is Loop cut now), and the axis locks are **Shift + X / Y / Z** (plain X is kept for the Slit tool).
- Projects are saved in **file format version 6**, which can hold loop cuts. Older projects still open; a project saved now won't open in an older version of the app.
- Our own names for things that had borrowed another program's names: the **Attributes panel** (was "Attribute Manager"), the **Objects panel** (was "Object Manager") and **Paint select** (was "Live selection", the brush in the Select tools). The future "Quick Align" tool will be called **Snap to**. Only names change; everything works as before.
- Projects are saved in **file format version 5**, which keeps more about each edit so it can follow later changes to the shape. Older projects still open; a project saved now won't open in an older version of the app.
- Edits hold on to their points better: when an earlier edit changes only where points are, later edits stay on the same points instead of searching for them.
- **Bevelling big selections is faster still**: every edge of a 32,000-polygon sphere now takes about a third of a second (it was 2 seconds).
- **Edits stay quick however long the history.** Dragging, adding an edit, undoing it and changing the last edit now only work out that one edit, instead of every edit since the shape was made. A small prop with 200 edits went from 26 ms per frame to under a tenth of a millisecond; a 3-million-polygon shape from 3.5 s per frame to 35 ms.
- **Faster bevels on big selections**: bevelling every edge of a 32,000-polygon sphere went from 65 s to 2 s, and every point from 5 minutes to a tenth of a second.
- **Edits find their elements again much faster** after a shape's settings change (1,000 points on a 3-million-polygon shape: 19 s to 0.15 s), and on dense shapes an edited polygon no longer picks up its neighbours.
- No pause when you let go of a drag on a heavy shape: hovering highlights again a moment later instead of making the window wait.
- Projects are saved in **file format version 4**, which can hold bevels. Older projects still open.
- Projects are saved in **file format version 3**, which can hold extrudes and insets. Older projects still open; projects saved now need this version of the app or newer.
- Wireframes too dense to see (a heavy shape from afar) fade out instead of covering the shape; zoom in and the edges come back. A selected heavy shape keeps a faint tint of the accent colour.
- In Points, Edges and Polygons mode, the edited shape's edges are drawn as fine lines, with its points as dots, by the graphics card directly.
- The Attributes panel's note on plain meshes no longer says editing arrives later.
- The menus' notes about upcoming UV tools now say Phase 8: UVs come after the modelling phases.
- Projects are saved in format version 2, so turned and scaled points are kept. Older projects still open; projects saved now need this version of the app or newer.
- **Ctrl + mouse wheel** no longer zooms the view; it grows and shrinks the selection.
- In Points, Edges and Polygons mode, what's under the mouse is now light red, and what's selected full red. Points are round dots instead of squares.
- Double-clicks may be up to half a second apart (was 0.3 s), as is standard on Windows, and two quick clicks on different things no longer count as a double-click.
- **Make editable** is now **Ctrl + E** (was C), so plain letters stay free for tools.
- **Quit has no key any more** (Ctrl + Q is gone), so a slip can't close the app: use File → Quit or the window's ×.
- The m/cm switch beside the Shape title is gone; the Attributes panel's Units menu and Settings do the same.
- Selected objects, the highlighted shape in the shape list, the chosen navigation scheme and other chosen items now use the same red as the active mode button.
- Unsaved changes are easier to see: an **Unsaved** badge next to the project's name on the right of the menu bar, and "● name (unsaved)" in the window's title instead of a small `*`.

### Fixed

- Double-clicking to select a loop works every time with the Move, Rotate and Scale tools on. Before, the first click moved the gizmo under the mouse and the second click landed on it instead.
- After dissolving an edge, its red selected line and its cyan hard-edge mark no longer stay behind.
- Bent polygons and polygons with a dent are drawn (and picked) correctly, cut along lines inside them instead of a wrong triangle and a gap.
- Picking Inset after an extrude no longer shows the extrude's bend rows as if they were Inset's.
- **Damaged or booby-trapped project files can't crash or freeze the app.** Opening a file now checks, while reading, that it doesn't ask for more than the app can handle (memory, objects, edits, polygons), and explains when it does. A file of a few kilobytes that would unpack to gigabytes is refused at once.
- An edit that would make a shape impossibly large (over 30 million polygons) is skipped with a warning in the Attributes panel, instead of freezing the app.
- After a crash, a backup copy that closed the app while being restored isn't offered again, so you can't get stuck restoring it over and over. The question says when a copy was left out.
- Backup copies are kept in folders only you can read (on macOS and Linux), and two copies of the app starting at the same moment no longer mistake each other's backups for leftovers.
- A project whose file name starts with `-` can be opened from the command line by putting `--` before it.
- Undo's memory use is counted more truthfully, including the extra data the app keeps about how each shape's parts connect, so the history stays within its budget on huge shapes.
- **A project can no longer be saved in a state the app can't open again.** Every save and backup copy is now checked to open before it replaces the previous one; if it wouldn't, the save stops with a message and your previous save stays as it was. Two ways this could happen were found and fixed: bevelling two polygons back to back, and moving points of a shape whose size was set to 0 (a size of 0 in the Coordinates panel is now ignored).
- Saving no longer writes through links placed beside a project, so a project folder from a zip or download can't make Save overwrite another file.
- Opening a project with a damaged hard edge (which only affects shading) now leaves that edge out instead of refusing the whole project.

- Opening a project while another was open could leave the viewport empty until something changed.
- On Windows, pressing keys no longer plays the system error sound after the app starts.
- The accent colour moved from the Window menu, and undo memory from the Edit menu, into the new Settings window.
- Every new shape is 1 m across as standard (a 1 m cube, a 1 m square plane, cylinders and cones 1 m across and 1 m tall, and so on), instead of 2 m.
- The grid button on the left palette now steps through three levels: a line every metre (standard), every 50 cm (each square cut into four), and every 10 cm. The View and Display menus list all three as **Grid lines**. A finer grid chosen before is kept as the 10 cm level.
- Sizes in the Attributes and Coordinates panels are shown in metres as standard; a switch beside the Shape title (and the Attributes panel's Units menu) changes to centimetres. Remembered between sessions.
- Selected objects' names in the Objects panel are now in the accent colour instead of orange, matching their outline in the viewport.
- The small ring that marks the point you orbit around is only shown when orbiting around the point under the mouse; turning around the middle of the view no longer shows it.
- A new scene starts empty: the test cube is gone, now that real shapes can be created.
- Faces show from both sides (the underside of a plane is a little darker), and the grid no longer shows through shapes lying on the ground.
- A darker, calmer look, closer to the approved mockup: darker panels with plain text tabs, small menu lines inside the panels, and a viewport with a grey sky over a slightly darker ground, a horizon, and a grid that fades out around the point you're looking at (at least 18 m around it, reaching farther as you zoom out). A green line rises from the origin to show Z, and the test cube is a mid grey.
- The viewport has its own menu line (View, Cameras, Display, Options, Panel) in place of its tab, with buttons you can drag to pan, zoom and orbit, and labels showing the view, the camera, which way the axes point, and the grid spacing.
- The active mode is now marked by a filled red square (the new accent colour) instead of orange; the opening view is a little lower, so the horizon shows.
- Axis colours are now X red, Y blue, Z green (most programs use Y green and Z blue), in the viewport, the Coordinates panel and the palette's axis locks.
- The viewport looks softer: gentler light with a slight gradient across each side, a little shade where shapes meet the ground, a quieter grid and fainter edges.
- The Coordinates panel is now just tall enough for its rows, and every panel can be dragged much smaller than before.
- Palette icons follow the mockups: soft white, with the shapes you can create in blue. Commands that aren't built yet are only slightly dimmed.
- The Coordinates panel follows the mockup: coloured X, Y and Z letters, wide dark value fields, and Object (Rel) and Apply on the right.
- Z is up: X and Y lie on the ground and Z is height, as in 3ds Max and Blender. Shown in the manual; the viewport is built this way from Phase 1.
- Project files will be saved as `.avtl` (short for Aventail) instead of `.rhy`. Nothing has been saved yet, so no files are affected.

### Added

- A **Mesh** section in the Attributes panel for every shape: points, edges, polygons and **triangles** (what a game engine draws), whether the surface is closed or open, and a check for problems (edges shared by three or more faces, faces facing the wrong way, edges with no length, faces with no area, loose points). Underneath, the app now knows how every mesh connects (edge loops and rings, borders, separate pieces), ready for the selection tools.
- **Saving and opening projects** (`.avtl` files): File → New, Open…, Open recent, Save and Save As… (Ctrl + N, O, S and Shift + S). Projects keep every object, its settings and edits, plain meshes, and the selection. Saving is safe (the old file stays until the new one is complete, and the previous save is kept as a `.bak` backup). The app asks before unsaved changes would be lost (Enter saves, Ctrl + D doesn't, Esc cancels), and the title shows the project's name with `*` for unsaved changes. A project given on the command line opens at start. New manual page: Saving and opening projects.
- A **Settings** window (**Edit → Settings…**, or **Ctrl + ,**) gathers every app-wide preference in one place: accent colour and metres/centimetres (Interface), navigation controls and grid lines (Viewport), and undo memory (Undo). **Restore standard** puts them all back. New manual page: Settings.
- Undo is now limited by memory instead of being kept forever: steps are kept until they fill 2 GB (the standard), and then the oldest are forgotten first. There's still no limit on the number of steps. **Edit → Undo memory** shows how much is in use and offers 256 MB to 8 GB or Unlimited; the choice is remembered. The status bar says once when old steps start being forgotten.
- **Make editable** (**C**, also in the Mesh menu and the Objects panel's Object and right-click menus): turns the selected shapes into plain meshes, baking in their settings. Undo brings the settings back. The Attributes panel shows a mesh's points and polygons, and the Objects panel gives it the polygon icon.
- Shapes now keep edits apart from their settings: the modelling tools coming in Phase 3 will record edits on top of a shape, so its settings (a cube's segments, say) stay editable afterwards and the edits are made again on the new shape. The Attributes panel will count a shape's edits and flag any that no longer fully fit.
- Small step arrows beside every value in the Attributes panel: click to add or take away one step (1 cm, or 1 segment) for fine tuning; hold Shift for ten.
- Press **B** for a list of shapes by the mouse: **B** again moves down it, **Shift + B** back up, and **Enter** or **Space** creates the highlighted shape. Clicking a shape in the list creates it too; **Esc** closes it.
- Cylinder and cone ends, and discs, can be cut into rings (**Cap segments**, **Ring segments**), and their middle can be one n-gon (standard) or triangles meeting at a **Centre point**.
- **Copy settings** and **Paste … settings** in the Objects panel's right-click menu.
- The Attribute Manager works: select one object to see its name, a Visible switch, and for a shape its settings (sizes in centimetres, segments, sides, closed ends and so on). Drag a value to watch the shape change, or click and type one; each drag or typed value is one undo step. Edit → Copy settings and Paste settings give several shapes of the same kind the same settings. New manual page: The Attribute Manager.
- Primitives: Create → Cube, Plane, Cylinder, Sphere, Disc, Tube or Cone (or the blue shape buttons on the command palette). Each stands on the ground at the centre of the scene, with crisp edges where it should have them and smooth curves elsewhere, and is listed in the Objects panel. Selected objects are outlined in the accent colour in the viewport, and the Attributes panel names the selection. New manual page: Primitives.
- The Object Manager works: the scene's objects are listed as a tree (for now the test cube, plus any groups you make). Click to select (Ctrl adds, Shift selects a run), double-click to rename, drag to put objects inside each other or reorder them, click the eye to show or hide, and press Delete to delete. Create → Group (or the palette's group button) makes an empty group; Alt + G puts the selection in a new group and Shift + G takes it out again. Right-click a row for these commands. F now frames the selected objects. New manual page: Objects and the Object Manager.
- Undo and redo for every change to the scene: Ctrl + Z, Ctrl + Y (or Ctrl + Shift + Z), the arrows on the command palette, or the Edit menu, which names what would be undone.
- Top, front and right views beside the perspective one, and a four-view layout: **F1** to **F4** show one view (perspective, top, right, front), **F5** shows all four, and a middle click switches between one and four (also the square button on the viewport's menu line, **Panel → Four views** and **Cameras**). The top, front and right views are flat: no perspective, so sizes read true; they pan and zoom but don't orbit, and their grid faces you. The last view clicked is the active one, marked with a thin accent line. Each view keeps its own camera, and the layout is remembered.
- Frame all (**H**) and frame selected (**F**), also in the View menus: the view keeps looking the same way and glides until everything (or the selection) fits, with a little room around it. Until objects can be selected, F frames everything.
- Orbit and zoom around the point under the mouse: grab the model where you want to turn around it, or point at a spot and roll the wheel to close in on it. A small ring marks the point while you orbit, and zooming while holding a button keeps to that point. On by default; switched off in the navigation controls or Options, orbiting and zooming work around the middle of the view.
- Navigation controls to choose from, with the button in the bottom-right corner of the viewport (or Options → Navigation controls…): **Mouse only** is now the standard (right drag orbits, middle drag pans, the wheel zooms), and Alt + mouse (as before), Mouse only with middle zooming, Middle pans with Alt orbiting, and Middle orbits can be picked instead. Alt + mouse can keep working in every scheme, and orbiting can be inverted. The status bar shows the controls in use, and the choice is remembered.
- Accent colour: red as standard, or Blue, Marine green, Plum, Olive or Petrol blue, from Window → Accent colour. Remembered between sessions. Each colour has a small swatch next to its name.
- Layout tabs on the right of the menu bar: Standard, with Modelling and UV Edit to come.
- Move around the 3D view as in Cinema 4D: hold Alt and drag with the left mouse button to orbit, the middle button to pan, the right button to zoom; the mouse wheel zooms too. The status bar shows the controls, and **View → Reset view** brings back the opening view. New manual page: Navigating the view.
- A grid detail switch at the bottom of the left palette (also **View → Fine grid lines**): the grid now shows only 1 m lines by default, for a calmer ground, and the 10 cm lines come back when you switch it on. The choice is remembered.
- The 3D viewport: a calm grey background, a grid on the ground (10 cm and 1 m lines, fading softly into the distance), the red X and blue Y axes, and a softly lit 2 m test cube. Without a usable graphics card, the viewport says why instead.
- The calm dark look from the mockups: gentle greys, a warm orange for the active mode, the Inter font, and a viewport that fills its panel edge to edge. The app stays dark whatever your system prefers.
- The app window, laid out like Cinema 4D: menu bar, command palette, mode palette, viewport, Attributes (top right), Objects (below it), Coordinates (under the viewport) and a status bar.
- Panels can be dragged by their tabs, stacked and resized; **View → Reset panel layout** brings the standard layout back. The layout and window size are remembered between sessions.
- Pick a mode (Model, Object, Points, Edges, Polygons, UV) from the left palette or the Modes menu.
- Commands that aren't built yet are listed greyed out, with the phase that brings them.
- File → Quit (Ctrl+Q) and Help → About.
- Command-line options `--version` and `--help`.
- User manual: Interface tour page.
- Project foundations: Cargo workspace with 9 single-purpose crates in five layers, with generic names so renaming the app never touches them.
- `layer-check` tool whose tests enforce the architecture rules on every change.
- Workspace-wide safety lints: no `unsafe` code, no `unwrap`/`expect`/`panic!` in library code, documentation required on every public item.
- Continuous integration: formatting, lint, tests and docs on Windows, Linux and macOS; license and vulnerability audit; downloadable builds for all three platforms; changelog check on pull requests.
- API reference published to GitHub Pages from `main`.
- Documentation: README with progress tracker, architecture overview, contributing guide, security policy, decision records, pull request and issue templates, user manual structure with a concepts page.
- Program skeleton that reports its name and version, with the app's name kept in one place (`branding.rs`).
