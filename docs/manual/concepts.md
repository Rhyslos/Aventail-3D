# Concepts

Plain-language explanations of the ideas Aventail 3D is built around. Other pages link here instead of re-explaining.

## Mesh basics

**Vertex (point)** — A single position in 3D space. Everything else is built from these.

**Edge** — A straight line between two vertices.

**Polygon (face)** — A flat surface enclosed by edges. Aventail 3D works with:
- **Triangles** (3 sides): what game engines and GPUs actually render. Everything gets triangulated on export.
- **Quads** (4 sides): the easiest to work with while modelling. Loop and ring tools rely on them.
- **N-gons** (5+ sides): fine on flat surfaces in hard-surface work, but they can triangulate unpredictably on curved ones and cause shading errors.

**Topology** — How the vertices, edges and polygons are connected, as opposed to where they sit. Two meshes can have the same shape but different topology.

**Edge loop** — A chain of edges running through quads, end to end, like a belt around the mesh. **Edge ring** — the edges running *across* a loop, like the rungs of a ladder.

**Manifold** — A mesh that could exist as a real, solid object: every edge is shared by exactly two polygons. Non-manifold geometry (edges shared by three faces, loose edges, holes) often breaks booleans, bakes and some tools.

## Objects and scenes

**Object** — Anything listed in the Objects panel (Cinema 4D calls it the Object Manager): a mesh, a primitive, a group.

**Mode** — What kind of thing your tools work on: whole objects, or a mesh's points, edges or polygons, or its UVs. The same Move tool moves a whole object in Model mode and a single edge in Edges mode. See [Interface tour](interface-tour.md#modes).

**Primitive** — A parametric shape (cube, cylinder…) whose settings, like size and segments, stay editable.

**Hierarchy** — Objects can be children of other objects. Moving a parent moves its children with it.

**Pivot (axis / origin)** — The point an object rotates and scales around. In Godot, it's also the point the asset is placed and snapped by, so where you put it matters.

**Axes (X, Y, Z)** — The three directions of 3D space, always coloured X red, Y green, Z blue. In this app **Z points up**, and X and Y lie flat on the ground, as in 3ds Max and Blender. Cinema 4D, Godot and the glTF file format use Y for up instead; the app converts when it exports, so models arrive standing upright.

**Local vs. world space** — *World* directions are the same for the whole scene. *Local* directions follow the object's own rotation.

## Shading and normals

**Normal** — An invisible arrow pointing out of a surface, telling the renderer which way the surface faces. Lighting is calculated from normals.

**Flipped normals** — Normals pointing inward. The surface looks inside-out or disappears in a game engine.

**Hard and soft edges (smoothing)** — A *soft* edge blends the shading of the two faces next to it, so they look like one smooth surface. A *hard* edge keeps a sharp light/dark break. Hard-surface models need the right edges marked hard, or curved and flat areas shade wrongly. (3ds Max calls this *smoothing groups*; C4D calls it *Phong* breaks.)

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

**Triangulation** — Splitting polygons into triangles. It needs to happen the same way every time, so the model you bake in Substance Painter matches the one in Godot exactly.

**glTF** — An open 3D file format ("the JPEG of 3D") that both Godot and Substance Painter read.

**Material slot / texture set** — Groups of polygons that get their own material. Each one becomes a separate texture set in Substance Painter.

**LOD (level of detail)** — Simpler versions of a model that the engine swaps in when it's far away.

**Collision mesh** — A simple, invisible shape the physics engine uses instead of the detailed visual model.
