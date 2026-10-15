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

**Selecting through** — Selecting with a box, lasso or brush can take only what faces you (*visible only*), or everything inside the outline, including the far side of the shape hidden behind it (*through*). The app selects through unless you switch it off: see [Selecting](selecting.md).

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

**Bevel and chamfer** — Cutting a sharp edge away and putting a strip of polygons in its place. A *chamfer* is a single flat strip; a *bevel* with more segments is rounded. A *cove* is a bevel that curves inward instead, hollowing the edge out. Real objects rarely have perfectly sharp edges, so a small bevel catches the light and makes a model read as solid. See [Bevel](bevel.md).

**Manifold** — A mesh that could exist as a real, solid object: every edge is shared by exactly two polygons. Non-manifold geometry (edges shared by three faces, loose edges, holes) often breaks booleans, bakes and some tools.

## Objects and scenes

**Object** — Anything listed in the Objects panel: a mesh, a primitive, a group. See [The Objects panel](objects.md).

**Group** — An empty object whose only job is to hold other objects, so they can be moved, hidden or exported together. (Some programs call it a *null* or an *empty*.) Put the parts of one asset in a group and give it the asset's name.

**Mode** — What kind of thing your tools work on: whole objects, or a mesh's points, edges or polygons, or its UVs. The same Move tool moves a whole object in Model mode and a single edge in Edges mode. See [Interface tour](interface-tour.md#modes).

**Tool menu** — The menu that opens at the mouse when you press **E**, holding the tools for what you're working on: object tools in Model mode, modelling tools while editing points, edges or polygons. Shown as a ring (a *radial* or *pie* menu) of icons, whose places never move, or as a list. See [The tool menu](tool-menu.md).

**Primitive** — A ready-made shape (cube, cylinder, sphere…) built from a few settings, like size and segments, which stay editable. In this app every primitive stands on the ground with its pivot at the middle of its base. See [Primitives](primitives.md).

**Project file (.avtl)** — The file your scene is saved in: every object, its settings and edits, and what was selected. It's this app's own working format; models go to other programs by *exporting* them instead. See [Saving and opening projects](saving-and-opening.md).

**Backup copy (autosave)** — A copy of your scene the app saves by itself in the background, every few minutes while you have unsaved changes, so a crash or power cut costs at most those minutes. Backup copies are kept in the app's own folder, never in place of your project file, and are offered back the next time the app starts if it didn't close normally. See [Backup copies and crash recovery](autosave.md).

