# 0146. Scatter onto surface

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack, its recipe and its sources), [0144](0144-lattice-cage.md) (the first modifier reading other objects), [0145](0145-decimate.md) (the latest modifier), [0135](0135-live-booleans.md) (the selection order of booleans, cutters as objects of their own, materials matched by name), [0068](0068-pivot-editing-and-scene-units.md) (pivots), [0072](0072-gltf-export.md) (the glTF writer), [0028](0028-ask-about-controls.md) (ask about controls), [0035](0035-heavy-meshes.md) (heavy work off the interface thread), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0145](0145-decimate.md) (the development file format moves to version 47)

## Problem

Phase 10's eighth milestone is Scatter onto surface: select the surface first, then Ctrl + click the shapes to scatter (the booleans' order rule: the first selected carries the modifier), and "Scatter onto surface" from the Mesh menu, the palette or command search spreads copies of them over it. Settings: place on points, polygon centres or at random over the surface, with Count, Seed and spacing (a least distance); Align to surface, Random turn (degrees round the normal), Random scale (smallest to largest) and Offset along the normal; and only the selected polygons of the surface, kept as a polygon set. The scattered shapes stay their own objects, as cutters do, and hidden ones still scatter. The result is the surface plus its copies, merged on export as one mesh, with an option "Export copies as instances" if glTF instancing (`EXT_mesh_gpu_instancing`) is easy and Godot reads it.

It is the second modifier that reads other objects (after the lattice), and the first that reads **several**, and reads their **shapes**, so it must reuse the stack's sources: deleting a source, copies following copies, and refusing loops (a shape scattered onto itself, A onto B onto A).

Left to decide: where a copy stands and which way it faces, how the random choices are made, spacing, what happens when one of several sources is deleted, materials, the polygon set, the cost estimate, the controls, saving, and instancing on export.

## Options

Where a copy stands:
1. **On its pivot, as the shape stands in the world, only moved (chosen):** each copy is the source's final shape with its own turn and size as it stands (as seen through the surface's space), moved so the source's pivot is on the spot. A primitive's pivot is the middle of its base (decision 0068), so copies stand on the surface rather than half sunk. Rotating or scaling the source turns or sizes every copy; moving it changes nothing. This is how Blender's instancing on points treats the instanced object's own rotation and scale.
2. On the shape's centre: a rock would be half buried, and every source would need an Offset.
3. On its place relative to the surface (as cutters are): the copies would all sit off to one side by wherever the source happens to be.

Which way is up:
1. **The shape's up (Z, the app's up) turned onto the surface's outward direction, with Align to surface (chosen, on as standard):** a rock lies on a slope, a spike stands out of a ball. *Random turn* is then round that direction. Off, every copy stays as the shape stands, and Random turn is round Z. The offset follows the surface's direction either way.
2. Aligning the shape's own local axis: the same for shapes that aren't rotated, but a shape rotated in the world would stand at an unexpected angle once aligned.

The random choices:
1. **A small SplitMix64 stream per copy, keyed by the seed and the point or polygon id the copy stands on (or its place in the random spread) (chosen):** the same everywhere, raising Count adds copies without moving the others, and editing one part of the surface leaves the copies on the rest where they were. The random spread itself is one stream from the seed.
2. One stream for everything: changing Count would reshuffle every copy's turn and size.

Spacing:
1. **Rejection against a grid of cells the size of the spacing (chosen):** a spot closer than the spacing to one already taken is skipped; a random spread tries up to thirty times a copy, so a spacing too large gives an even spread with fewer copies. Points and polygon centres are thinned in order.
2. True Poisson-disc sampling on the surface (Bridson): more even, but much more machinery for a surface of many polygons; the rejection result looks the same for scattered props.

When one of several sources is deleted:
1. **That shape is taken out of the scatter; the last one deleted ends the modifier (chosen),** in the same undo step. A scatter of rocks and pebbles keeps its pebbles when the rocks go.
2. The whole modifier ends (as a lattice does with its cage): one deleted pebble would lose the whole scatter.

