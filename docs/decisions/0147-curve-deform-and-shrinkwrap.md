# 0147. Bend along curve and Shrinkwrap

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack, its recipe and its sources), [0144](0144-lattice-cage.md) (the deformer groundwork, a source that isn't a shape), [0146](0146-scatter.md) (the latest modifier: the selection order, sources, loops refused, chosen elements kept by id), [0138](0138-curves-sweep-lathe.md) and [0139](0139-spline-editing.md) (curves, and the rotation-minimising frames a sweep follows), [0135](0135-live-booleans.md) (the booleans' selection order), [0068](0068-pivot-editing-and-scene-units.md) (pivots), [0028](0028-ask-about-controls.md) (ask about controls), [0035](0035-heavy-meshes.md) (heavy work off the interface thread), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0146](0146-scatter.md) (the development file format moves to version 48)

## Problem

Phase 10's ninth milestone is two modifiers that lay a shape onto another object:

- **Bend along curve:** select the shape first, Ctrl + click a curve, then "Bend along curve". Settings: **Axis** (which of the shape's axes runs along the curve; recommended the longest), **Stretch to fit** the curve's length, **Offset** along the curve, and twist following the curve's frames (the same rotation-minimising frames as a sweep). The curve stays live.
- **Shrinkwrap:** select the shape first, Ctrl + click the target, then "Shrinkwrap onto". **Mode:** nearest surface point, or projected along the shape's normals or an axis; an **Offset**; **only the selected points** (a point set). It needs a fast nearest-point search (a tree of boxes).

Both read another object through the stack's sources and are deformers (only points move). Left to decide: where along the curve the shape starts and what "across" means, what happens past a curve's ends, how the frames are interpolated, the projection's directions, what the offset is measured along, the search structure (a crate or our own), the names, the controls, the cost estimates and saving.

## Options

Where the shape sits on the curve:
1. **Its start along the axis at the curve's start (plus Offset); across the curve, measured from the shape's pivot (chosen).** The shape's own place in the scene doesn't matter (moving it changes nothing), as scatter's copies ignore where their shape is (0146); its turn and size are kept. The pivot says where the curve runs through it: a standing cylinder (pivot on its axis) is threaded on the curve; a cube lying along X (pivot at its foot) rides on top. Moving the pivot ([Edit pivot](../manual/pivots.md)) is the control.
2. The shape's box middle across the curve: always threaded through, with no way to ride on top short of an extra setting.
3. As in Blender, the shape's place relative to the curve's object counts: the shape "flies off" when either is moved, the most common complaint about Blender's Curve modifier.

Past an open curve's ends:
1. **Straight on, along the way the curve leaves (chosen),** as in Blender: no tear, and a shape longer than its curve stays whole.
2. Clamped at the end: every point past it would collapse onto the end's cross-section.

Round a closed curve: **round again** (the distance wraps), so a ring bar stretched to fit closes on itself.

The search structure:
1. **Our own tree of boxes (`mesh::bvh`, chosen):** about 300 lines, median splits along the longest side, nearest-point and both-way line queries, deterministic, no new dependency. Collision hulls (milestone 13) and later tools can use it.
2. parry3d (Apache-2.0): a full collision library with its own math types (nalgebra), a large dependency tree to check and keep, for two queries.

## Decision

### Bend along curve (`mesh::curve::bend`, layer 0)

- **`BendOptions`** (`Copy`, `Default`, `clamped`): **Axis** (`BendAxis::Longest`, standard, or X, Y, Z: the longest side of the shape's box, X first on a tie), **Stretch to fit** (standard **off**), **Offset** (standard 0, ±1 km), **Twist** (standard 0°, ±36 000° as a sweep's; rounded to whole turns on a closed curve). Values that aren't numbers become standard.
- **`bend_along(mesh, curve, pivot, options)`**: the curve (already in the shape's space) is sampled as a sweep samples it (`path::samples`, at the finest detail, 64 pieces a quarter turn; straight stretches are one piece) and given the same rotation-minimising frames (`path::frames`, decision 0138), the twist spread evenly along the length on top. The chosen axis gets a rest frame built the same way (`first_up`: the shape's Z made square to the axis), so a straight curve along the axis moves the shape without bending it. A point's distance along the curve is `(its place along the axis − the shape's start) × stretch + Offset` (stretch = the curve's length over the shape's, when on and the shape has length); its two other coordinates, measured from the pivot in the rest frame, are laid out in the curve's frame there. Places and frames are interpolated linearly between samples (the up made square to the way again). Past an open curve's ends the shape carries on straight; round a closed curve the distance wraps. Through `deform`, so only points move (ids, UVs, seams, creases, materials stay) and many points are worked out on several threads.
- A curve with no length is refused ("the curve has no length…"); an empty shape comes back as it is. Sharp corners of the curve fold the shape on their inner side (no mitre, unlike a sweep): smooth curves bend best.

### Shrinkwrap (`mesh::shrinkwrap`, layer 0)

- **`ShrinkwrapOptions`**: **Mode** (`WrapMode::Nearest`, standard; `Normals`; `AxisX`, `AxisY`, `AxisZ`) and **Offset** (standard 0, ±1 km).
- **`shrinkwrap(mesh, target, only, options)`**: each point (or each in `only`) goes to the nearest place on the target (*Nearest*), or where the line through it along its normal (*Normals*: the sum of its polygons' Newell vectors, so bigger polygons count more) or along the shape's own axis meets the target, **either way**, the meeting nearer the point. A line meeting nothing, or a point with no normal (a loose point), leaves the point where it is. **Offset** then moves it along the way the target's triangle faces there (positive outside a target facing outward). Points sent anywhere that isn't a number stay.
- **The tree** (`mesh::bvh::TriangleTree`): the target's triangles (cut as drawing cuts polygons), triangles with no area or with a corner that isn't a number left out; leaves of up to four, split at the median of their centres along the longest side, built without recursion. `nearest(p)` looks at the nearer child first and skips boxes farther than the best so far; `along_line(origin, way)` the same with the smallest |t| a box can give. Ties go the same way every time. Ericson's closest point on a triangle moved here from Remesh, which now uses it from here.

### In the scene (`scene/src/wrap.rs`)

- **`ModifierKind::BendAlongCurve(CurveBendSettings { options, curve })`** and **`ModifierKind::Shrinkwrap(ShrinkwrapSettings { options, target, points })`**, family **Deform**, after Add lattice on the palette and in Mesh → Modifiers. Their standard settings name no object, which every shape refuses (`needs_sources`); the commands name theirs.
- **Sources:** `takes_source` is a curve object for Bend along curve, any object with a shape for Shrinkwrap. The curve is read as its **line** (`SourceData::Curve`, new: its points and handles, in its own space), not its solid; the target as its final shape (hidden or not). Both are placed into the shape's space by the source's matrix, and compared in the stack's cache, so reshaping, moving or (for the target) editing them works the step out again. A curve that is only a line has no surface: as a Shrinkwrap target it says "its target has no shape…".
- **The pivot:** `ModifierContext::pivot` (new) is the object's own pivot in its shape's space. Only kinds that read it (`ModifierKind::reads_pivot`, Bend along curve) get it, and it is part of the cached step, so moving the pivot works out that step again and nothing else.
- **Deleting** the curve or target removes the modifier in the same undo step; **copies** made with their curve or target follow the copy; **loops** are refused (`shape_uses`: a shape wrapped onto a shape made from it). Bake into shape keeps the result, and the curve and target stay.
- **The chosen points** are point ids of the shape as modelled (the Points mode's), listed once each; a modifier before Shrinkwrap that renumbers points (Remesh, Decimate) leaves none of them, which is said in the Attributes panel ("none of its chosen points are left…").
- **Cost:** a bend is a lattice's (20 points a polygon of a boolean's worth). A shrinkwrap is the target's polygons over 4 (building the tree, about 0.6 µs a triangle) plus one per point (2 to 3 µs each near the surface), through `work_with`, so a big target is wrapped onto on another thread.

### In the interface

- **Bend along curve** (`CommandId::BendAlongCurve`) and **Shrinkwrap onto** (`CommandId::Shrinkwrap`; the modifier is named **Shrinkwrap**), in `OFF_THE_RING` and `modifiers::COMMANDS`. `modifiers::pair_plan` takes the selection in order: exactly two, the shape first, then a curve (or a target); it says what's missing or wrong ("Ctrl + click a curve to bend along: the shape selected first is bent", "Select only two…", "The second selected isn't a curve…", a loop). The palette's buttons, the menu entries and command search use it. Icons drawn in code: a striped bar along a dashed quarter circle; a sheet hugging a hump with two arrows pressing it on.
- **The Attributes panel** (`attributes/wrap_rows.rs`): Bend along curve shows **Curve** (its name, **Select curve**), **Axis**, **Stretch to fit**, **Offset**, **Twist**; Shrinkwrap shows **Target** (**Select target**), **Mode**, **Offset**, and **Points**: "All points" or "*N* chosen", with **Use selected** (the points selected on this shape in the Points mode; greyed out elsewhere, saying how) and **All points**. One drag is one undo step.
- **No new keys or mouse actions.**

### Saving

Development file format **version 48**: version 47's model with `KindV43::BendAlongCurve { curve, axis, stretch, offset, twist }` (axis as a number, anything unknown the longest) and `KindV43::Shrinkwrap { target, mode, offset, points }` (mode as a number, anything unknown nearest; `points` `#[serde(default)]`). Settings are clamped on reading; the curve or target is checked as any source when modifiers are put back: missing, itself, the wrong kind (a bend along a shape that isn't a curve) or a loop is refused as damaged. (Since [0162](0162-kind-changes-and-wrong-sources.md), the wrong kind is taken off with a note instead, and baking a curve takes off the bends along it.) New sample `sample-v48.avtl`.

