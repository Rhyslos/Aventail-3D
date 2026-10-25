# 0144. Lattice cages, and the deformer groundwork

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack, its recipe and its sources), [0143](0143-subdivide-and-creases.md) (the latest modifier), [0138](0138-curves-sweep-lathe.md) and [0139](0139-spline-editing.md) (a new kind of object whose points are edited in the Points mode), [0135](0135-live-booleans.md) (a live feature reading other objects: cutters), [0006](0006-base-plus-operation-stack.md) (tools make commands), [0028](0028-ask-about-controls.md) (ask about controls), [0035](0035-heavy-meshes.md) (heavy work off the interface thread), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0143](0143-subdivide-and-creases.md) (the development file format moves to version 45)

## Problem

Phase 10's sixth milestone is a lattice cage: with shapes selected, **Add lattice** adds a cage object fitted round them, with **Divisions X/Y/Z** (recommended 2 × 2 × 2, at most 16), and each shape gets a Lattice modifier following it. In the Points mode the cage's points move as a mesh's do (selecting, the gizmo, Coordinates), as curves' do. The deformation is smooth (trilinear for 2 × 2 × 2, a cubic B-spline beyond, with an *Interpolation* option), the cage is drawn as lines, and deleting it removes the modifiers in the same undo step, as deleting a cutter ends its booleans.

It is the first modifier that reads another object, so decision 0142's notes on sources become real (and its tests: deleting the source, copies, loops). And it is the first **deformer**: bend, twist and taper (milestone 20) will move points through a space with a range the same way, so the groundwork is shaped for them.

Left to decide: what the cage is in the scene, how the modifier reads it, the maths (which points, what weights, what happens outside the box), how the cage is fitted, how its points are edited and picked, the controls, what the Attributes panel shows, saving, and the groundwork's shape.

## Options

What the cage is:
1. **A new kind of object with no shape of its own (chosen):** `ObjectKind::Lattice`, like curves a kind holding its own data (decision 0138), but like a group it has no shape: `has_shape()` is false, so it is never exported, cut, mirrored, edited by recorded edits or given modifiers, booleans or edit layers, with no special cases needed in those places.
2. A mesh object of loose points: its points could be edited by recorded edits, but the modifier would have to guess the grid from ids, every mesh tool would need to refuse it, and exports would have to skip it.
3. Data inside the modifier, with no object: no gizmo, no selecting, nothing to move as a whole, and two shapes couldn't share one cage.

How the shape follows the cage:
1. **By the points' offsets from rest (chosen):** a shape's point moves by the weighted sum of how far the cage's points near it have moved from their rest places. A cage at rest changes nothing, exactly, whatever its interpolation; moving or scaling the cage object without touching its points changes nothing either (as in Blender).
2. By the points' positions directly: a B-spline doesn't pass through its points, so even a cage at rest would squash the shape slightly.

What lies outside the box:
1. **Clamped to the box (chosen):** a point outside moves as the nearest place on the box's sides does, so the shape carries on with no tear (raise the top of the cage, and what is above it rises with it, unbent). The range's fraction is clamped, which is the same rule bend, twist and taper need past their limits.
2. Left alone: a tear wherever the shape crosses a side whose points moved.
3. Fading out over a cell outside (as Blender's B-spline does): a soft edge, but the tear's place becomes a stretch that is hard to predict.

The smooth interpolation:
1. **A clamped uniform B-spline of degree `min(3, n − 1)` along each side (chosen):** the end points pull the box's sides fully (so a 2 × 2 × 2 cage is exactly trilinear, and its corners carry the shape's corners), smooth inside. With 3 points a side it is quadratic, 4 a cubic Bézier, 5 and more a cubic B-spline.
2. Blender's uniform B-spline with the end points repeated: the box's corners never fully follow their points.

## Decision

### The deformer groundwork (for bend, twist and taper)

`mesh::deform` (layer 0) holds what every deformer shares:

- **`Deformer`**, a trait with one method, `moved(p) -> Vec3`: where a point in the deformer's own space goes, in the same space. It is `Sync`, so it can be shared between threads.
- **`deform(mesh, into, deformer)`**: carries each of the mesh's points into the deformer's space by the matrix `into`, moves it, carries it back, and returns the mesh with only its points moved: polygons, ids, UVs, seams, creases, hard edges and materials stay, so a deformed shape textures and selects exactly as before. Points sent to a place that isn't a number stay where they were. A flattened space (a determinant near 0) or one that isn't made of numbers is refused (`DeformProblem::FlatSpace`, a plain-words reason). Over 100 000 points the work is shared between threads (`parallel::in_parallel`).
- **`AxisRange`** (from–to along an axis, in either order, `fraction(x)` from 0 at its start to 1 at its end, clamped beyond, 0 for a range with no length) and **`BoxRange`** (three of them; a lattice's box). A range is where a deformer works; past it, the deformer carries on as at its end.

**For milestone 20 (bend, twist, taper):** each is a `ModifierKind` variant whose settings (axis, angle or amount, **Limits** as an `AxisRange` from–to, and **Centre**) live in a `Copy` struct in `mesh` with `Default` and `clamped()`, and a `Deformer` working in a space centred on **Centre** with the chosen axis as its Z (say). Its `moved(p)` takes `t = limits.fraction(p.z)` for the amount (so the shape past the limits carries on rigidly, as Blender's *Simple Deform* does) and bends, twists or tapers. `ModifierKind::apply` builds the matrix `into` (the object's space into the deformer's, from Centre and the axis) and calls `deform`; `work` is the input's point count over about 20 (see Timings). They read no other object, so `sources()` stays empty. The viewport helper (axis and range) is drawn by the interface's drawing code as overlay lines, as the cage's are (`ui/src/components.rs`, `curve_marks`). Their tests follow `mesh/src/lattice/tests.rs`: at rest nothing moves, the range clamps, degenerate shapes (n-gons, zero-length edges, open borders, non-manifold, empty) keep their polygons, NaN and huge settings are brought within limits, and a fuzz of settings never gives a point that isn't a number.

### The cage

`mesh::lattice::Lattice`: **divisions** (points along each side, 2 to 16), a **size** (its box, centred on its middle, each side 0.01 cm to 10 km), its **points** (X fastest, then Y, then Z; at most 16³ = 4 096) and its **interpolation** (`Smooth`, standard, or `Linear`). Each point has a **rest** place, spread evenly over the box. `Lattice::fitted(size, divisions)` fits a box of a shape's size; a flat side (a plane) gets a twentieth of the largest side, at least 0.1 cm, so there is room inside. `Lattice::deformer()` works out each point's offset from rest once and gives the `Deformer`: for a place, the fraction across the box along each axis (clamped), the weights of the points along each side (`lattice/weights.rs`: two for linear, up to four for the B-spline, by the Cox–de Boor recurrence), and the offset is the sum over the up to 64 points round it.

Changes go through **`LatticeEdit`**: `Transform { points, matrix }` (a point sent to a place that isn't a number, or further than 10 km, is refused; places not in the cage are left alone), `Divisions` (each new point placed where the old cage moves its rest place, so the cage keeps its shape: exact for linear interpolation), `Interpolation` and `Reset`.

### In the scene

- **`ObjectKind::Lattice(Lattice)`**, named "Lattice". `has_shape()` is false (no shape to export, cut, mirror, edit or modify; `evaluated` is `None`), its `local_bounds` is its points' box. **Apply transforms** leaves it alone (its box is its own space; moving the points instead would bend the shapes). Copies, groups, hiding, parenting and deleting work as for any object.
- **`ModifierKind::Lattice(LatticeOptions { cage })`**, family **Deform**, second on the palette and in Mesh → Modifiers (after Subdivide, before Remesh). `sources()` is the cage; `takes_source(kind)` (new on `ModifierKind`) says which kinds of object can be a source: a Lattice modifier takes only a cage, and the kinds before it any shape (`Scene::check_modifiers` uses it). Its standard settings name no cage (`LatticeOptions::NO_CAGE`), which every object refuses: only `Command::AddLattice` makes a working one.
- **Sources carry their kind:** `SourceShape { id, data, matrix }` where `data` is a **`SourceData`**: `Shape(mesh)` (the source's final shape, for scatter and shrinkwrap later) or `Cage(lattice)`. A later kind that reads a curve (curve deform) adds a `Curve` variant. The stack's cache compares them, so moving a cage point, the cage, or the shape works the modifier out again; anything else is kept.
- **`apply`**: the cage's points at rest give the input unchanged; a flattened cage is a problem shown in the Attributes panel ("the cage is flattened…") with the shape unbent; otherwise `deform(input, cage_space ← shape_space, cage.deformer())`.
- **`Command::AddLattice { ids, divisions }`**: refused as a whole for a group, a cage or an instance; otherwise one new cage at the top of the Objects panel, fitted round the shapes as the viewport shows them (round **one** shape: in its own space, turned and sized as it is; round **several**: their world box, lined up with the world), and a Lattice modifier at the end of each shape's list, all one undo step ("Add lattice", "Add lattice round 3 objects"). The cage becomes the selection, so pressing 1 edits its points at once.
- **`Command::EditLattice { id, edit }`**: one `SetKind` edit ("Move 4 cage points", "Change lattice divisions", "Change lattice interpolation", "Reset cage points"); no step when nothing changes.
- **Deleting the cage** removes every modifier that follows it in the same undo step (0142's `forget_modifier_sources`); **copying** shapes with their cage points the copies at the copy (`follow_copied_sources`), and a shape copied alone follows the same cage; an object put in from elsewhere loses modifiers naming no cage (`keep_usable_modifiers`). **Loops** can't arise (a cage reads nothing and takes no modifiers), and odd sources are refused: a shape, the object itself, no cage. Tests: `scene/src/lattice_tests.rs`.

### In the interface

- **Add lattice** is `CommandId::Lattice` ("Add lattice", as it makes an object too, not only a modifier), in `OFF_THE_RING` and `modifiers::COMMANDS`; `modifiers::command` turns it into `Command::AddLattice` with 2 × 2 × 2. Its targets are the selected shapes, groups and cages left out. Palette icon: a cage of three by three dots pulled out on one side, the shape inside swelling (drawn in code).
- **Drawing:** a cage is a `DrawItem` with `cage: Some(CageDrawing)` (its lines and points) and empty meshes, except in the Points mode when selected, where its `mesh` is its points as loose points named by their place (`Lattice::points_mesh`), so picking, box and brush selection, Ctrl and Shift, the selection's undo, the gizmo and Coordinates all work unchanged. Its lines are overlay lines in the curves' colours (accent when selected), its points dots in the Points mode.
- **Gizmo and Coordinates:** `PointsStart::cage` makes the change a `LatticeEdit::Transform` of those places, in the cage's own space.
- **Picking the cage:** a click within 6 screen points of one of its lines picks it, **in front of the surfaces** (a cage hugs its shape, so its lines lie along the shape's edges and would otherwise never be reached), but not through them: a line hidden behind a surface isn't picked (`PickScene::surface_distance`, new, gives the depth to compare). A box touching its lines takes it, as curves'.
- **Delete** on cage points says they can't be deleted (change the Divisions or Reset points); the Edges and Polygons modes say a cage has neither.
- **The Attributes panel:** a selected cage shows a **Lattice** section: **Divisions X**, **Y**, **Z** (2 to 16; one drag one undo step), **Interpolation** (Smooth, Linear), **Reset points** (greyed out at rest) and the shapes it bends. A bent shape's **Lattice** modifier names its **Cage** with a **Select cage** button. The heading calls a cage "(lattice cage)"; the Objects panel shows the lattice icon; Edit layers says a cage has none.

### Saving

Development file format **version 45**: version 44's model (version 43's) with each cage's divisions, size, points and interpolation (`v45::LatticeV45`, in `ProjectV43::lattices`; the object itself is an empty group in the older model, as curves are) and `KindV43::Lattice { cage }`. A version of its own because an older app would open the cages as empty groups and the shapes unbent, and lose both on saving. Cages are read before modifiers (which name them). Divisions outside 2 to 16, sides outside their limits, a point count that doesn't match, points that aren't numbers or are further than 10 km, a cage listed twice or naming an object that isn't an empty group are refused as damaged; a Lattice modifier naming anything but a cage is refused. (Since [0162](0162-kind-changes-and-wrong-sources.md), one naming an object that is there but isn't a cage is taken off with a note instead.) New sample `sample-v45.avtl`.

### Timings

Release build, a sphere of 130 562 points: a 2 × 2 × 2 linear cage 7 ms, 4 × 4 × 4 smooth 18 ms, 16 × 16 × 16 smooth 26 ms (one thread; over 100 000 points a bigger shape is shared between threads). `ModifierKind::work` counts 20 points as one polygon of a boolean's worth, so from 100 000 points the viewport works the bend out on another thread, showing the last result meanwhile (decision 0142).

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** Add lattice on the modifiers palette, **Mesh → Modifiers → Add lattice** (under *Deform*) and command search; the cage's points with the usual Points-mode selecting, gizmo and Coordinates; clicking a cage's line selects it, as a curve's.
- The name **Add lattice** for the command (it makes an object as well as a modifier) and **Lattice** for the modifier and the cage; the setting **Divisions X / Y / Z** counts **points** along each side (2 is a point at each end), as 3ds Max's FFD 2×2×2 and Blender's lattice resolution do.
- **Divisions 2 × 2 × 2** to start with, at most 16 a side.
- **Interpolation on the cage**, not on each modifier (as in Blender): **Smooth** (a B-spline that reaches the box's sides) as standard, **Linear** the other choice; a side of 2 points is linear either way.
- The cage is **fitted snugly** round the shapes as the viewport shows them, lined up with one shape (turned as it is) or with the world for several; flat shapes get a little thickness.
- **Add lattice selects the new cage**, so pressing 1 edits its points straight away.
- Parts of a shape **outside the cage** move as its nearest side does (no tear); moving the cage object without touching its points changes nothing.
- A cage's **lines are picked in front of surfaces** (so it can be clicked where it hugs its shape), never through them.
- **Deleting a shape leaves its cage**; deleting the cage removes its Lattice modifiers (undo brings both back). **Apply transforms** leaves a cage as it is.
- Exports leave cages out.

## Consequences

- Projects with lattice cages need this version of the app or later.
- `SourceShape` carries a `SourceData` now (a shape or a cage); a later modifier reading a curve adds a variant.
- The Points-mode machinery now edits three kinds of points: a mesh's (recorded edits), a curve's (curve edits) and a cage's (lattice edits). A fourth would be worth a small trait instead of flags on `DrawItem` and `PointsStart`.
- Bend, twist and taper (milestone 20) reuse `mesh::deform`: a `Deformer`, an `AxisRange` for their limits, and `deform` for the work.
- Not yet: Lattice modifier settings of its own (a strength, or a point group limiting it), cages in the UV Editor, and a cage following a shape's later edits by itself (move or refit it by hand).