Instancing on export:
1. **Always merged into one mesh (chosen).** Godot's glTF importer (`modules/gltf`, master as of 2026-10-23) lists the extensions it reads (`GODOT_single_root`, `KHR_animation_pointer`, `KHR_lights_punctual`, `KHR_materials_emissive_strength`, `KHR_materials_pbrSpecularGlossiness`, `KHR_materials_unlit`, `KHR_node_visibility`, `KHR_texture_transform`, plus document extensions for KTX and WebP textures): `EXT_mesh_gpu_instancing` isn't among them. An optional extension Godot doesn't know is skipped, so a file written with it would open in Godot with **one** rock at the node instead of fifty, and Substance Painter ignores it too. The writer could emit it cheaply (one mesh, a node with `TRANSLATION`, `ROTATION` and `SCALE` accessors), but an option whose result is wrong in the target engine is worse than none. Merging is what you see, and what Godot and Substance Painter both read.
2. Instances as an option: cheap to write, but broken in Godot today. Worth adding when Godot reads the extension; the copies' matrices are already worked out in one place (`mesh::scatter::spots`, `Spot::matrix`), so the writer would only need them.
3. One node per copy in the glTF, sharing a mesh: Godot reads that, but a thousand nodes is slow to import and edit, and the copies would no longer be one game object with the surface.

## Decision

### The work (`mesh::scatter`, layer 0)

- **`ScatterOptions`** (`Copy`, `Default`, `clamped`): **Place on** (`PlaceOn::Points`, `Centres`, `Random`, standard Random), **Count** (standard **50**, 1 to 100 000), **Seed** (standard 1), **Spacing** (standard 0, up to 1 km), **Align** (standard on), **Random turn** (standard **0°**, 0 to 360), **Smallest** and **Largest size** (standard 100%, 1% to 100 times, swapped if out of order) and **Offset** (standard 0, ±1 km). Values that aren't numbers become the standard ones.
- **`spots(surface, only, options, shapes)`**: the spots copies go on. *Points*: each point of the chosen polygons, facing the sum of their directions there. *Polygon centres*: each polygon's middle, facing its direction (Newell's normal, sensible for n-gons). *Random*: triangles weighted by area, picked through a running total, an even spot inside each. Polygons with a point that isn't a number are skipped, triangles with no area aren't picked. Each spot gets its turn, size and shape (an index into the shapes) from its own stream.
- **`scatter(surface, only, shapes, options, most)`**: the surface with each copy appended (`parts::append`: the surface keeps its polygons and ids, copies get new ones after them; UVs, seams, creases, hard edges, materials and other layers come along; a mirroring matrix turns copies the right way round). Each copy's matrix is `translate(spot + normal × offset) · stand · turn · scale` times the shape's own matrix into the surface's space with its pivot at the origin (`ScatterShape::matrix`).
- **Refused, with a plain reason:** no shapes, a surface with no polygons, chosen polygons none of which are left ("choose them again, or use the whole surface"), no area for a random spread, more than 100 000 copies (points or centres: "use Random, or a larger Spacing"), or a result over `most` polygons (`MAX_SHAPE_POLYGONS`, 30 million).
- `parts::append` no longer copies a whole layer of the result to see whether it has it: appending is now linear in the polygons added, instead of growing with the result (1 000 copies went from 481 ms to 32 ms).

### In the scene

