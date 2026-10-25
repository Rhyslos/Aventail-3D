# 0151. Collision hulls: Wrapped in parts, and collision shapes drawn

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0078](0078-kit-export.md) (kit export and the Collision setting Godot reads from names), [0122](0122-solis-preset-and-remembered-export-settings.md) (Godot's names only in exports made for Godot), [0135](0135-live-booleans.md) and [0142](0142-modifier-stack-and-remesh.md) (live work kept in the scene, heavy work on another thread; Remesh's inside test), [0147](0147-curve-deform-and-shrinkwrap.md) (our own geometry rather than parry3d), [0053](0053-hard-and-soft-edges.md) (Display menu switches drawn over the shapes), [0110](0110-controllable-automation.md) (automatic tools show their settings and give the same result every time), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0150](0150-quadify.md) (the development file format moves to version 52)

## Problem

Phase 10's thirteenth milestone is the **collision hull**: simple convex shapes for physics, made automatically. A game engine collides cheaply with convex shapes (no dents), and much more slowly with an exact copy of a model. The Collision setting from decision 0078 already offers *The shape itself* (`-col`), *Wrapped (convex)* (`-convcol`, one hull Godot makes on import) and *A box*. What's missing:

- a shape with dents (an arch, a chair, a U-shaped wall, a ring) wrapped as one hull fills them in: the player can't walk through the arch. It needs **several hulls**, one per convex part (a *convex decomposition*);
- you can't see any of these shapes in the app, so you only find out what the player bumps into once you're in Godot.

The notes ask for a single **convex hull** and **convex pieces** (with *Max pieces*, recommended 8, and *Detail*), drawn as outlines when the object is selected (or always, with a display switch), and exported with Godot's collision names; tests that the hull holds every point, the piece count stays within the maximum, and the names; Godot's import check if it can be run.

Left to decide: whether this is a modifier or part of the Collision setting; how to make hulls and parts (a library or our own); what the choices and settings are called; how the parts are named in the file; when the outlines show; what happens with shapes that can't be split; saving.

## Options

A modifier, or the Collision setting:
1. **Two more ways of colliding, on the Collision setting (chosen).** Collision already says what the game engine collides with; the hulls are extra shapes beside the object, not a change to it. A modifier changes the shape you see and export, sits in the stack's order, and is made permanent by Bake into shape: none of that fits (baking a collision hull into the model would be a mistake, and its place in the stack would mean nothing).
2. A *Collision hull* modifier: one more palette button, but it would have to not change the shape it sits on, unlike every other modifier, and two places (the setting and the modifier) would say what the object collides as.

*Convex hull (one)*:
1. **It is the existing *Wrapped (convex)* (chosen),** now drawn. Godot makes exactly this hull from the `-convcol` name, so the file and older projects stay the same, and there aren't two choices meaning one thing.
2. A new *Convex hull* choice exporting our own hull: two near-identical entries in one list.

Making hulls and parts:
1. **Our own (chosen):** a quickhull (`mesh::hull`, about 500 lines) and a V-HACD-style split (`mesh::convex_parts` and its `split.rs`, about 700 lines), built on Remesh's inside test. Deterministic, no new dependency, our own measures and limits, and in keeping with 0147, which chose our own tree of boxes over parry3d.
2. parry3d's `convex_hull` and `VHACD` (Apache-2.0): proven, but nalgebra and a large tree to check and keep, conversions both ways, and its panics on some flat input to guard against.

How the split works (approximate convex decomposition):
1. **Split the worst part by a plane square to an axis, over and over (chosen),** as V-HACD does, measured on a voxel grid; each part's hull then made exactly from the shape's triangles cut to its box. Parts meet exactly on the plane between them. (Correction, 2026-10-24: that holds for hulls under the 64-corner limit. A limited hull used to drop corners of the cut at random, leaving a small gap between neighbours; it now keeps the same corners of each cut as its neighbour, up to 12 spread evenly round the section, so they meet there; a part cut on more than four sides is limited as a whole and may still leave a small gap.)
2. Voxel hulls only: quicker, but the hulls are a half voxel too big, and neighbours overlap or leave gaps.
3. Hierarchical clustering of the surface (as in some engines): no inside, so it splits thin walls and hollows badly.

