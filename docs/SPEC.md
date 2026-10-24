# What exists now

A short list of what the app does today, area by area: one line per thing, with where to find it. It covers the `feature/curves` branch as of 2026-10-24 (Phase 10). How each thing works is in the [user manual](manual/SUMMARY.md); why it is so is in the [decision records](decisions/README.md); what comes next is in the [progress tracker](../README.md#progress-tracker) and [DESIGN.md](../DESIGN.md) §10.

Keep this page current: a pull request that adds or changes something a user would notice updates its line here (see [CONTRIBUTING.md](../CONTRIBUTING.md#documentation-rule)). Nothing goes here until it is built.

Keys are written as on Windows and Linux; on a Mac, Ctrl is Cmd. Menu paths use →.

## The window and panels

- **Splash screen:** the name, version and what the app is doing while it starts ([interface tour](manual/interface-tour.md#starting-the-app)).
- **Menu bar:** File, Edit, Create, Modes, Select, Tools, Mesh, UV, View, Window, Help, then **Keys**; the project's name and a red **Unsaved** badge on the right ([interface tour](manual/interface-tour.md)).
- **Layouts:** **Standard** and **UV Edit** (viewport and UV Editor side by side), tabs on the right of the menu bar; each keeps its own arrangement.
- **Command palette** (top row of icons): undo, redo, the select tool (with a list arrow: Box, Lasso, Paint select), Move, Scale, Rotate, axis locks, world or object axes, snapping, symmetry, the four blue shape buttons, group, and the command search button. Names show beside the main ones when the window is wide.
- **Mode palette** (left column): the five modes, **Edit pivot** under Model, **Select only visible** and the grid button; rest the mouse on it and the names slide out.
- **Viewport:** one view or four (F1 to F5), its own menu line (View, Cameras, Display, Options, Panel), the view dice and the shading button bottom left, the navigation picker bottom right ([navigating the view](manual/navigating-the-view.md)).
- **Modifiers palette** (thin column right of the viewport): **Add edit layer** ([edit layers](manual/edit-layers.md)).
- **Attributes panel:** the selection's settings (shape settings, Smoothing, Materials, Collision, Last edit), changed live by dragging or typing ([attributes](manual/attributes.md)).
- **Objects panel:** the object tree: select, rename, nest, reorder, hide, groups (Alt + G, Shift + G) ([objects](manual/objects.md)).
- **Edit layers tab** (beside Objects): appears for an object with more than one layer, or from **Window → Edit layers**.
- **Coordinates band** (under the viewport): position, size and rotation (X, Y, Z) on one line, typed or dragged ([transforming](manual/transforming.md#typing-exact-values)).
- **Status bar:** the mode, what is selected or under the mouse, and short messages.
- **Panels** dock and rearrange by their tabs; **View → Reset panel layout** brings the standard back.
- **Windows** (Export, Settings, Check for baking, About, questions) share one look: Enter presses the main button, Esc closes.
- **Looks:** always dark; one accent colour (Red as standard; Blue, Marine green, Plum, Olive, Petrol blue); Z is up, axes X red, Y blue, Z green.

## Viewing and navigating

- **Navigation:** as standard right drag orbits, middle drag pans, wheel zooms (Alt + mouse too); four other schemes from the bottom-right button or Settings ([navigating the view](manual/navigating-the-view.md)).
- **Orbit and zoom around the point under the mouse** (switchable in Settings).
- **Frame:** H frames everything, Z the selection.
- **Views:** F1 perspective, F2 top, F3 right, F4 front, F5 all four; middle click switches one or four; **P** perspective or orthographic.
- **View dice:** click a face, edge or corner to look from there; Ctrl + click for the opposite side; Ctrl while orbiting snaps to its views.
- **Shading:** Shaded, Shaded with wires, Wireframe (bottom-left button or **Display**) ([how shapes are drawn](manual/viewport-display.md)).
- **Display menu:** Cavity (on as standard), Show hard edges, Show seams, Show normals, and **Surface**: Colours, Checker, Numbered checker (always blue to green), Stretch.
- **Grid:** lines every 1 m, 50 cm or 10 cm (left palette button).
- **Hide and isolate:** Shift + H hide, Ctrl + H isolate, Alt + H show everything; also **View → Show and hide** ([hide and isolate](manual/hide-and-isolate.md)).

## Modes and selecting

- **Five modes:** Model, Points (1), Edges (2), Polygons (3), UV; the same key again goes back to Model; **Tab** goes straight to Model and puts a tool away ([modes](manual/modes.md)).
- **Edit pivot:** a switch in the Model mode (button under Model, **Tools → Edit pivot**, no key) to move and turn pivots instead of objects ([pivots](manual/pivots.md)).
- **Click, box, lasso, Paint select** (brush); Q steps through Box, Lasso, Paint select ([selecting](manual/selecting.md)).
- **Selection keys:** Ctrl adds, Shift removes (standard; three other patterns in Settings).
- **Loops and rings:** double-click, Alt + double-click; a loop runs round an n-gon's rim (a cylinder's cap).
- **Grow and shrink:** Ctrl + wheel, or the Select menu.
- **Select all, invert, deselect:** Ctrl + A, Ctrl + I, Esc.
- **Select connected:** Alt + A (over the UV Editor, or in the UV mode: whole UV pieces).
- **Select menu:** Select flipped, Select same facing, Select hard edges, Select by material, Select seams (UV menu).
- **Select only visible:** on as standard (left palette, Settings).
- **The selection is drawn last, thickest, in the accent colour;** **Edit → Settings → Interface → High contrast** makes it bigger with a dark rim.

## Moving, rotating and scaling

- **Gizmos:** W move, R rotate, S scale, T all three; Q back to selecting ([transforming](manual/transforming.md)).
- **While dragging:** Shift ten times finer, Ctrl snaps (or not), Esc or right click cancels.
- **Axis locks** Shift + X / Y / Z; **L** world or object axes.
- **Snapping:** to points, edges, polygons and grid lines, with turn and scale steps (palette button, Settings).
- **Slide:** Alt + drag the Move gizmo slides points along their edges.
- **Typed values** in the Coordinates band.
- **Pivot commands:** Pivot to centre, Pivot to world origin, Pivot to object…, Object to world origin (Tools menu, or E with Edit pivot on).
- **Snap to and Align:** **Tools → Snap to…** and **Tools → Align…**, then click the target ([snap to and align](manual/snap-to-and-align.md)).
- **Apply transforms:** **Mesh → Shape and object → Apply transforms**, or Apply rotation and scale in E (Model mode) ([apply transforms](manual/apply-transforms.md)).
- **Units:** metres or centimetres per project (Attributes → Units, or Settings).

## Creating shapes

Every shape stands on the ground with its pivot at the middle of its base, keeps its settings in the Attributes panel, and comes with UVs, seams and hard edges where it should ([primitives](manual/primitives.md)).

- **B list, the seven basic shapes:** Cube, Plane, Cylinder, Sphere, Disc, Tube, Cone. B opens it and moves down, Shift + B up, Enter or Space creates, Esc closes.
- **Alt + B list, the eleven others:** Icosphere, Torus, Capsule, Dome, Pipe elbow, Pie sector, Gear, Hex grid, Hex sphere, Stairs, Arch, then the curves: **Draw curve**, **Line**, **Circle**, **Arc**, **Spiral**. Once open, B moves down this list too; Shift + Alt + B opens it at the bottom.
- **Create menu:** all 18, with a line between the two groups, the curves after another line, and **Group**.
- **Palette buttons:** cube, cylinder, plane, sphere.
- **Bake into shape** (Ctrl + E): turns a shape into a plain mesh without settings.

## Modelling tools

In the Points, Edges and Polygons modes. Every tool is also in the tool menu (**E**), the **Mesh** menu (in groups: Build, Cut and join, Remove, Shading, Shape and object) and command search.

- **Extrude** (D): drag to push polygons out; border edges and points along an open edge grow new polygons; a selection all round a polygon pushes it out; Direction, End size, Steps, Flat end ([extrude and inset](manual/extrude-and-inset.md)).
- **Pipe bends and spirals:** Extrude's Angle, Direction, Bend radius and Segments; past 360°, a spiral with Rise per turn.
- **Inset** (I): drag towards the polygons' middle; Keep together or each on its own.
- **Bevel** (V): edges and points; width, segments, profile, even width, skew, stop at overlaps, weld points ([bevel](manual/bevel.md)).
- **Groove** (K, Edges mode): cut a thin line into a surface or raise a ridge ([groove](manual/groove.md)).
- **Loop cut** (F): a new loop of edges, shown before the click, slid by dragging ([loop cut](manual/loop-cut.md)).
- **Slit** (X): cut along a line of clicked points, visible only or through; close the line to cut a shape in; Enter or Tab finishes, Esc cancels ([slit](manual/slit.md)).
- **Connect** (A), **Bridge** (C, or drag edge to edge), **Weld** (M at the middle, Ctrl + M drag onto a point; in the Polygons mode, each group of polygons melts at its middle) ([connect, bridge and weld](manual/connect-bridge-weld.md)).
- **Delete** (Delete) and **Dissolve** (Backspace) points, edges or polygons; **Fill hole** (J); **Flip normals**; **Fix normals** (Mesh → Shading) ([clean-up](manual/clean-up.md)).
- **Ctrl + wheel** changes the count of the tool in use: Bevel's segments, Loop cut's cuts, Extrude's steps or bend segments, Bridge's segments.
- **Last edit:** the newest edit's settings stay changeable in the Attributes panel.
- **Split off** (Mesh → Shape and object) and **Join objects** (E in Model mode, or Mesh) ([split and join](manual/split-and-join.md)).
- **Copies and instances:** Ctrl + C, Ctrl + V paste copies, Alt + V paste instances; Duplicate and Duplicate as instance in E ([copies and instances](manual/copies-and-instances.md)).
- **Booleans** (Mesh → Booleans, the modifiers palette, or command search; no keys): select the shape to keep, Ctrl + click the cutters, then **Union**, **Subtract** or **Intersect**. Cutters stay objects, drawn as outlines; move or edit one and the cut follows; hidden ones still cut; Attributes → **Booleans** changes the kind or takes a cutter out; **Bake into shape** makes it permanent and hides the cutters. Closed solids only: anything else is left uncut with the reason shown ([booleans](manual/booleans.md)).
- **Modifiers** (the modifiers palette, Mesh → Modifiers, or command search; no keys): added to the selected shapes, worked out live after the edits, booleans and mirror, first to last; Attributes → **Modifiers** switches each on or off, moves it Up or Down, removes it, changes its settings and **Show in viewport** (off: skipped in the viewport, still exported and baked). Exports carry those that are on; **Bake into shape** makes them permanent, with the mirror ([modifiers](manual/modifiers.md)).
- **Remesh** (a modifier): a closed shape rebuilt as an even surface of quads, **Voxel size** 2 cm and **Smooth** 2 passes as standard, at most 2 million voxels; materials kept, box-projected UVs with seams; open shapes left as they are with the reason shown ([remesh](manual/remesh.md)).
- **Subdivide** (a modifier): Catmull-Clark subdivision smoothing, **Levels** 1 in the viewport and **Levels for export** 2 (0 to 5, at most 4 million polygons), **Hard edges stay sharp** on objects shaded by their marked edges; UVs smoothed keeping corners, materials and seams kept, open borders and sheet corners kept ([subdivide](manual/subdivide.md)).
- **Lattice** (a modifier with its own object): **Add lattice** fits a cage of points round the selected shapes (**Divisions** 2 × 2 × 2 as standard, up to 16 a side; **Interpolation** Smooth or Linear; **Reset points**); move its points in the Points mode (selecting, gizmo, Coordinates) and the shapes bend smoothly; the cage is drawn as lines, picked by them, and left out of exports; deleting it removes its modifiers in one step ([lattice](manual/lattice.md)).
- **Scatter onto surface** (a modifier; the surface selected first, the shapes to scatter Ctrl + clicked after): copies of the shapes on the surface's **Points**, **Polygon centres** or **Random over the surface** (**Count** 50), with **Seed**, **Spacing**, **Align to surface** (on), **Random turn**, **Smallest/Largest size**, **Offset** and **On polygons** (**Use selected**, **Whole surface**); the shapes stay objects (hidden, they still scatter), copies stand on their pivot, materials matched by name; exported as one mesh with the surface ([scatter](manual/scatter.md)).
- **Bend along curve** (a modifier; the shape selected first, a curve Ctrl + clicked after): the shape laid along the live curve from its start, through its pivot, on the sweep's frames, with **Axis** (Longest side, X, Y, Z), **Stretch to fit**, **Offset** and **Twist** ([bend along curve](manual/bend-along-curve.md)).
- **Shrinkwrap onto** (a modifier, **Shrinkwrap**; the shape selected first, the target Ctrl + clicked after): the shape's points moved onto the target's surface, **Mode** Nearest surface point, Along the shape's normals or Along X/Y/Z, with **Offset** and **Points** (**Use selected**, **All points**) ([shrinkwrap](manual/shrinkwrap.md)).
- **Decimate** (a modifier): fewer triangles with the shape kept, by quadric error collapse; **Reduce by** a **Ratio** (50% as standard) or **Target triangles**; **Keep UV seams**, **Keep hard edges** (as the shading shows them), **Keep material borders** and **Keep open borders** (all on), a kept line's points sliding only along it and its corners staying; triangles out, or **Quads where flat**; UVs, materials, hard edges, seams and creases carried; **Triangles shown** in the Attributes panel ([decimate](manual/decimate.md)).
- **Weighted normals** (a modifier, best last): shading where bigger polygons count for more, so big flat faces shade flat and bevels round; **Weight by** Polygon area (standard), Corner angle or Area and angle, **Face influence** 75%, **Strength** 100%, **Keep hard edges** (on, by the object's Smoothing); stored as custom normals per corner, drawn, exported (glTF normals and tangents, OBJ normals) and kept by Bake into shape; a modifier after it, or an edit that moves points after baking, drops them, with an amber note in Attributes ([weighted normals](manual/weighted-normals.md)).
- **Live bevel** (a modifier, family Generate; added before any Subdivide or Weighted normals): the improved bevel on the edges chosen by **Edges**: **By angle** (sharper than **Angle**, 30° standard), **Hard edges** (as the object's Smoothing shows them) or **Marked edges** (by **Bevel weight**, each that share of the width); **Width** 2 cm, **Segments** 2 (1 to 64), **Profile** 100% (round), **Stop at neighbours** on; even width and welding on; refused with a reason when there's nothing to bevel; worked out on another thread from about 1 200 bevelled edges ([live bevel](manual/live-bevel.md)).
- **Quadify** (a modifier, family Clean up; added before any Live bevel, Subdivide or Weighted normals) and **Quadify now** (once, a recorded edit: **Mesh → Clean up**, command search; the selected polygons in the Polygons mode, whole shapes in the Model mode; settings under Last edit, remembered for the next): pairs of triangles joined into quads, the squarest first; **Max angle** (40°) between the two triangles, **Max shape distortion** (40°, a corner's distance from a right angle), no dents; **Keep UV seams**, **Keep hard edges** (as the object's Smoothing shows them), **Keep material borders**, all on; creased and bevel-weighted edges always stay; no keys ([quadify](manual/quadify.md)).
- **Bevel weights** (Edges mode; no keys): a **Bevel weight** field (0 to 1) beside Crease in the Attributes panel's Selected edges section, a recorded edit (one drag, one edit and undo step; mirrored with symmetry); drawn lime green while editing ([live bevel](manual/live-bevel.md#marking-edges-with-a-bevel-weight)).
- **Creases** (Edges mode; no keys): **Crease edges** and **Uncrease** in the E ring's **Creases** group (Flip normals' place), Mesh → Shading and command search; the Attributes panel's **Crease** field (0 to 1) for the selected edges; drawn magenta while editing ([subdivide](manual/subdivide.md#keeping-edges-sharp-creases)).
- **Curves** (Create, Alt + B, command search; no keys): **Draw curve** (click a sharp point, press and drag a smooth one; Enter or right click finishes, Esc cancels, Backspace takes back, the first point closes; on the ground or a flat view's plane, snapping when on) or ready-made Line, Circle, Arc, Spiral. Attributes → **Makes**: Line only, **Sweep** (circle, square, rectangle or another curve as the cross-section; width, height, sides, segments, adaptive, twist, caps) or **Lathe** (axis, spin angle, segments round, caps), live. Points mode: points and handles selected and moved like a mesh's; double-click switches sharp and smooth; Delete takes points away; Closed in Attributes. Bake into shape makes a mesh; a bare line exports as nothing ([curves](manual/curves.md)).
- **Spline editing** (Points mode, curves selected): double-click the line adds a point keeping the shape; Delete keeps the shape near; **M** / **A** on two end points weld them at the middle / join them with a straight stretch (one curve: closes it); **Join objects** joins curves end to end; Draw curve carries a selected curve on from either end (click the end first). **E** shows the curve ring; **Mesh → Curve** and command search hold **Split curve**, **Reverse curve**, **Open or close curve**, handle types (**Sharp**, **Smooth**, **Broken**, **Auto**), **Round corners** (radius) and **Even spacing** (count); Attributes → **Curve points** has the handle type, radius and count ([curves](manual/curves.md)).
- **Heavy shapes:** a 3-million-polygon object stays usable; **Help → Developer → Add a heavy test shape** to try it ([heavy shapes](manual/heavy-shapes.md)).

## The edit history and edit layers

- **Undo and redo:** Ctrl + Z, Ctrl + Y (or Ctrl + Shift + Z), for everything; limited by memory (2 GB as standard), not by steps.
- **Every edit is recorded on top of the shape,** so a primitive's settings stay editable afterwards ([concepts](manual/concepts.md#objects-and-scenes)).
- **Drags in a row** of the same points make one edit (each still its own undo step).
- **The edit list** (Edit layers tab): click any edit to change it in Attributes; switch it off, **Bake into shape up to here**, or **Delete edit** ([edit list](manual/edit-list.md)).
- **Edit layers:** **Add edit layer** (modifiers palette, Mesh → Shape and object, or the panel); switch off, rename, move up or down, merge down, delete ([edit layers](manual/edit-layers.md)).
- **Edits find their parts again** when an earlier edit or a shape's settings change; ones that can't say **parts missing**.

## Smoothing and symmetry

- **Smoothing per object:** Attributes → **Smoothing**: By angle (30° as standard) or marked edges only; **Mesh → Shading → Smooth by angle** for several objects ([hard and soft edges](manual/hard-and-soft-edges.md)).
- **Mark hard edges, Mark soft edges** (E or Mesh → Shading); on an object smoothed by angle, the first mark keeps its look and switches it to marks, in one undo step.
- **Hard edges by angle** (Mesh → Shading): marks every edge from an angle, changeable under Last edit.
- **Symmetry per object:** the **Symmetry** palette button or **Tools → Symmetry** gives the object a live mirror at its pivot, moving and turning with it; the arrow beside it sets the axes, the mirror's offset and the weld distance ([symmetry](manual/symmetry.md)).
- **Apply mirror** (Symmetry menu, Mesh → Shape and object, or command search): makes the mirrored half real and switches symmetry off, in one undo step.

## UVs

The UV tools are in the **UV** menu (main menu bar, and the **UV** button on the UV Editor's menu line), in **E** over the UV Editor or in the UV mode, and in command search.

- **UV Editor:** the selected objects' UVs on the texture square, one selection with the viewport; **UV Edit** layout or **Window → UV Editor**; menus UV, Texture, Trim, Snapshots, View ([UV Editor](manual/uv-editor.md)).
- **UV mode:** a click selects a whole piece, in the editor or on the model.
- **Editing by hand:** W, R, S, T gizmo in the editor; **Turn & flip** in E ([editing UVs](manual/editing-uvs.md)).
- **Editing on the model:** in the UV mode, W, R, S and drag in the viewport ([UVs on the model](manual/uvs-on-the-model.md)).
- **Seams:** Mark seams, Clear seams (E), Seams from hard edges, Suggest seams, Select seams (UV → Seams); shown in orange ([seams](manual/seams.md)).
- **Seam preferences:** Prefer seams here, Never seams here (UV → Seams); **stretch budgets** (UV → Pieces) ([seam preferences](manual/seam-preferences.md)).
- **Unwrap, Relax, Straighten** (E or UV → Lay out); live unwrap as seams change; Unfold across only, Unfold up only; four methods ([unwrapping](manual/unwrapping.md)).
- **Pins:** Pin, Unpin (E or UV → Pieces), no key ([pins](manual/pins.md)).
- **Locked pieces and piece settings** (UV → Pieces) ([locked pieces](manual/locked-pieces.md)).
- **Texture size and texel density:** the editor's **Texture** menu; Set and Match texel density (UV → Lay out) ([texture and density](manual/texture-and-density.md)).
- **Pack** (E or UV → Lay out): padding in real pixels, nesting, texture sets per material or object, stacking identical pieces ([packing](manual/packing.md)).
- **Settings of every automatic tool:** UV → Settings, and the **UV settings** panel in the UV Edit layout; same model and settings always give the same UVs ([UV settings](manual/uv-settings.md)).
- **Checks while you work:** overlapping (red), mirrored (purple) and outside-the-square (amber) pieces counted; **View → Numbers on pieces**, **View → Measure** ([seeing and checking](manual/seeing-and-checking.md)).
- **View → Explain choices:** every seam and piece says why it's there and how to change it ([why is this here?](manual/why-is-this-here.md)).
- **Checker and Stretch views** on the model and in the editor ([checker and stretch](manual/checker-and-stretch.md)).
- **UVs survive modelling:** new polygons get fitting UVs; hand work is kept ([UVs and modelling](manual/uvs-and-modelling.md)).
- **Trim sheets:** rows in the editor's **Trim** menu; **UV → Lay out → Put on trim row** ([trim sheets](manual/trim-sheets.md)).
- **Layout snapshots:** Take, Restore, Compare, Update, remove (the editor's **Snapshots** menu) ([layout snapshots](manual/layout-snapshots.md)).

## Materials

- **Material slots:** name and colour per material, Add material, Assign, Remove last, in Attributes → Materials; up to 64 per object ([materials](manual/materials.md)).
- **Select → Select by material.**
- Materials of the same name are one material on export (a texture set in Substance Painter, a surface in engines).

## Saving, backups and the emergency copy

- **Projects** (`.avtl`): File → New (Ctrl + N), Open… (Ctrl + O), Open recent, Save (Ctrl + S), Save As… (Ctrl + Shift + S), Quit (no key) ([saving and opening](manual/saving-and-opening.md)).
- **Safe saving:** written beside the old file, checked to open again, then swapped in; the previous save kept as `.avtl.bak`.
- **"Save changes?"** before work is lost: Enter saves, Alt + N doesn't save, Esc cancels.
- **File format 51**, the development format with booleans, curves (and each curve point's handle type), improved bevels, modifiers, Subdivide with creases, lattice cages, Decimate, Scatter, Bend along curve, Shrinkwrap, Weighted normals, Live bevel with bevel weights and Quadify, until the alpha; older projects still open ([decisions 0126](decisions/0126-rework-and-first-use.md), [0135](decisions/0135-live-booleans.md), [0138](decisions/0138-curves-sweep-lathe.md), [0139](decisions/0139-spline-editing.md), [0141](decisions/0141-improved-bevel.md), [0142](decisions/0142-modifier-stack-and-remesh.md), [0143](decisions/0143-subdivide-and-creases.md), [0144](decisions/0144-lattice-cage.md), [0145](decisions/0145-decimate.md), [0146](decisions/0146-scatter.md), [0147](decisions/0147-curve-deform-and-shrinkwrap.md), [0148](decisions/0148-weighted-normals.md), [0149](decisions/0149-live-bevel.md) and [0150](decisions/0150-quadify.md)).
- **Damaged or hostile files** are refused with a message, never a crash.
- **Backup copies:** every 5 minutes when something changed, newest 5 kept, offered back after a crash (Settings → Autosave) ([backup copies](manual/autosave.md)).
- **Emergency copy:** if the app itself crashes, it saves the work as it was a moment before, offered first at the next start.

## Export and import

- **Export…** (File): glTF (`.glb`) or OBJ (`.obj` with `.mtl`); Everything shown or Selected only; what goes out shown first ([export and import](manual/export-and-import.md)).
- **Made for** presets: **Godot** (standard), **Unity**, **Unreal Engine**, **Solis** (glTF, metres, Y up, triangles), **Substance Painter** (OBJ, triangles), or Custom settings.
- **Your last export's settings are remembered,** also after closing the app.
- **Export again** (File) repeats the last export.
- **Export into a game project:** a folder in a Godot, Unity, Unreal Engine or Solis project, exported into on every save and from **File → Export to game project**.
- **Export kit…** (File): each selected piece its own file, pivot at the origin; **Collision** (Attributes) for Godot ([kit export](manual/kit-export.md)).
- **What goes out:** shapes as they're shown, cut by their booleans, with cutters and lattice cages left out, with every modifier that is on; curves as their sweep or lathe (a bare line not at all); each object at its pivot (glTF), its own smoothing, the viewport's triangles, MikkTSpace tangents for shapes with UVs, materials by name.
- **Check for baking…** (UV menu, command search, or the Substance Painter preset's button): problems in the model and its UVs, with Select and one-click fixes ([check for baking](manual/bake-check.md)).
- **Import…** (File): OBJ files, as plain meshes.

## Settings

- **Edit → Settings…** (Ctrl + ,): sections Interface, Viewport, Undo, Autosave; Restore standard ([settings](manual/settings.md)).
- **Interface:** accent colour, High contrast, the tool menu's look and keys, Tap E to open, Right-click opens the quad menu (on), the Dissolve warning, units.
- **Viewport:** navigation controls, Alt + mouse, orbit around the mouse, invert orbit, grid lines, Viewport display switches, selection keys, snapping, Select only visible.
- **Undo:** undo memory, 256 MB to 8 GB or Unlimited.
- **Autosave:** on or off, how often, how many copies.
- The viewport's **Display** and **Options** menus end with **All settings…**.

## Help and finding things

- **Command search** (Space, the palette's magnifying glass, or **Help → Search for a command**): type part of a name, Enter runs it; shows the menu and key ([command search](manual/command-search.md)).
- **Tool menu** (E): the tools for the mode (or for UVs over the UV Editor) as a ring or a list ([tool menu](manual/tool-menu.md)).
- **Quad menu** (right-click in the viewport, on as standard): Show, Modes, Edit, Tools ([quad menu](manual/quad-menu.md)).
- **Keys** (menu bar, or Help → Keys): every key on a drawn keyboard in seven layouts; Esc closes ([keyboard shortcuts](manual/keyboard-shortcuts.md)).
- **Help → User manual** and **Help → Words used in the app** open the manual and the glossary in the web browser ([concepts](manual/concepts.md)).
- **Help → About**, and **Help → Developer** for test tools.
- **Tooltips** say what each command does, and why a greyed-out one can't be used now.

## Not yet

The big things planned and not built. The order is the rework (Phase 9), then advanced modelling (Phase 10), then the rest of the UVs and baking (Phase 8), then texture painting ([decision 0125](decisions/0125-modelling-before-the-rest-of-the-uvs.md)).

- **The rest of Phase 9:** the key audit's changes (written, waiting for the project lead's ticks); English as the one language package; the big interface files split; the gate checks of Phases 4 to 7; the newcomer test.
- **Phase 10, advanced hard-surface:** weighted normals, a better bevel, array and mirror generators, shell, a smoothing (subdivision) modifier with creases, a game bevel, modular kit helpers.
- **The rest of Phase 8, UVs and baking:** straighten by edge flow, a seam slider and brush, projections, stitch and cut, UV snapping and exact placement, UV brushes, more selection tools, packing groups and UDIM tiles, several UV sets, lightmap UVs, real textures behind the UVs and UV layout export, explained auto-unwrap, unwrap recipes, player-view UVs, UV intents, paint-aware detail, and the in-app baker (normal, AO, curvature and more, high to low, cages).
- **Later phases:** texture painting (11), a material node editor (12), alpha release (13), polish (14), scripting in LustScript and Lua (15), rigging (16), animation (17), effects (18).
- **Wanted, not scheduled:** glTF import, custom hotkeys, other languages, interface size scaling, a first-start welcome with sample projects, FBX export, an installer and file association.
