# 0153. Array, and the mirror generator

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack, its recipe and its sources), [0152](0152-weld-by-distance.md) (Weld by distance, which joins the copies), [0146](0146-scatter.md) (Scatter: copies appended with `parts::append`), [0144](0144-lattice-cage.md) and [0147](0147-curve-deform-and-shrinkwrap.md) (modifiers reading another object), [0054](0054-symmetry.md), [0059](0059-symmetry-for-every-edit.md), [0088](0088-live-mirror.md) and [0126](0126-rework-and-first-use.md) (symmetry: the live mirror, per object), [0068](0068-pivot-editing-and-scene-units.md) (pivots), [0028](0028-ask-about-controls.md) (ask about controls), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0152](0152-weld-by-distance.md) (the development file format moves to version 54)

## Problem

Phase 10's fifteenth milestone is **Array and mirror generators, radial included**: an Array modifier laying copies of the shape in a line, a grid or a ring, with a **Count** per axis and an **Offset** (relative to the size, or a distance); for a ring, a **Count**, an **Angle** (360° standard), an **Axis** and a **Centre** (the object's pivot or a picked object); **Merge touching copies** through Weld by distance with a **Merge distance**; caps (optional start and end objects) if they aren't heavy; and the copies' UVs kept, with an option to offset them so the copies don't overlap. For the mirror generator: the per-object symmetry of Phase 9 already exists, so the question is whether anything is missing.

Fence posts, railings, stair steps, a row of bolts, tiles, the spokes of a wheel, the teeth of a gear, rivets round a hatch: a game prop is full of repeats. Modelling one and repeating it live means editing one changes all of them, and the repeat count can change at the end.

Left to decide: the settings and their standard values, how a ring's copies are spread, where a ring turns round, how the centre object is picked, what merging welds, what "UVs that don't overlap" means, caps, saving, and the mirror.

## Options

The layouts:
1. **One modifier with a Layout choice: Line, Grid or Radial (chosen).** A line's step is a vector, so an up-and-along step makes stairs and a diagonal makes a slope; a grid has a count and a spacing per axis. One button on the palette, and switching layout keeps the other settings.
2. Blender's way: a line only, a grid made by stacking two arrays, a ring made with an empty object's turn. Powerful but indirect, and the ring needs a second object.
3. Three modifiers. Three palette buttons for one idea.

Spreading a ring's copies:
1. **A whole turn (360°) shared out evenly with no copy on top of the first; less than a whole turn as a fan with copies at both ends (chosen),** as in most programs: 8 copies over 360° are 45° apart, 3 copies over 90° are at 0°, 45° and 90°.
2. Always `angle / count`: a fan of 90° would stop short of its end.

Where a ring turns round, and which way copies move out:
1. **The object's own pivot, or another object's pivot (the Centre), round the object's own X, Y or Z axis (chosen; Z, up, standard).** A **Radius** moves each copy out from the centre before it is turned (along X for the Y and Z axes, along Y for the X axis), standard **0**: a part modelled off its pivot (a spoke, a tooth) turns as it stands, as in Blender. The Attributes help says to move the shape off its pivot or raise Radius when the copies sit on top of each other.
2. A standard radius (200 cm, as Cinema 4D's cloner): every ring visible at once on a fresh cube, but a part already modelled off its pivot would be moved out twice.

Picking the centre object:
1. **A Centre list in the Attributes panel: "Its pivot" and every other object by name (chosen),** leaving out the shape itself and objects made from it (a loop). No selection order to remember, and any object can be the centre, a group included (an "empty" to turn round).
2. Select the shape, then Ctrl + click the centre (as Bend along curve does). That order is for kinds that can't work without the other object; a ring works without one, and the array is added to every selected shape.

The centre object is read through the stack's sources (0142) for **where it stands only** (`SourceData::Place`): its shape is never worked out for this, so any object can be one, and moving it re-works the ring. Deleting it puts the ring back on the pivot (`without_sources`) in the same undo step, rather than removing the array; copies made with it turn round the copy.

Merging:
1. **Weld by distance with Only at open borders and Keep UV seams on (chosen),** as decision 0152 planned: only the points where copies touch melt; closed copies stay closed solids side by side (welding faces of two closed boxes together would make edges of four polygons); joins where the UVs split are marked as seams on shapes that have seam marks. The first copy's points stay where they are. Standard **off**, as in Blender: copies are often meant to stay apart. Merge distance standard **0.1 mm** (Weld by distance's), shown in millimetres.
2. Welding only the touching ends of neighbouring copies (`Some(points)`): more exact for a long line, but a full ring's last copy touches its first, and a grid's copies touch on four sides. The open-border rule already keeps welding to where copies meet.