Names in Godot's file:
1. **`name-part1-convcolonly`, `name-part2-convcolonly`, … beside the object (chosen),** as *A box* writes `name-convcolonly` beside it: Godot turns each into a collision shape and doesn't draw it. Each part a node of its own, because Godot makes **one** convex shape from all of a mesh's points.
2. The parts as children of the object: different from the box, and nothing gained.

## Decision

### The choices

`scene::Collision` gets **`Parts`**, named **Wrapped in parts (convex)**, listed after *Wrapped (convex)*: "Collides with a few wrapped parts that follow the shape's dents: nearly as cheap as Wrapped, and things fit under arches and into hollows". Its settings are `Object::collision_parts: ConvexPartsOptions`, kept when another collision is chosen (so choosing it again brings them back):

- **Max parts** (standard **8**, 1 to 32);
- **Detail** (standard **50%**, 0 to 100%): how small a dent is still followed. It sets the grid (32 to 96 voxels across the shape's longest side) and the share of the whole volume a part may waste and not be split (20.5% at 0, 5.5% at 50%, 0.5% at 100%).

`Command::SetCollisionParts { ids, options }` (clamped; groups left alone; `Edit::SetCollisionParts`) is its undo step, "Change the collision parts". In the Attributes panel, **Max parts** and **Detail** show under Collision when it is *Wrapped in parts*, a drag one undo step.

The other choices are unchanged: *The shape itself* (`-col`), *Wrapped* (`-convcol`, Godot's own hull of every point) and *A box* (`name-convcolonly` beside it).

### Hulls (`mesh::hull`, layer 0)

`convex_hull(points)` and `convex_hull_limited(points, max_corners)`: quickhull in double precision, points at the same place counted once, a point within a billionth of the points' size of a triangle counting as on it. A hull's corners where fewer than three sides of different directions meet (on a flat side or a straight edge, left from a rougher hull) are dropped and the hull built again, so a box is always its 8 corners. A limited hull stops once it has that many corners: the farthest point always goes first, so it is the best few-cornered hull of that kind, a little inside the full one. `Hull::outline` gives its edges without the diagonals of flat sides (a box's 12). Points in a line or plane are refused (`HullProblem::Flat`), none at real places too (`Empty`); never a panic. Rounding that would make a step impossible leaves that point out.

### Parts (`mesh::convex_parts`, layer 0)

`convex_parts(mesh, options) -> Result<Vec<Hull>, PartsProblem>`:

1. **Closed only** (Remesh's `is_closed`): an open shape has no inside (`NotClosed`).
2. **Voxels:** a grid over the shape, inside or outside by Remesh's winding-number test (overlapping pieces are one solid).
3. **Waste:** a part is the shape inside a box (sides only where it was cut). Its hull is made from the shape's triangles cut to the box (Sutherland–Hodgman; a triangle only touching a side, or lying in it facing the wrong way, adds nothing), plus the box's corners inside the shape. Its waste is that hull's volume less its own (its voxels').
4. **Splitting:** while there are fewer parts than Max parts, the one with most waste is cut in two, if its waste is more than Detail's share. Six evenly spaced places along each axis are tried, and the three where its slices change most (a box standing on another changes there), then places closer and closer round the best. Each is scored by the halves' waste as their voxels show it, plus a little for uneven halves (V-HACD's balance: 0.1 of the volume difference) and for the size of the cut (its area times 5% of the part's longest side): of cuts that save about as much, one near the middle through a thin place goes first, so a ring is cut across its band, not along it or by slicing slivers off. A cut lands on a point of the shape between the two voxels' middles when there is one, so it runs along a box's side rather than through it.
5. **Joining:** parts whose joint hull wastes no more than the share are joined back, least wasteful first (cutting one part at a time can leave a convex piece cut in two, as the base of a U).
6. **Hulls:** each part's hull, at most **64 corners** (`MAX_HULL_CORNERS`: V-HACD's standard, and well within Jolt's 256 in Godot).

A shape the grid doesn't see inside (thinner than a voxel) is one hull; Max parts 1 too. Deterministic. Timings (release build): a 384-polygon ring split into 8 parts in about 0.35 s; a 32 768-polygon sphere (one part) 0.07 s; a U of three boxes 0.02 s.

### Collision shapes in the scene (`scene::collision`)

`collision_shapes(mesh, collision, parts, offset) -> CollisionShapes { hulls, problem }`: none for *None* and *The shape itself*; one hull of every point for *Wrapped*; the box square to the pivot for *A box* (as the export writes it); the parts for *Wrapped in parts*. A shape that can't be split collides as **one wrapped hull** (64 corners at most) and says so: "Not split into parts: the shape isn't closed… It collides as one wrapped shape instead." A flat shape has none: "No collision shape: the shape is flat…".

The scene keeps each object's last shapes with what they were made from (`CollisionCache`, not part of what the scene is, forgotten when the object goes): `Scene::collision_state(id, shape)` is `Ready` or `Needed(CollisionJob)`, `CollisionJob::run` works on any thread, `Scene::store_collision` keeps the result, `Scene::collision_shapes` works it out on the spot (exports), and `Scene::collision_problem(id)` gives the Attributes panel its note. Splitting into parts always counts as heavy (`COLLISION_IN_BACKGROUND_FROM`, 50 000); wrapping from about 12 500 polygons.

### Drawn

The selected objects' collision shapes are outlined over the view in warm yellow, thin, under the other guides (`OverlayStyle::Collision`): *Wrapped*, *Wrapped in parts* and *A box*; *The shape itself* is the shape. **Display → Show collision** (also in Settings → Viewport, under Viewport display, and command search; off as standard, remembered) outlines every shown object's. They are worked out from the shape as the viewport draws it (modifiers shown in the viewport and the mirror included, polygons hidden with H left out), by the component overlay (`ui/src/components/collision.rs`): light work at once, heavy work on another thread, one job per object, the last outline shown meanwhile, the window redrawing until it is in. In amber under the Basic section, the reason when the shapes aren't what the setting asks for.

### Exported

With Godot's names (the Godot preset, decision 0078), an object *Wrapped in parts* goes out under its own name, with each part beside it as `name-part1-convcolonly`, `name-part2-convcolonly`… at its pivot, made from its exported shape (every modifier that is on). Other exports (Solis, Unity, Unreal, OBJ) carry no parts, as with the other collision names. Godot 4.7.2 imports the check file (an arch and a ring in parts) as 2 meshes and 12 collision shapes; `tools/gltf-check/check-scenes.gd` now checks it.

### Saving

Development file format **version 52**: version 51's model with, in each object's development settings (`v39::ObjectV39`), `collision_parts` (`v52::CollisionPartsV52`: whether chosen, Max parts, Detail), written when chosen or not the standard. Version 20's collision list holds an object in parts as *Wrapped*, its closest older choice. Settings are clamped when read; parts chosen for an object with no shape are refused as damaged. Files before 52 hold none.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **Part of the Collision setting, not a modifier**: no palette button and no Mesh menu entry; it's chosen in Attributes → Basic → Collision.
- The single convex hull **is the existing *Wrapped (convex)***, now drawn; no second "Convex hull" choice.
- The new choice is called **Wrapped in parts (convex)**, with **Max parts** and **Detail**, rather than the notes' "Convex pieces" and "Max pieces": *pieces* already means UV pieces in this app (decision 0046), and *Wrapped* matches the choice above it.
- Standard **Max parts 8** (up to 32) and **Detail 50%**; each part at most **64 corners**.
- A shape that **can't be split** (open) **collides as one wrapped hull** and says why, rather than having no collision.
- Parts are named **`name-part1-convcolonly`** and on, beside the object, like the box.
- Outlines: **warm yellow**, for the **selected objects**, and every object with **Display → Show collision** (off as standard).
- **No new keys or mouse actions.**

## Consequences

- Projects with an object wrapped in parts (or changed parts settings) need this version of the app or later.
- Kits exported for Godot with *Wrapped in parts* import with a convex collision shape per part, and no work in Godot.
- `mesh::hull` and `mesh::convex_parts` are there for later work: LODs' and Check for baking's "is it convex?", and Unreal's `UCX_` collision when other engines get collision names.
- Not yet: collision names for Unity and Unreal Engine; a capsule or sphere collision; choosing the cut planes by hand.