- **`ModifierKind::Scatter(ScatterSettings)`**, family **Generate**, after Subdivide on the palette and in Mesh → Modifiers. `ScatterSettings { options, shapes, polygons }`: the shapes (each once, at most 64) and the chosen polygons by id (each once, in order; empty for the whole surface).
- **Sources:** `sources()` is the shapes; `takes_source` any object with a shape. `Scene::check_modifiers` refuses a shape that is the object itself or depends on it (`shape_uses`, following booleans, instances and modifier sources: A onto B onto A, an instance of the surface), a group or a cage, and (new) a modifier that needs sources and names none (`ModifierKind::needs_sources`, `ModifierRefusal::NoSource`; the Lattice is the other kind).
- **`SourceShape`** now carries the source's **pivot** in the object's space and its **materials** renumbering (each slot of the source matched by name to the surface's, the first where there's no match, as a cutter's are, decision 0135). Both are cheap to compare, so the cache redoes the step when either changes, and the meshes aren't copied each frame.
- **Deleting** a source goes through `ModifierKind::without_sources`: a scatter loses those shapes, and goes when none are left; a lattice goes with its cage; others are kept. **Copies** made with their sources scatter the copies (`with_sources_renamed`).
- **Hidden shapes still scatter**: the source's final shape is read whether it is shown or not. They are exported as objects of their own only when shown, as any shape; cutters are left out of exports because they are drawn as outlines, but a scattered shape is drawn as itself, so what you see is what is exported.
- **Cost:** `ModifierKind::work_with(input, purpose, sources)` (new; `StackJob::work` uses it) estimates a scatter by the polygons it could add (`polygons_added`: the copies it could make times the largest source's polygons) over 8, so from about 40 000 new polygons the viewport works it out on another thread, as booleans are (decision 0035). The other kinds keep `work`.
- **The chosen polygons** are polygon ids of the shape as modelled (the Polygons mode's), so they survive edits elsewhere; a modifier before Scatter that renumbers polygons (Remesh, Decimate) leaves none of them, which is said in the Attributes panel.

### In the interface

- **Scatter onto surface** is `CommandId::Scatter`, in `OFF_THE_RING` and `modifiers::COMMANDS`; named for what it does with the selection, as Add lattice is. `modifiers::scatter_plan` takes the shape selected first as the surface and the others, in the order they were selected, as the shapes; it says what's missing ("Select the surface first, then Ctrl + click the shapes to scatter over it"; "Ctrl + click one or more shapes to scatter: the shape selected first is the surface"), and why a loop can't be made. The palette's button, the menu entry and command search use it. Icon: three small cones standing out of a curved hill, each leaning with the ground (drawn in code).
- **The Attributes panel** (`attributes/scatter_rows.rs`): **Shapes** (their names, with **Select shapes**), **Place on**, **Count** (Random only, a large-number field), **Seed**, **Spacing**, **Align to surface**, **Random turn**, **Smallest size**, **Largest size**, **Offset**, and **On polygons**: "Whole surface" or "*N* chosen", with **Use selected** (the polygons selected on this shape in the Polygons mode; greyed out elsewhere, saying how) and **Whole surface**. One drag is one undo step.
- **No new keys or mouse actions.**

### Saving

Development file format **version 47**: version 46's model with `KindV43::Scatter { place_on, count, seed, spacing, align, random_turn, scale_min, scale_max, offset, shapes, polygons }` (place on as a number, anything unknown read as Random; settings clamped on reading; `polygons` `#[serde(default)]`). The shapes are checked as any source when the modifiers are put back: a scatter naming nothing, a missing object, itself or a loop is refused as damaged. New sample `sample-v47.avtl`; files before 47 never hold it.

### Timings

Release build, a sphere of 2 048 polygons as the surface, a rock of 72 polygons, random turns and sizes: 1 000 copies 54 ms, 10 000 copies 0.40 s (with a spacing of 10 cm, 0.39 s), 100 000 copies (7.2 million polygons) 4.9 s. About half a microsecond a polygon added.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** The palette button, **Mesh → Modifiers → Scatter onto surface** (under *Generate*) and command search; the surface clicked first, the shapes Ctrl + clicked after, as for booleans.
- The name **Scatter onto surface** for the command and **Scatter** for the modifier.
- **Random over the surface** as the standard placing, with **Count 50**; **Seed 1**; **Spacing 0**.
- **Align to surface on**; **Random turn 0°** (all copies face the same way until you ask; 360° is suggested in the manual for rocks); **sizes 100% to 100%**; **Offset 0**.
- Copies **stand on the source's pivot**, keep the source's own turn and size, and ignore where it is.
- **Several shapes:** each copy picks one at random (evenly). Running the command again adds **another** Scatter modifier rather than adding shapes to the last one.
- Deleting one of several shapes **takes it out**; deleting the last **ends the modifier**.
- Scattered shapes are **left as they are** (not hidden for you): shown, they also export on their own; hide them to leave them out.
- **Materials matched by name**, the surface's first where there's no match.
- The chosen polygons are set with **Use selected** in the Attributes panel (Polygons mode), not taken from the selection when the command is run.
- **No "Export copies as instances" option:** copies are always merged, because Godot doesn't import `EXT_mesh_gpu_instancing` (see *Options*).

## Consequences

- Projects with Scatter need this version of the app or later.
- `SourceShape` has a `pivot` and `materials`; a later modifier reading shapes (shrinkwrap, curve deform's curve) gets both for free.
- `ModifierKind::without_sources` decides what deleting a source does, kind by kind; `needs_sources` refuses a modifier that reads nothing; `work_with` lets a kind's cost depend on its sources.
- When Godot reads `EXT_mesh_gpu_instancing`, an export option can write each scatter's spots as instances of its shapes (from `spots` and `Spot::matrix`), keeping big scatters small.
- Not yet: weights for how often each shape is picked, density painted or taken from a vertex colour, copies kept off each other's bounds (spacing is spot to spot), and scattering onto curves.
