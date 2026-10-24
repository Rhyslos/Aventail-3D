# 0158. Bend, twist and taper

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack and its recipe), [0144](0144-lattice-cage.md) (the deformer groundwork in `mesh::deform`, and its notes for this milestone), [0153](0153-array.md) (one modifier with a choice of layouts), [0151](0151-collision-hulls.md) (outlines drawn over the view), [0157](0157-displace.md) (the latest modifier work, `deform::with_points_moved`), [0068](0068-pivot-editing-and-scene-units.md) (pivots), [0028](0028-ask-about-controls.md) (ask about controls), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0157](0157-displace.md) (the development file format moves to version 59)

## Problem

Phase 10's twentieth milestone is **Bend, twist and taper**: three deformer modifiers on the lattice's groundwork. **Bend:** an angle and an axis. **Twist:** an angle and an axis. **Taper:** an amount, an axis and a curve. All three: **Limits** (from–to along the axis) and a **Centre**. And a helper drawn in the viewport (the axis and the range) while the object is selected.

Horns, tusks, bent pipes, leaning trunks, twisted columns, rope, drill bits, blades, spikes and table legs are all simple shapes changed along their length; modelling them bent, or bending them point by point, gives uneven curves and can't be undone later.

Left to decide: one modifier or three, what Limits are measured in, where the centre is and which point stays still, what each kind does exactly (the bend's arc and its way round the axis, the taper's curve), what lies past the limits, the standard settings, the helper, the controls and saving.

## Options

One modifier or three:
1. **One modifier, *Bend, twist or taper*, with a Kind choice (chosen),** as Array's Layout (decision 0153). The three share every setting but one or two (axis, limits, centre), so switching kind keeps them; one palette button, one icon, one file kind, one set of rows. The list names each by its kind ("Bend", "Twist", "Taper"), and the undo steps too ("Add Bend", "Change Taper"), so a stack reads as clearly as three kinds would. Command search finds it by any of the three words, which are in its name.
2. Three modifiers: three palette buttons for one idea in a column that is already long, three file kinds and three sets of rows that differ by one field, and no way to turn a bend into a twist without losing its axis and limits.

What the Limits are measured in:
1. **Shares of the shape's length along the axis, 0 % to 100 % (chosen),** as Blender's *Simple deform* does. The standard 0 % to 100 % covers the whole shape whatever its size, so no switch is needed to say "the whole shape", and the limits keep their meaning when the shape is scaled or edited. The length is measured on the modifier's input (the shape as the modifiers before it leave it), along the axis in the deformer's space.
2. Distances from the centre, as 3ds Max's *Upper* and *Lower limit*: needs a *Limit effect* switch as well (without limits, the bend still has to spread over the shape's length), and a standard that suits one size of shape only.

