# Concepts

Plain-language explanations of the ideas Aventail 3D is built around. Other pages link here instead of re-explaining.

## Mesh basics

**Vertex (point)** — A single position in 3D space. Everything else is built from these.

**Edge** — A straight line between two vertices.

**Polygon (face)** — A flat surface enclosed by edges. Aventail 3D works with:
- **Triangles** (3 sides): what game engines and GPUs actually render. Everything gets triangulated on export.
- **Quads** (4 sides): the easiest to work with while modelling. Loop and ring tools rely on them.
- **N-gons** (5+ sides): fine on flat surfaces in hard-surface work, but they can triangulate unpredictably on curved ones and cause shading errors.

**Element (component)** — A vertex, edge or polygon: one of the parts a mesh is made of. The Points, Edges and Polygons modes work on elements instead of whole objects (see [Modes](modes.md)).

**Selecting through** — Selecting with a box, lasso or brush can take only what faces you (*visible only*), or everything inside the outline, including the far side of the shape hidden behind it (*through*). The app selects only what you can see unless you switch **Select only visible** off: see [Selecting](selecting.md).

**Topology** — How the vertices, edges and polygons are connected, as opposed to where they sit. Two meshes can have the same shape but different topology.

**Edge loop** — A chain of edges running through quads, end to end, like a belt around the mesh. **Edge ring** — the edges running *across* a loop, like the rungs of a ladder.

**Loop cut** — Cutting a new edge loop through a ring of quads: every edge of the ring is cut, and the new points are joined across each quad. It adds rows exactly where you need them, instead of everywhere. See [Loop cut](loop-cut.md).