UVs that don't overlap:
1. **"UVs side by side": each copy's UVs shrunk into a square of their own, in a grid of squares in the 0–1 texture square (chosen, off as standard),** keeping their shape, with a small margin. Everything stays in the texture square, where Substance Painter and lightmap bakers look, so each copy can be painted or baked on its own (at less texture resolution each).
2. Offsetting each copy's UVs by one whole texture to the right (Blender's offset U). Copies stay full size but sit outside the texture square, which game engines tile (the copies look the same again) and Substance Painter reads as separate tiles only with UDIMs.

Caps (a start and an end object, as Blender's):
1. **Not now (chosen).** Each needs another object picked and placed at the ends, and a merge with it; a cap is easily modelled on the first or last copy after baking, or placed as its own object. Can be added later as two more sources.
2. Build them now: two more source pickers and their placing rules for line, grid and ring alike, for a rarely used option.

The mirror generator:
1. **Per-object symmetry is the mirror generator; nothing new (chosen).** Since decisions 0088 and 0126, symmetry is a live mirror on each object: a copy reflected across its pivot's X, Y or Z plane, the seam welded live, **Move the mirror to X, Y, Z** placing the mirror anywhere along the object's axes, **Apply mirror** baking it. It is worked out before the modifiers (0142), so an array of a mirrored shape repeats the whole shape. A mirror across another object's place is symmetry moved there with *Move the mirror to*; a ring of **Count 2** over 360° round another object gives a **turned** (point-symmetric) copy, as a propeller's blades, not a mirrored one.
2. A Mirror modifier as well: a second way to do the same thing, with the questions of which goes first.

The one gap: a mirror that *follows* another object live (Blender's *Mirror object*) — symmetry's mirror is placed by numbers, so it stays when the other object moves. It is rarely needed for game props and isn't built now.

## Decision

### The work (`mesh::array`, layer 0)

- `ArrayOptions` (`Copy`, `Default`, `clamped`): **Layout** (`ArrayLayout::Line`, `Grid`, `Radial`; standard Line), **Count** (line and ring; standard **3**, 1 to 1 000), **Counts** X, Y, Z (grid; standard 3, 3, 1, each 1 to 1 000), **Relative to size** (standard on), **Offset** (a line's step; standard 1, 0, 0) and **Spacing** (a grid's step per axis; standard 1, 1, 1), each within ±100 000 (shares or centimetres), **Angle** (standard **360°**, −360° to 360°), **Axis** (`ArrayAxis`, standard Z), **Radius** (standard 0, ±1 km), **Merge** (standard off), **Merge distance** (standard 0.1 mm, 0 to 1 m), **UVs side by side** (standard off). Values that aren't numbers become the standard ones.
- `copy_matrices(options, centre, size)`: one matrix per copy, the shape's own first. Relative offsets are multiplied by the shape's box size per axis (1 = touching; a flat shape has no size across, so its copies stack there). A ring's copy *k* is `T(centre) · R(axis, k · step) · T(−centre) · T(outward · radius)`.
- `array(input, options, centre, most)`: the first copy is the input itself, **keeping its ids** (selections, recorded edits and Scatter's chosen polygons still find their way), moved only for a ring with a radius; the others are appended after it (`parts::append`: UVs, hard edges, seams, creases, materials and other layers come along). Then merged as above. Refused, with a reason, above **100 000 copies** ("lower the counts") or above `most` polygons (`MAX_SHAPE_POLYGONS`). Empty input and one copy give the input unchanged. Deterministic.
- `work`: copies times the input's polygons over 8 (appending, as a scatter), plus copies times its points over 5 when merging (as Weld by distance); none when refused at once. Timings (release build, a standard sphere of 288 polygons): 100 copies 17 ms (25 ms merged), 1 000 copies (288 000 polygons) 0.17 s (0.26 s merged). So from about 140 copies of that sphere the viewport works on another thread.

### In the scene

`ModifierKind::Array(ArraySettings { options, centre: Option<ObjectId> })`, family **Generate**, after Subdivide on the palette and in Mesh → Modifiers. `sources()` is the centre, `takes_source` any object, `without_sources` keeps the array with no centre, `with_sources_renamed` follows copies, `reads_pivot` is true for a ring with no centre object (so moving the pivot re-works only that). The same result for the viewport and exports. Added at the end of the list (no `goes_before` rule: an array after a smoothing repeats the smooth shape, before it smooths each copy the same).

### In the interface

`CommandId::Array` ("**Array**", icon: three boxes in a row, drawn in code) in `OFF_THE_RING` and `modifiers::COMMANDS`: a click adds it to each selected shape. The Attributes panel (`attributes/array_rows.rs`): **Layout**; for a line **Count**, **Relative to size**, **Offset X/Y/Z**; for a grid **Count X/Y/Z**, **Relative to size**, **Spacing X/Y/Z**; for a ring **Count**, **Angle**, **Axis**, **Centre** (a list: *Its pivot* and the other objects) and **Radius**; then **Merge touching copies**, **Merge distance** (only while merging) and **UVs side by side**. Offsets show as plain numbers when relative, as lengths in the project's units otherwise. One drag is one undo step. **No new keys or mouse actions.**

### Saving

Development file format **version 54**: version 53's model with `KindV43::Array(ArrayV54)` (layout and axis as numbers, anything unknown read as the standard; the centre as an optional object id, checked as any source: missing, itself or a loop refused as damaged; settings clamped on reading). New sample `sample-v54.avtl`; files before 54 never hold it.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** The palette button, **Mesh → Modifiers → Array** (under *Generate*) and command search.
- **One Array modifier with a Layout choice** (Line, Grid, Radial) rather than three modifiers.
- Standard **Line of 3 copies, touching** (relative offset 1 along X); a grid **3 × 3 × 1**; a ring **8 copies over 360° round Z**.
- A ring turns round **the shape's own pivot** unless a **Centre** object is picked from a list in the Attributes panel; **Radius 0** as standard (the copies turn where the shape is, as in Blender).
- A ring of less than 360° is a **fan with copies at both ends**; 360° shares the turn out with no copy on top of the first.
- Deleting the centre object puts the ring **back on the pivot** (the array stays).
- **Merge touching copies off** as standard; when on, only **open borders** weld (closed copies stay closed), with a **0.1 mm** distance.
- **UVs side by side** shrinks each copy's UVs into its own square of the texture (rather than offsetting them outside it), off as standard.
- **No caps** (start and end objects) for now.
- **No separate mirror modifier:** per-object symmetry is the mirror generator.

## Consequences

- Projects with an Array need this version of the app or later.
- `SourceData::Place` lets a later modifier read where another object stands without working out its shape.
- Not yet: caps; a mirror that follows another object live; copies along a curve (Bend along curve on a line array does it in two steps); random offsets per copy; an "export copies as instances" option (as for Scatter, Godot doesn't read glTF GPU instancing, decision 0146).