### Timings

Release build. Bending a sphere of 130 562 points along a three-turn spiral: 14 ms. Shrinkwrap of 32 514 points onto a sphere of 261 120 triangles: building the tree 150 to 170 ms; points 20 cm off the surface, nearest 99 ms, along normals 68 ms, along Z 44 ms; points 200 cm off, nearest 1.1 s (the search must look at many more triangles that far away).

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** Buttons on the modifiers palette, **Mesh → Modifiers → Bend along curve** and **Shrinkwrap onto** (under *Deform*) and command search; the shape clicked first, the curve or target Ctrl + clicked after, as for booleans and scatter. Exactly two selected.
- The names **Bend along curve** (command and modifier) and **Shrinkwrap onto** (command) / **Shrinkwrap** (modifier).
- **Axis: Longest side** as standard; **Stretch to fit off** (the shape keeps its length); **Offset 0**; **Twist 0°**.
- The shape's **start goes to the curve's start**, its place in the scene is ignored, and the curve runs through its **pivot**.
- Past an open curve's ends the shape **carries on straight**; round a closed curve it **goes round again**.
- Shrinkwrap's standard **Mode: Nearest surface point**, **Offset 0**; along normals or an axis it looks **both ways** and takes the nearer meeting; a point that meets nothing **stays**.
- **Offset** measured along the target's facing where the point lands.
- The chosen points are set with **Use selected** in the Attributes panel (Points mode), not taken from the selection when the command is run.
- Our own tree of boxes rather than parry3d (no new dependency).

## Consequences

- Projects with these modifiers need this version of the app or later.
- `SourceData` has a `Curve` variant; `ModifierContext` has the object's `pivot`; `ModifierKind::reads_pivot` keeps other steps cached when a pivot moves.
- `mesh::bvh` is there for the collision hulls and any later "where is the surface?" question (snapping onto surfaces, baking checks).
- Not yet: a curve deform that follows the curve's own tilt or radius per point (curves have neither yet), shrinkwrap's "keep above the surface" (Blender's *outside surface*), a falloff round the chosen points, and wrapping onto several targets.