**Mitre joint** — Where two straight pieces meet at an angle with no rounding, each cut at half the angle, like the corner of a picture frame. A pipe bent this way keeps its full width. See [bending an extrude](extrude-and-inset.md#bend-it-pipes-that-turn-a-corner).

**Cavity** — A way of drawing a model in the viewport that brightens ridges and darkens creases and inner corners, like dust settling in the cracks and light catching the edges. It makes a shape's form read clearly without changing it. See [How shapes are drawn](viewport-display.md#cavity).

**Spiral (helix)** — A curve that goes round and round while climbing along a line, like a spring, a screw thread or a spiral staircase. An extrude bent past one full turn becomes one. See [Spirals](extrude-and-inset.md#spirals-going-round-more-than-once).

**Bridge** — A band of new polygons between two openings (open borders, or two polygons that are removed), like a tube between two holes. See [Connect, Bridge and Weld](connect-bridge-weld.md).

**Weld** — Melting points into one, at their middle or onto a chosen point. Polygons squeezed to fewer than three corners disappear. See [Connect, Bridge and Weld](connect-bridge-weld.md).

**Dissolve** — Removing an edge or point without leaving a hole: the polygons around it merge into one. *Deleting* removes the polygons too, and leaves a hole. See [Clean-up](clean-up.md).

**Bent polygon** (non-planar polygon) — A polygon whose corners don't all lie flat on one plane, like two sides of a box merged into one round the corner. It can't be drawn as one flat shape, so it's cut into triangles along its fold; it shades less cleanly than flat polygons and game engines may cut it differently. Dissolve asks before making one across a sharp corner. See [Clean-up](clean-up.md#dissolving-across-a-sharp-corner).

**Fill hole** — Closing an opening (a loop of border edges) with a polygon, like a lid. See [Clean-up](clean-up.md).

**Slide** — Moving points along the edges they already have, so they stay on the surface, instead of moving them freely through space. See [Move, rotate and scale](transforming.md#slide).

**Slit** — Cutting new edges along a line drawn over the shape, across any polygons, as you see them (or right through the shape). Other programs call this a knife or line cut. See [Slit](slit.md).

**Triangle count** — How many triangles a model becomes once every face is cut into triangles, as game engines and GPUs do: a quad is 2, an n-gon with 8 corners is 6. It's the usual measure of how heavy a game asset is. The Attributes panel shows it for every shape.

**High-poly and low-poly** — A *high-poly* model has millions of small polygons for fine detail, like a sculpt or a 3D scan; it's too heavy for a game. A *low-poly* model is the light version a game uses, often modelled on top of the high-poly one (*retopology*), with the detail baked into textures. The app copes with high-poly shapes in the scene: see [Heavy shapes](heavy-shapes.md).

**Border (open edge)** — An edge with a face on one side only, like the rim of a plane or the open end of a pipe. A shape with none is *closed*, like a solid. Borders are fine where you mean them; in a shape meant to be solid, they're holes.

**Flipped normal** — Every face has a front, the side its *normal* points to. Neighbouring faces should all face outwards; a face pointing the other way is *flipped*, and shades, bakes and exports wrongly. [Clean-up](clean-up.md) finds and fixes them.

**Extrude** — Pushing polygons (or border edges) out of a shape and building new polygons, the *walls*, between where they were and where they are now: the most common way to add volume. Pushing them in makes a recess. See [Extrude and inset](extrude-and-inset.md).

**Inset** — Shrinking polygons inward, flat on the surface, with a ring of new polygons around them: a frame or border drawn onto the shape, often followed by an extrude. See [Extrude and inset](extrude-and-inset.md).

**Bevel and chamfer** — Cutting a sharp edge away and putting a strip of polygons in its place. A *chamfer* is a single flat strip; a *bevel* with more segments is rounded. A *cove* is a bevel that curves inward instead, hollowing the edge out. Where several bevelled edges meet at a corner, a rounded bevel fills the corner with a small curved *corner patch*, like a piece of a ball on a rounded box. Real objects rarely have perfectly sharp edges, so a small bevel catches the light and makes a model read as solid. See [Bevel](bevel.md).

**Panel line (groove, ridge)** — A thin line cut into a surface (a groove) or raised out of it (a ridge): the seams between panels on a gun or a vehicle, slots, score lines, trims. They break up big flat areas so a model reads as built from parts, and they bake well into normal maps. See [Groove](groove.md).

**Manifold** — A mesh that could exist as a real, solid object: every edge is shared by exactly two polygons. Non-manifold geometry (edges shared by three faces, loose edges, holes) often breaks booleans, bakes and some tools.

**Closed solid (watertight)** — A shape whose surface closes all the way round, with no holes, so it has a clear inside and outside, like a sealed box. It is [manifold](#mesh-basics) and has no [borders](#mesh-basics). Booleans, 3D printing and some bakes need closed solids. A plane, or a cylinder without caps, isn't one.

**Subdivision (subdivision smoothing, subD)** — Smoothing a shape by splitting each polygon into smaller ones and moving every point towards the average of its neighbours, level after level, so a simple low-polygon **cage** stands for a smooth surface. The usual method, which this app uses, is **Catmull-Clark**: each level turns every polygon into quads, four times as many each time. It changes the shape itself, unlike [smoothing the shading](#shading-and-normals), which only changes how light falls on it. (3ds Max calls it TurboSmooth, Blender a Subdivision Surface.) See [Subdivide](subdivide.md).

**Crease** — A value on an edge, 0 to 1, for subdivision: how sharply the edge is kept. 0 is smoothed like any other edge, 1 stays fully sharp, and in between the edge stays sharp for part of the levels, coming out as a tighter curve. Creases don't change the shading or the shape without a subdivision modifier. See [Subdivide](subdivide.md#keeping-edges-sharp-creases).

**Boolean** — Combining shapes as if they were solid blocks: **Union** joins them into one, **Subtract** cuts one out of another, **Intersect** keeps only where they overlap. The name comes from the logic words *or*, *and not* and *and*. See [Booleans](booleans.md).

**Cutter** — In a boolean, a shape that cuts (or is added to, or intersected with) the shape you keep. In this app cutters stay objects of their own, drawn as outlines, and the cut follows when you move or edit them. See [Booleans](booleans.md).

## Curves

**Curve (spline)** — A line through space defined by a few points, bending smoothly or turning sharply at each one. Curves aren't surfaces: a game engine can't draw one on its own. They're guides that become solids through a sweep or a lathe. In this app a curve is an object of its own. See [Curves](curves.md).

**Bézier handle** — The two short arms on a curve's point that pull the curve towards them, like magnets: the longer the arm, the further the curve swings out before it turns. A point with no handles is a sharp corner; a *smooth* point keeps its two handles in one line, so the curve passes through without a kink. Named after Pierre Bézier, who used these curves to design car bodies; drawing programs' pen tools use the same idea.

**Sweep** — A solid made by carrying a cross-section (the profile) along a curve, like squeezing icing from a piping bag along a path: a circle makes a pipe or a cable, a rectangle a bar, a moulding's outline a trim. (Other programs call it *extrude along path*, *loft* or a *sweep NURBS*.) See [Curves](curves.md).

**Lathe** — A solid made by spinning a curve round an axis, like turning wood or throwing a pot: draw half a vase's outline and it becomes the vase. (Also called a *revolve* or *spin*.) See [Curves](curves.md).

**Profile (cross-section)** — The shape a sweep carries along its curve: what you'd see if you sawed through the pipe or rail. A built-in circle, square or rectangle, or another curve you draw. See [Curves](curves.md).

**Handle types** — How a curve point's two handles behave. *Sharp*: no handles, a corner. *Smooth*: the handles stay in one line, so dragging one swings the other round with it and the curve never kinks there. *Broken*: each handle moves on its own, for a kink with curved sides (a heart's top, a leaf's tip). *Auto*: the app works the handles out from the neighbouring points, so the curve stays smooth as they move; pulling an auto handle by hand makes the point smooth. (Other programs call these *corner*, *aligned* or *smooth*, *free* or *vector*, and *automatic*.) See [Curves](curves.md#handle-types).

**Fillet (round corner)** — A sharp corner replaced by a small arc of a chosen radius, as a woodworker rounds a table's corner or an engineer rounds a bracket's: softer to look at, catches light, and in a sweep makes a smooth bend in the pipe instead of a mitred one. The app's command is **Round corners**. See [Curves](curves.md#rounding-corners).

**Resample (even spacing)** — Putting a curve's points again at equal distances along it, keeping its shape: a drawn curve's points often bunch up in some places and spread out in others, which makes its sweep's detail and twist uneven. The app's command is **Even spacing**. See [Curves](curves.md#spacing-points-evenly).

## Objects and scenes

**Object** — Anything listed in the Objects panel: a mesh, a primitive, a group. See [The Objects panel](objects.md).

**Group** — An empty object whose only job is to hold other objects, so they can be moved, hidden or exported together. (Some programs call it a *null* or an *empty*.) Put the parts of one asset in a group and give it the asset's name.

**Mode** — What kind of thing your tools work on: whole objects, or a mesh's points, edges or polygons, or its UVs. The same Move tool moves a whole object in Model mode and a single edge in Edges mode. See [Interface tour](interface-tour.md#modes).

**Tool menu** — The menu that opens at the mouse when you press **E**, holding the tools for what you're working on: object tools in Model mode, modelling tools while editing points, edges or polygons. Shown as a ring (a *radial* or *pie* menu) of icons, whose places never move, or as a list. See [The tool menu](tool-menu.md).

**Primitive** — A ready-made shape (cube, cylinder, sphere…) built from a few settings, like size and segments, which stay editable. In this app every primitive stands on the ground with its pivot at the middle of its base. See [Primitives](primitives.md). Ready-made [curves](#curves) (line, circle, arc, spiral) work the same way.

**Project file (.avtl)** — The file your scene is saved in: every object, its settings and edits, and what was selected. It's this app's own working format; models go to other programs by *exporting* them instead. See [Saving and opening projects](saving-and-opening.md).

**Backup copy (autosave)** — A copy of your scene the app saves by itself in the background, every few minutes while you have unsaved changes, so a crash or power cut costs at most those minutes. Backup copies are kept in the app's own folder, never in place of your project file, and are offered back the next time the app starts if it didn't close normally. See [Backup copies and crash recovery](autosave.md).

**Emergency copy** — One last backup copy the app saves by itself when it crashes with an error, holding your work as it was a moment before. It is offered back at the next start with the other backup copies. See [Backup copies and crash recovery](autosave.md#the-emergency-copy).

**Base and edits** — A shape in this app is kept in two parts: its *base*, what it starts from (a primitive's settings, or a plain mesh), and the *edits* recorded on top of it, such as moving some points. The final shape is the base with the edits applied. Because the two are kept apart, a primitive's settings stay editable after you've modelled on it: change a cube's height and your edits are made again on the taller cube. Moving, rotating and scaling points, edges and polygons records edits like this ([Move, rotate and scale](transforming.md)). Each edit also remembers *where* its points were, so when new settings renumber the points (more segments, say), it finds them again by place ([Changing settings after editing](transforming.md#changing-settings-after-editing)).

**Edit layer** — A set of edits kept together on an object. An object's edits are kept in layers, applied bottom first, each working on the shape the ones below it made; new edits go into the top layer that is switched on. A layer can be switched off to see the shape without its edits, moved, merged into the layer below or deleted with everything in it. 3ds Max users know the idea as a stack of Edit Poly modifiers. See [Edit layers](edit-layers.md).

**Edit list** — The list of an object's edits in the Edit layers panel. Any edit in it can be opened and changed, and the edits after it are made again. *Bake into shape up to here* bakes the old edits up to one into the shape, so they no longer have to be made again: the shape stays the same, but those edits can't be changed any more. See [The edit list](edit-list.md).

**Modifiers palette** — The thin column of buttons on the right of the viewport, beside the panels, for things that change a shape without destroying it. Its first button adds an edit layer; then come the [booleans](booleans.md) (Union, Subtract, Intersect), then one button per kind of [modifier](#objects-and-scenes), grouped by family (Subdivide, then Remesh, so far). More join it in later versions.

**Modifier** — A change worked out live on top of a shape, after its edits, booleans and mirror, rather than made to its points: you can change its settings, switch it off, reorder it or remove it at any time, and the shape underneath stays as you modelled it. An object's modifiers are its **modifier stack**, worked out first to last, each on the result of the one before. **Bake into shape** makes them permanent. (Blender and 3ds Max call them modifiers too; Cinema 4D has generators and deformers.) See [Modifiers](modifiers.md).

**Remesh** — Rebuilding a shape's surface from scratch as even polygons, ignoring how it was made: useful after booleans, joins or rough modelling, at the price of sharp corners rounded slightly and UVs to redo. See [Remesh](remesh.md).

**Voxel** — A small cube of a 3D grid, the 3D cousin of a pixel. A voxel remesh fills a grid of voxels with the shape (each inside or outside), then wraps a new surface round the inside ones; the **voxel size** is how big the cubes are, and so roughly how big the new polygons are.

**Bake into shape** — Turning a shape into a plain mesh with no settings: its base and edits are baked together, so what you see becomes the new starting point. Do it when you no longer need the settings, or before editing a mesh freely. Undo brings the settings back. A shape's [modifiers](#objects-and-scenes) that are on are baked in too. (Other programs call it *make editable*, *convert to editable poly* or *collapse*.) See [Objects](objects.md#bake-into-shape).

**Hierarchy** — Objects can be children of other objects (sit *inside* them, in the Objects panel's tree). Moving a parent moves its children with it, and hiding it hides them too.

**Undo history** — The list of changes you've made, kept so they can be taken back (undo, `Ctrl + Z`) and done again (redo, `Ctrl + Y`). This app keeps steps while it's open, up to a memory limit (2 GB as standard, set in **Edit → Settings… → Undo**), then forgets the oldest first. There's no limit on the *number* of steps. See [Objects](objects.md).

**Pivot (axis / origin)** — The point an object rotates and scales around. In Godot, it's also the point the asset is placed and snapped by, so where you put it matters: on the hinge of a door, at the bottom corner of a wall piece. The [gizmo](transforming.md) sits on it. With **Edit pivot** on you can move and turn it without moving the shape; see [Pivots and Edit pivot](pivots.md).

**Stretched object (uneven scale)** — An object scaled more along one of its axes than another in the Model mode, say twice as tall as it is wide. The shape itself isn't changed: the object is drawn stretched. Edits that measure across a shape (an inset's border, a bevel's width, which way is "straight out", a bend) have to know about the stretch, or they come out stretched too: a border wider on one side, a round pipe turning oval. Extrudes, insets, bevels, grooves and bends in this app take the object's stretch into account ([on a stretched object](extrude-and-inset.md#on-a-stretched-object)). Scaling the points in the Points mode instead changes the shape itself, with no stretch on the object; many modellers prefer it for that reason. To fix the stretch for good, [apply the scale](apply-transforms.md): the shape takes it, and the object goes back to 100%.

**Applying transforms (freezing)** — Making an object's placement part of its shape: its rotation goes back to 0° and its scale to 100% (its position to the centre, if you choose), while it looks the same, because its points move to where they were seen. Afterwards edits and game engines see a plain, unturned, unscaled object. Other programs call it *apply*, *freeze transformations* or *reset XForm*. See [Apply transforms](apply-transforms.md).

**Gizmo** — The handle drawn on what's selected while moving, rotating or scaling: arrows, rings and boxes in the axis colours. Dragging one part changes the selection in that direction only. See [Move, rotate and scale](transforming.md).

**Snapping** — Making a move, turn or resize jump to exact places (a point, an edge, the grid) or steps (15°, 10%) instead of going wherever the mouse is. See [Move, rotate and scale](transforming.md#snapping).

**World and object axes** — The *world's* axes are the same everywhere: Z is always up. An *object's own* axes turn with it: a cube turned on its side has its own Z pointing sideways. The gizmo can follow either (press `L`).

**Axes (X, Y, Z)** — The three directions of 3D space, always coloured X red, Y blue, Z green. (Most 3D programs colour Y green and Z blue; this app swaps them, so blue is the second ground direction. The letters are always shown too.) In this app **Z points up**, and X and Y lie flat on the ground; a green line rising from the origin marks Z. Some 3D programs, Godot and the glTF file format use Y for up instead; the app converts when it exports, so models arrive standing upright.

**Origin and grid** — The origin is the point where X, Y and Z are all zero: the centre of the world, where the red and blue axis lines cross. The grid on the ground helps you judge sizes: in this app its lines are 1 m apart, and you can add fainter lines every 50 cm (cutting each square into four) or every 10 cm when you need them. It's drawn around the point you're looking at and fades out at its edges, so it reads as a work surface rather than an endless floor. Lengths are shown in metres as standard, or centimetres if you prefer; 1 m is 100 cm.

**Units** — What a project's lengths are measured in: metres or centimetres, saved with the project. Game engines differ: Godot and Unity count 1 unit as 1 metre, Unreal as 1 centimetre. The shapes are the same either way; only the numbers you read and type change. See [Units](pivots.md#units-metres-or-centimetres).

**Local vs. world space** — *World* directions are the same for the whole scene. *Local* directions follow the object's own rotation.

**Instance** — An object that shows another object's shape instead of having its own. Change the original and every instance changes too. See [Copies and instances](copies-and-instances.md).

**Symmetry (mirror modelling)** — Working on one half of a shape while the other half copies every change, mirrored across a plane through the middle. In this app the other half is a *live mirror*: a copy drawn and exported with the object, joined to your half where they meet, that follows every edit (other programs call it a mirror modifier). The mirror belongs to the object: its plane goes through the object's pivot and moves and turns with it. *Applying* the mirror makes the other half real geometry and switches symmetry off. See [Symmetry](symmetry.md).

## Viewing the scene

**View dice (view cube)** — A small cube in a corner of the 3D view that turns with the scene, so you always see which way you're looking. Clicking a face, edge or corner swings the view round to look from there; 3ds Max calls it the ViewCube. See [The view dice](navigating-the-view.md#the-view-dice).

**Orbit, pan and zoom** — The three ways to move the view. *Orbiting* turns the camera around a point, like walking around a statue. *Panning* slides the camera sideways or up and down without turning it. *Zooming* (strictly, *dollying*) moves the camera closer or farther. None of them change the model. See [Navigating the view](navigating-the-view.md).

**Perspective and flat (orthographic) views** — A *perspective* view shows the scene as your eye or a camera would: far things look smaller, and parallel lines meet in the distance. A *flat* view (the proper term is *orthographic*) drops that: things look the same size however far away they are, and parallel lines stay parallel, like a technical drawing or a floor plan. That makes flat views good for checking that edges line up, that two parts are the same height, or that a shape is symmetrical, because nothing is distorted. The **top**, **front** and **right** views are flat views looking straight down, from the front (along Y), and from the right (along X, back towards the origin). The **Orthographic** view is flat too but can be turned any way: press **P** in any view to switch between perspective and orthographic. See [Navigating the view](navigating-the-view.md#views-and-the-four-view-layout).

**Camera pivot** — The point the view turns around and moves towards. In this app it's normally the spot under the mouse when you start to orbit, pan or zoom, so you turn around and close in on whatever you point at; over empty space it is always something you can see. A small ring shows it while you orbit. (Not the same as an object's pivot, which belongs to the object.)

**Isolate** — Showing only the selection for a while, with everything else hidden, so you can work on it without distractions. See [Hide and isolate](hide-and-isolate.md).

## Shading and normals

**Normal** — An invisible arrow pointing out of a surface, telling the renderer which way the surface faces. Lighting is calculated from normals.

**Flipped normals** — Normals pointing inward. The surface looks inside-out or disappears in a game engine.

**Hard and soft edges (smoothing)** — A *soft* edge blends the shading of the two faces next to it, so they look like one smooth surface. A *hard* edge keeps a sharp light/dark break. Hard-surface models need the right edges marked hard, or curved and flat areas shade wrongly. Other programs may call this *smoothing groups* or *Phong* breaks. See [Hard and soft edges](hard-and-soft-edges.md).

**Smoothing by angle (auto-smooth)** — Shading an edge hard wherever its polygons meet more sharply than an angle, and soft everywhere else, instead of choosing edge by edge. In this app each object has its own smoothing: *smoothed by angle* (the standard, at 30°), worked out again after every change, or shaded only by the edges you mark hard. Change it on the object's Smoothing row in the Attributes panel, or with **Mesh → Shading → Smooth by angle**. See [Hard and soft edges](hard-and-soft-edges.md#smoothed-by-angle).

**Weighted normals** — A trick that makes large flat faces shade perfectly flat on low-poly bevelled models, without baking a normal map.

## UVs and texturing

**UV map** — A flattened copy of the mesh's surface, laid out on a square, telling the engine where each part of a texture goes. "U" and "V" are just the names of the 2D axes, like X and Y.

**Seam** — An edge where the surface is "cut" so it can be flattened. Every UV piece is bounded by seams. In hard-surface work, a good starting point is placing seams on hard edges. The ready-made shapes come with their seams marked where they're cut open; see [Seams](seams.md) to mark your own or have them suggested.

**Seam preference** — A note on an edge for the automatic UV tools: **prefer** (when a cut is needed, here first) or **never** (no cut here unless a piece couldn't lie flat otherwise). Not a seam itself. See [Seam preferences and stretch budgets](seam-preferences.md).

**Stretch budget** — How much stretching a piece may have once laid flat (1.5× means nearly all of it within one and a half times of perfect). Pieces over budget get cut further. See [Seam preferences and stretch budgets](seam-preferences.md).

**Overlapping pieces** — UV pieces lying over each other in one texture, so they share pixels: paint and bakes on one show on the other. The UV Editor tints them red. See [Seeing and checking UVs](seeing-and-checking.md).

**Layout snapshot** — A named copy of the UV layout (every polygon's UVs and the seams) kept in the project, to put back in one step or see faintly under the current one. See [Layout snapshots](layout-snapshots.md).

**Mirrored piece** — A UV piece lying flipped in the texture, so anything painted on it reads backwards on the model. The UV Editor tints it purple. See [Seeing and checking UVs](seeing-and-checking.md).

**Locked piece** — A UV piece no automatic tool changes (Unwrap, Pack, mending and the rest); you can still move it by hand. See [Locking pieces and piece settings](locked-pieces.md).

**Pin (UV pin)** — A UV nailed to its spot on the texture: unwrapping lays the rest of its piece out around it, and relaxing leaves it where it is. See [Pinning UVs](pins.md).

**UV piece (island, shell)** — A connected piece of the flattened surface. Other programs call it an *island* or a *shell*; this app says *piece*. Polygons are in the same piece when they share an edge whose UVs match on both sides.

**Corner (of a polygon)** — A point as one polygon uses it. UVs belong to corners, not points: a point on a seam has one UV in each piece it touches. Every shape in this app comes with UVs ([Primitives: ready-made UVs](primitives.md#ready-made-uvs)).

**Unwrapping** — Flattening the mesh along its seams into pieces ([Unwrap](unwrapping.md)). A piece can be laid flat in one go only if it is shaped like a **disk**: one piece, with one border all round (a cube's side, a sheet). A tube (two borders) or a ball (none) needs a cut first. A curved piece can't lie flat perfectly: an unwrap keeps either the shapes of its parts (squares stay square) or their sizes (pixels stay the same size), or a balance of the two (see **Unwrap method** in [Unwrap](unwrapping.md#settings)).

**Stretching (distortion)** — When a piece's shape on the UV map doesn't match its real shape, textures look squashed or stretched. A checker texture makes it easy to spot ([Checker and Stretch views](checker-and-stretch.md)).

**Packing** — Arranging all pieces to fill the UV square as efficiently as possible, with a gap (**padding**) between them so textures don't bleed into each other ([Packing](packing.md)).

**Texel density** — How many texture pixels cover a given real-world size, e.g. 512 pixels per metre. Keeping it consistent across an asset (and across a game) makes everything look equally sharp. See [Texture size and texel density](texture-and-density.md).

**Trim sheet** — A texture made of horizontal rows that repeat left to right (bolts, panel edges, seals). Long thin parts of many models are laid along its rows instead of getting texture of their own, so props share one texture and stay sharp. See [Trim sheets](trim-sheets.md).

**Mipmaps** — Smaller copies of a texture (half, a quarter, an eighth… the size) that a game shows when a surface is far away, so it doesn't shimmer. In a smaller copy each pixel covers several of the original's, so pieces that lie close together on the UV map blur into each other there. That's why packing leaves a gap of several pixels ([Check for baking](bake-check.md)).

## The game-asset pipeline

**Baking** — Transferring detail into textures, e.g. from a high-poly model into a normal map for a low-poly one. Substance Painter does this; clean UVs and correct hard edges are what make bakes come out right.

**Bake check** — A list of what in a model and its UVs would spoil a bake (polygons facing the wrong way, points on the same spot, bent n-gons, hard edges that aren't UV seams, overlapping UVs and so on), with fixes. See [Check for baking](bake-check.md).

**Normal map** — A texture that fakes surface detail by bending the normals, making a low-poly model look more detailed.

**Tangent space** — The coordinate system normal maps are stored in. The standard one is MikkTSpace, which Substance Painter and Godot both use; when the bake and the engine agree on it, normal maps look the same in both.

**Triangulation** — Splitting polygons into triangles. It needs to happen the same way every time, so the model you bake in Substance Painter matches the one in Godot exactly. The app cuts each polygon along lines that stay inside it: flat round polygons from one corner like a fan, polygons with dents round the dent, and [bent polygons](#mesh-basics) along their fold. The same polygon always gets the same cut.

**glTF** — An open 3D file format ("the JPEG of 3D") that both Godot and Substance Painter read.

**Export preset** — A ready-made set of export settings for one program: the kind of file, its units, which way is up, and whether polygons are cut into triangles. Picking the program under **Made for** sets them all at once, so the model arrives the way that program expects. The app remembers the one you last exported with. See [Presets](export-and-import.md#presets).

**Solis** — A game engine being made beside this app, by the same project. It reads glTF, so models go to it through the **Solis** export preset. See [Presets](export-and-import.md#presets).

**OBJ** — An old, plain-text 3D file format that almost every 3D program reads: points, polygons, normals and UVs, but no pivots or animation. See [Export and import](export-and-import.md).

**Export and import** — Writing your models into a file another program reads (export), and reading one in (import). The project file keeps everything the app needs to keep working; an export holds just the finished shapes. See [Export and import](export-and-import.md).

**Material** — What a surface is made of: painted metal, rubber, glass. In this app for now a material is a name and a colour; textures and the rest come with texturing. See [Materials](materials.md).

**Material slot / texture set** — Groups of polygons that get their own material. Each one becomes a separate texture set in Substance Painter, and a separate surface (with its own material) in Godot. In this app each polygon holds the number of its material in its object's list. See [Materials](materials.md).

**LOD (level of detail)** — Simpler versions of a model that the engine swaps in when it's far away.

**Collision mesh** — A simple, invisible shape the physics engine uses instead of the detailed visual model. In this app, each object's **Collision** setting tells Godot which to make. See [Kit export and collision](kit-export.md).

**Modular kit** — A set of pieces (walls, floors, doors) built to fit together and placed again and again to make a level. Each piece is exported as its own file with its pivot where it snaps. See [Kit export and collision](kit-export.md).