**Base and edits** — A shape in this app is kept in two parts: its *base*, what it starts from (a primitive's settings, or a plain mesh), and the *edits* recorded on top of it, such as moving some points. The final shape is the base with the edits applied. Because the two are kept apart, a primitive's settings stay editable after you've modelled on it: change a cube's height and your edits are made again on the taller cube. Moving, rotating and scaling points, edges and polygons records edits like this ([Move, rotate and scale](transforming.md)). Each edit also remembers *where* its points were, so when new settings renumber the points (more segments, say), it finds them again by place ([Changing settings after editing](transforming.md#changing-settings-after-editing)).

**Editable mesh (Make editable)** — Turning a shape into a plain mesh with no settings: its base and edits are baked together, so what you see becomes the new starting point. Do it when you no longer need the settings, or before editing a mesh freely. Undo brings the settings back. See [Objects](objects.md#make-editable).

**Hierarchy** — Objects can be children of other objects (sit *inside* them, in the Objects panel's tree). Moving a parent moves its children with it, and hiding it hides them too.

**Undo history** — The list of changes you've made, kept so they can be taken back (undo, `Ctrl + Z`) and done again (redo, `Ctrl + Y`). This app keeps steps while it's open, up to a memory limit (2 GB as standard, set in **Edit → Settings… → Undo**), then forgets the oldest first. There's no limit on the *number* of steps. See [Objects](objects.md).

**Pivot (axis / origin)** — The point an object rotates and scales around. In Godot, it's also the point the asset is placed and snapped by, so where you put it matters: on the hinge of a door, at the bottom corner of a wall piece. The [gizmo](transforming.md) sits on it. In the Object mode you can move and turn it without moving the shape; see [Pivots and the Object mode](pivots.md).

**Gizmo** — The handle drawn on what's selected while moving, rotating or scaling: arrows, rings and boxes in the axis colours. Dragging one part changes the selection in that direction only. See [Move, rotate and scale](transforming.md).

**Snapping** — Making a move, turn or resize jump to exact places (a point, an edge, the grid) or steps (15°, 10%) instead of going wherever the mouse is. See [Move, rotate and scale](transforming.md#snapping).

**World and object axes** — The *world's* axes are the same everywhere: Z is always up. An *object's own* axes turn with it: a cube turned on its side has its own Z pointing sideways. The gizmo can follow either (press `L`).

**Axes (X, Y, Z)** — The three directions of 3D space, always coloured X red, Y blue, Z green. (Most 3D programs colour Y green and Z blue; this app swaps them, so blue is the second ground direction. The letters are always shown too.) In this app **Z points up**, and X and Y lie flat on the ground; a green line rising from the origin marks Z. Some 3D programs, Godot and the glTF file format use Y for up instead; the app converts when it exports, so models arrive standing upright.

**Origin and grid** — The origin is the point where X, Y and Z are all zero: the centre of the world, where the red and blue axis lines cross. The grid on the ground helps you judge sizes: in this app its lines are 1 m apart, and you can add fainter lines every 50 cm (cutting each square into four) or every 10 cm when you need them. It's drawn around the point you're looking at and fades out at its edges, so it reads as a work surface rather than an endless floor. Lengths are shown in metres as standard, or centimetres if you prefer; 1 m is 100 cm.

**Units** — What a project's lengths are measured in: metres or centimetres, saved with the project. Game engines differ: Godot and Unity count 1 unit as 1 metre, Unreal as 1 centimetre. The shapes are the same either way; only the numbers you read and type change. See [Units](pivots.md#units-metres-or-centimetres).

**Local vs. world space** — *World* directions are the same for the whole scene. *Local* directions follow the object's own rotation.

**Instance** — An object that shows another object's shape instead of having its own. Change the original and every instance changes too. See [Copies and instances](copies-and-instances.md).

**Symmetry (mirror modelling)** — Working on one half of a shape while the other half copies every change, mirrored across a plane through the middle. See [Symmetry](symmetry.md).

## Viewing the scene

**Orbit, pan and zoom** — The three ways to move the view. *Orbiting* turns the camera around a point, like walking around a statue. *Panning* slides the camera sideways or up and down without turning it. *Zooming* (strictly, *dollying*) moves the camera closer or farther. None of them change the model. See [Navigating the view](navigating-the-view.md).

**Perspective and flat (orthographic) views** — A *perspective* view shows the scene as your eye or a camera would: far things look smaller, and parallel lines meet in the distance. A *flat* view (the proper term is *orthographic*) drops that: things look the same size however far away they are, and parallel lines stay parallel, like a technical drawing or a floor plan. That makes flat views good for checking that edges line up, that two parts are the same height, or that a shape is symmetrical, because nothing is distorted. The **top**, **front** and **right** views are flat views looking straight down, from the front (along Y), and from the right (along X, back towards the origin). The **Orthographic** view is flat too but can be turned any way: press **P** in any view to switch between perspective and orthographic. See [Navigating the view](navigating-the-view.md#views-and-the-four-view-layout).

**Camera pivot** — The point the view turns around and moves towards. In this app it's normally the spot under the mouse when you start to orbit or zoom, so you turn around and close in on whatever you point at; panning carries it along. (Not the same as an object's pivot, which belongs to the object.)

**Isolate** — Showing only the selection for a while, with everything else hidden, so you can work on it without distractions. See [Hide and isolate](hide-and-isolate.md).

## Shading and normals

**Normal** — An invisible arrow pointing out of a surface, telling the renderer which way the surface faces. Lighting is calculated from normals.

**Flipped normals** — Normals pointing inward. The surface looks inside-out or disappears in a game engine.

**Hard and soft edges (smoothing)** — A *soft* edge blends the shading of the two faces next to it, so they look like one smooth surface. A *hard* edge keeps a sharp light/dark break. Hard-surface models need the right edges marked hard, or curved and flat areas shade wrongly. Other programs may call this *smoothing groups* or *Phong* breaks. See [Hard and soft edges](hard-and-soft-edges.md).

**Auto-smooth (hard edges by angle)** — Marking every edge hard where its polygons meet more sharply than an angle, and the rest soft, instead of choosing edge by edge. This app does it for every shape after every change while **Automatic hard edges** is on (the standard; Mesh menu). See [Hard and soft edges](hard-and-soft-edges.md#automatic-hard-edges).

**Weighted normals** — A trick that makes large flat faces shade perfectly flat on low-poly bevelled models, without baking a normal map.

## UVs and texturing

**UV map** — A flattened copy of the mesh's surface, laid out on a square, telling the engine where each part of a texture goes. "U" and "V" are just the names of the 2D axes, like X and Y.

**Seam** — An edge where the surface is "cut" so it can be flattened. Every UV island is bounded by seams. In hard-surface work, a good starting point is placing seams on hard edges.

**UV island (shell)** — A connected piece of the flattened surface.

**Unwrapping** — Flattening the mesh along its seams into islands.

**Stretching (distortion)** — When an island's shape on the UV map doesn't match its real shape, textures look squashed or stretched. A checker texture makes it easy to spot.

**Packing** — Arranging all islands to fill the UV square as efficiently as possible, with a gap (**padding**) between them so textures don't bleed into each other.

**Texel density** — How many texture pixels cover a given real-world size, e.g. 512 pixels per metre. Keeping it consistent across an asset (and across a game) makes everything look equally sharp.

## The game-asset pipeline

**Baking** — Transferring detail into textures, e.g. from a high-poly model into a normal map for a low-poly one. Substance Painter does this; clean UVs and correct hard edges are what make bakes come out right.

**Normal map** — A texture that fakes surface detail by bending the normals, making a low-poly model look more detailed.

**Tangent space** — The coordinate system normal maps are stored in. The standard one is MikkTSpace, which Substance Painter and Godot both use; when the bake and the engine agree on it, normal maps look the same in both.

**Triangulation** — Splitting polygons into triangles. It needs to happen the same way every time, so the model you bake in Substance Painter matches the one in Godot exactly. The app cuts each polygon along lines that stay inside it: flat round polygons from one corner like a fan, polygons with dents round the dent, and [bent polygons](#mesh-basics) along their fold. The same polygon always gets the same cut.

**glTF** — An open 3D file format ("the JPEG of 3D") that both Godot and Substance Painter read.

**Material slot / texture set** — Groups of polygons that get their own material. Each one becomes a separate texture set in Substance Painter.

**LOD (level of detail)** — Simpler versions of a model that the engine swaps in when it's far away.

**Collision mesh** — A simple, invisible shape the physics engine uses instead of the detailed visual model.
