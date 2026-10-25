# 0157. Displace

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack and its recipe), [0144](0144-lattice-cage.md) (the deformer groundwork in `mesh::deform`), [0147](0147-curve-deform-and-shrinkwrap.md) (Shrinkwrap's chosen points, *Use selected*, and the shape's facing at a point), [0156](0156-game-bevel.md) (the latest modifier work), [0110](0110-controllable-automation.md) (the same model and settings always give the same result), [0028](0028-ask-about-controls.md) (ask about controls), [0035](0035-heavy-meshes.md) (heavy work off the interface thread), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0156](0156-game-bevel.md) (the development file format moves to version 58)

## Problem

Phase 10's nineteenth milestone is **Displace and noise**: a modifier that pushes a shape's points in and out by a noise, for rough rock, lumpy ground, dented metal and other damage. Settings: **Strength**, **Scale**, **Seed**, **Detail** (octaves) and the **noise kind** (smooth, ridged, cells), pushed along the normals or an axis; optionally limited to the selected points with a falloff. Deterministic, with a hand-written noise (no crate).

Left to decide: which noises and how they are made so they never change between computers or versions, what each kind's values mean for the push (both ways, or outward only), how the falloff is measured, the standard settings, where a new Displace goes in a list, how much work it is, the controls and saving.

## Options

The noise:
1. **Hand-written gradient noise and cell noise, hashed (chosen).** Perlin's improved gradient noise (quintic fade, the cube's 12 edge directions) and Worley's cell noise (one random point a cell, 3 × 3 × 3 cells searched), each corner's or cell's random numbers a hash of its whole-number place and the seed (`mesh::names::mix`, already the app's hash), never a permutation table or a generator's state. A value depends only on the place, the seed and the settings, and is the same on every computer (only adds, multiplies and `floor`, no library maths that might differ).
2. A noise crate (`noise`, `fastnoise-lite`): MIT, but their output could change with an update, and saved files would open with other bumps; the milestone asks for a hand-written one.
3. Simplex noise: a little faster in 3D, but trickier to write and check, for no visible gain at these sizes.

What each kind pushes:
1. **Smooth both ways (about −1 to 1), Ridged and Cells outward only (0 to 1) (chosen).** A smooth noise centred on the surface keeps the shape's size, as rock or ground should; crests and cells read as raised features, and a negative Strength turns them into grooves and dents. No *Midlevel* setting to explain.
2. Every kind 0 to 1 with Blender's *Midlevel* (0.5): one more setting, and a smooth noise at the standard would still grow the shape.

How the falloff round chosen points is measured:
1. **Along the surface (chosen):** the shortest way along the edges from the chosen points (Dijkstra, many starting points at once, stopping at the falloff). Exact, quick, and never leaks across a gap to a part of the shape that only lies near (the other side of a thin wall, the next finger).
2. Straight-line distance: leaks across gaps, and needs a search structure to stay quick with many chosen points.

## Decision

### The work (`mesh::displace`, layer 0)

- **`DisplaceOptions`** (`Copy`, `Default`, `clamped`): **Strength** (standard **5 cm**, ±100 m; negative pushes the other way), **Scale** (the bumps' size, standard **25 cm**, 1 mm to 1 km), **Seed** (standard **1**, 0 to 99 999, as Scatter's), **Detail** (layers, standard **3**, 1 to 8), **Noise** (`NoiseKind::Smooth`, standard; `Ridged`; `Cells`), **Direction** (`DisplaceDirection::Normals`, standard; `AxisX`, `AxisY`, `AxisZ`) and **Falloff** (standard **20 cm**, 0 to 1 km; only used with chosen points). Lengths that aren't numbers become standard.
- **The noise** (`displace/noise.rs`): `gradient` (Perlin, 0 at every whole-number corner, clamped to −1…1), `cells` (F2 − F1 over 0.6, eased `t(2 − t)`: domed cells, sharp grooves where two meet), and `layered`: **Detail** layers, each half the size and half the height of the one before, shifted so their grids never line up, each with a seed of its own, summed and divided by the heights' sum so the range stays. *Ridged* is `(1 − |gradient|)²` a layer. The noise is read at `place / Scale` in the shape's own space.
- **`displace(mesh, only, options)`**: each point moves by `way × value × Strength × share`, where *way* is the shape's facing at the point (`shrinkwrap::point_normals`: its polygons' Newell vectors summed, bigger polygons counting more) or the axis, and *share* is 1, or with chosen points their falloff share (`displace/falloff.rs`: 1 on them, `smoothstep(1 − d / Falloff)` within the falloff along the edges, 0 beyond; with Falloff 0 only the chosen points move). A point with no facing (a loose point) stays when pushed along normals.
- **Reusing `mesh::deform`:** the deformer groundwork's point loop is now `deform::with_points_moved(mesh, moved(id, place))`, which `deform` itself calls: it keeps polygons, ids, UVs, seams, creases, hard edges and materials, leaves a point sent anywhere that isn't a number where it was, and shares many points between threads. Displace needs the point's id (its facing, whether it was chosen), which `Deformer::moved(place)` doesn't give, so it calls `move_points` directly. Bend, twist and taper (milestone 20) keep using `deform`.