The centre and what stays still:
1. **The object's pivot, moved by Centre X, Y, Z (chosen):** the place along the axis that stays still is the centre brought within the limits. A new primitive stands on its pivot, so it bends over from its foot; a pivot in the middle bends both halves away (a C), as 3ds Max's centre does; a centre below the limits starts the bend at the lower limit, so "bend only the top half" is just *Limit from 50 %*. Moving the pivot (Edit pivot) moves the bend with it, which is what a pivot is for.
2. A centre object (an *origin* empty, as Blender's): a second object to make and keep track of, for something the pivot and a Centre offset already do.

Past the limits:
1. **Rigid, carrying on from the limit (chosen),** as the lattice's ranges clamp (decision 0144): above a bend's top limit the shape goes on straight in the direction the bend leaves it, turned and moved but not bent; a twist's top is turned by the whole angle; a taper's is scaled as its limit. No tear, and what is past the limit keeps its shape.
2. Left alone: a tear where the shape crosses the limit.

## Decision

### The work (`mesh::axis_deform`, layer 0)

- **`AxisDeformOptions`** (`Copy`, `Default`, `clamped`): **Kind** (`AxisDeformKind::Bend`, standard; `Twist`; `Taper`), **Axis** (`DeformAxis::X`, `Y`, `Z`, standard Z), **Angle** (bend and twist, standard **45°**, within ±3 600°, ten turns), **Direction** (bend, degrees round the axis, standard 0°, wrapped into 0° to 360°), **Amount** (taper, a share, standard **−0.5**, within ±10), **Curve** (taper, a share, standard 0, within ±10), **Limits** (two shares 0 to 1, either order, standard 0 and 1) and **Centre** (from the pivot, in centimetres, each within ±1 km). Numbers that aren't numbers become standard.
- **`frame(options, pivot)`**: the deformer's space in the shape's: its origin at the pivot plus Centre, its Z along the axis, its X the next axis round (X → Y → Z → X: Z bends towards X, X towards Y, Y towards Z) turned by Direction round the axis. Only turns and moves, so it is never flattened.
- **`limits(mesh, options, pivot)`**: the shape's stretch along the deformer's Z, and the part within the limits (an `AxisRange`).
- **`AxisDeformer`** (a `deform::Deformer`): with the range *R*, the still place *c* (0, the centre, clamped into *R*) and a point's place along Z clamped into *R*, *z′*:
  - **Bend:** the stretch within *R* follows an arc of total angle θ (curvature θ ÷ |*R*|) starting at *c* heading along Z and turning towards X; a point keeps its X as its distance across the arc (inside the bend nearer its centre) and its Y; past *R* it carries on along the arc's end direction. The arc's offsets are written as `d · sin(a)/a` and `d · (1 − cos a)/a`, with a series for tiny angles, so a huge radius loses no precision.
  - **Twist:** X and Y turned round Z by θ · (*z′* − *c*) ÷ |*R*|.
  - **Taper:** X and Y scaled by `1 + Amount · (z′ − c)/|R| + Curve · 4t(1 − t)` (*t* the fraction across *R*), never below 0 (a point, never inside out).
  - Limits with no length, no angle, or no amount and curve: `None`, nothing moves.
- **`axis_deform(mesh, options, pivot)`** = `deform(mesh, frame⁻¹, deformer)`: only points move; polygons, ids, UVs, seams, creases, hard edges and materials stay. A shape with no points comes back as it was.
- **`helper(mesh, options, pivot)`** (`DeformHelper`): the axis from end to end of the shape in 32 lines, each point moved by the deformer (so the line bends and stays straight for a taper or twist); a cross at each limit, a fifth of the shape's length across (at least 1 cm), moved too (so it turns with a twist and shrinks with a taper); and the centre.

### In the scene (`scene/src/axis_deform.rs`)

- **`ModifierKind::BendTwistTaper(AxisDeformOptions)`**, family **Deform**, after Displace on the palette and in Mesh → Modifiers. `name()` is its kind's ("Bend", "Twist", "Taper"). It **reads the pivot** (`reads_pivot`, as Bend along curve), so moving the pivot works it out again, and nothing else does. No sources; not by purpose.
- **A new one goes before Weighted normals** (`goes_before`), which shades best last (decision 0148); anywhere else at the end, so after a Subdivide already there (which gives it points to bend smoothly).
- **Work:** points ÷ 20 (decision 0144's estimate for deformers); from 100 000 points the viewport works it out on another thread.
- **`Scene::deform_helpers(id)`**: the helper of each Bend, twist or taper that applies in the viewport, measured on the shape that modifier last started from: **`Scene::modifier_input(id, index)`**, new, reads it from the stack's cache (the step before's result), or the shape before the modifiers when nothing is kept; it never works anything out, so a heavy stack still draws at once.

### In the interface

- **Bend, twist or taper** (`CommandId::BendTwistTaper`) on the modifiers palette after Displace, in **Mesh → Modifiers** under *Deform*, and in command search, named for all three kinds (it adds a Bend; its Kind is changed in the Attributes panel). Its icon (drawn in code): an upright bar curling over to the right, the straight bar faintly behind it, slices across the bend.
- **The Attributes panel** (`attributes/axis_deform_rows.rs`): **Kind**, **Axis**, then by kind **Angle** and **Direction** (bend), **Angle** (twist), **Amount** and **Curve** (taper, as percentages), then **Limit from**, **Limit to** (percentages) and **Centre X**, **Y**, **Z** (lengths in the project's units). One drag is one undo step.
- **The helper** (`components/deform_helper.rs`): for each selected shape (not a cutter), its helpers' lines, in the world, added to the overlay's guide lines (rebuilt when the drawing changes, as collision outlines are, decision 0151), in every mode, as a new **`OverlayStyle::Helper`**: light cyan, 1.5 points wide, drawn **over everything**, as the axis runs through the middle of the shape where the depth test would hide it: the renderer's overlay gets lines on top (`Overlay::line_on_top`, a `SpritesOnTop` pipeline with no depth test, drawn last). The centre is a small cross along the shape's axes, a quarter of a limit cross. Hidden when the modifier is off or not shown in the viewport.
- **No new keys or mouse actions.**

### Saving

Development file format **version 59**: version 58's model with `KindV43::BendTwistTaper(v59::AxisDeformV59 { kind, axis, angle, direction, amount, curve, limits, centre })` (kind and axis as numbers, anything unknown the standard). Settings are clamped on reading. New sample `sample-v59.avtl` (a pole with a standard Bend and a Twist after it, and a pole with a Taper with every setting changed, hidden in the viewport), whose shapes the tests check come out exactly the same.

### Timings

Release build, a sphere of 130 562 points (shared between threads): bend 6 ms, twist 5 ms, taper 4 ms.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **One modifier, *Bend, twist or taper*, with a Kind choice**, named by its kind in the list, rather than three modifiers.
- **No new keys or mouse actions.** The palette button (after Displace), **Mesh → Modifiers → Bend, twist or taper** (under *Deform*) and command search.
- The standard settings: **Bend**, **Axis Z**, **Angle 45°**, **Direction 0°** (towards X), **Taper Amount −50 %**, **Curve 0 %**, **Limits 0 % to 100 %**, **Centre** at the pivot.
- **Limits as shares of the shape's length along the axis** (as Blender's), not distances.
- It works **from the pivot**: the centre, brought within the limits, stays still and the shape bends away from it (a primitive bends from its foot; a pivot in the middle bends both halves).
- **Past the limits the shape carries on rigidly**, as the limit leaves it.
- A bend's **Direction** is measured from the next axis round (Z bends towards X, X towards Y, Y towards Z).
- A taper never goes **thinner than a point** (no turning inside out).
- The **helper** is light cyan lines (the axis as it bends, a cross at each limit, a small cross at the centre), drawn over the selected shape in every mode, **showing through the shape** (as 3ds Max's gizmos do).
- A new one goes **before Weighted normals**, otherwise at the end.

## Consequences

- Projects with a Bend, twist or taper modifier need this version of the app or later.
- `ModifierKind::name()` can now depend on a kind's settings (the Kind); undo labels follow it.
- The renderer's overlay can draw lines over everything (`Overlay::line_on_top`); only helpers use it, so the selection and the edited elements still hide behind surfaces as before.
- `Scene::modifier_input` gives any later modifier that draws a helper (a wireframe's preview, a mirror plane) the shape it starts from without working anything out.
- The deformer groundwork (`mesh::deform`) is now used by the Lattice and Bend, twist or taper through `deform`, and by Displace through `with_points_moved`.
- Not yet: a *Stretch* kind (Blender's), a taper on one side axis only, a centre following another object, and dragging the helper's limits in the viewport (they are set in the Attributes panel).