### In the scene (`scene/src/displace.rs`)

- **`ModifierKind::Displace(DisplaceSettings { options, points })`**, family **Deform**, after Shrinkwrap on the palette and in Mesh → Modifiers. `points` are point ids of the shape as modelled (the Points mode's), listed once each in order, empty for every point, as Shrinkwrap's are. No sources; not by purpose.
- A chosen point that isn't in the modifier's input (deleted, or renamed by a Remesh or Decimate before it) is left out; with none left, the step passes its input on and says why ("none of its chosen points are left…").
- **A new Displace goes before Weighted normals** (`goes_before`), which shades best last (decision 0148); anywhere else at the end. Subdivide, if wanted for fine detail, goes before it by hand (the manual says so).
- **Work** (`mesh::displace::work`): points × layers (×1 smooth, ×2 ridged, ×3 cells) plus 4 with chosen points, over 40, in polygons of a boolean's worth; from about 40 000 points at the standard settings the viewport works it out on another thread.

### In the interface

- **Displace** (`CommandId::Displace`) on the modifiers palette after Shrinkwrap onto, in **Mesh → Modifiers** under *Deform*, and in command search; its icon (drawn in code) a block whose top is pushed into bumps of two sizes, an arrow up over a bump and one down over a dip.
- **The Attributes panel** (`attributes/displace_rows.rs`): **Noise**, **Strength**, **Scale**, **Detail**, **Seed**, **Direction**, **Points** ("All points" or "*N* chosen", with **Use selected** and **All points**: Shrinkwrap's row, now shared, `wrap_rows::points_row`) and **Falloff** (greyed out with no chosen points). One drag is one undo step.
- **No new keys or mouse actions.**

### Saving

Development file format **version 58**: version 57's model with `KindV43::Displace(v58::DisplaceV58 { strength, scale, seed, detail, noise, direction, falloff, points })` (noise and direction as numbers, anything unknown the standard; `points` bounded as Shrinkwrap's). Settings are clamped on reading; points listed twice are listed once. New sample `sample-v58.avtl` (a sphere with a standard Displace, a cube with every setting changed and three chosen points, hidden in the viewport), whose shapes the tests check come out exactly the same, so the noise can't change by accident.

### Timings

Release build, one thread, a sphere of 130 562 points: smooth noise 13 ms at detail 1, 34 ms at 3, 169 ms at 8; ridged 34, 71 and 209 ms; cells 31, 91 and 330 ms; smooth at detail 3 with every seventh point chosen (falloff along the surface) 78 ms. Copying the shape alone is 3 ms. Over 100 000 points the work is shared between threads.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** The palette button (after Shrinkwrap onto), **Mesh → Modifiers → Displace** (under *Deform*) and command search; the chosen points with **Use selected** in the Points mode, as Shrinkwrap's.
- The standard settings: **Strength 5 cm**, **Scale 25 cm**, **Detail 3**, **Seed 1**, **Noise Smooth**, **Direction along the normals**, **Falloff 20 cm**.
- **Smooth pushes both ways** (the shape keeps its size); **Ridged and Cells push outward** only, inward with a negative Strength. No Midlevel setting.
- The **falloff is measured along the surface**, smoothly (smoothstep), so it never jumps across a gap.
- The noise is read where each point is **after the modifiers before it** (so moving the shape's points by modelling changes its bumps; moving the object doesn't).
- A new Displace goes **before Weighted normals**, otherwise at the end; Subdivide is put before it by hand.

## Consequences

- Projects with a Displace modifier need this version of the app or later.
- The noise's maths is now part of the file format in effect: changing it would change saved shapes. `v58_tests` and `mesh::displace` tests pin it.
- `deform::with_points_moved` is the shared point loop for every deformer, for those that need a point's id.
- Not yet: a texture image as the noise (Blender's image displace), a point's push painted by hand (weights), noise moved or turned by an object, and smoothing the shading of a displaced low-poly shape automatically (smoothed by angle, sharp bumps may shade as creases: the manual says to raise the angle or put a Subdivide first).
