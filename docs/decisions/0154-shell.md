# 0154. Shell

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack and its recipe), [0148](0148-weighted-normals.md) (custom normals dropped by later modifiers; Weighted normals best last), [0149](0149-live-bevel.md) (Live bevel, and `goes_before`), [0152](0152-weld-by-distance.md) (Weld by distance goes before what reads how polygons join), [0153](0153-array.md) (Array: the first copy keeps the shape's ids), [0043](0043-attribute-layers-and-the-builder.md) (the builder carries values to new elements), [0053](0053-hard-and-soft-edges.md) and [0126](0126-rework-and-first-use.md) (hard edges, each object's Smoothing), [0028](0028-ask-about-controls.md) (ask about controls), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0153](0153-array.md) (the development file format moves to version 55)

## Problem

Phase 10's sixteenth milestone is a **Shell** modifier: a surface given thickness, with a **Thickness** (recommended 1 cm), a **Direction** (outward, inward or both sides), **Even thickness** (on) and **Fill the rims** (on). The rims get UVs with seams along them; open and closed shapes both work; sharp corners keep the thickness even by moving further by the angle; hard edges at the rims. Self-intersections aren't fixed, and that is documented.

Game props are full of thin things with a thickness: sheet-metal panels, armour plates, a cup's or a bucket's wall, a helmet, a cloth's hem, a lampshade, pipes cut open, signs. Modelling them as a surface and giving it thickness live means the shape stays easy to edit, and the thickness can change at the end.

Left to decide: the standard direction, how "even" is worked out, how far a corner may move, which side keeps the ids, what the layer behind and the rims get (UVs, materials, hard edges, seams), where a new Shell goes in the list, the controls, cost, and saving.

## Options

The standard direction:
1. **Inward (chosen),** as in Blender's Solidify: the surface you modelled stays where it is, as the outside, and the thickness grows behind it. A shape modelled to size stays that size outside, and anything fitted against it (snapping, other parts) stays fitted.
2. Outward, as 3ds Max's Shell: the surface stays the inside, which suits a room modelled from within; the shape grows by the thickness.
3. Both sides: the surface in the middle of the wall; neither side keeps its size.

Even thickness:
1. **Each point moved so every polygon round it ends up the thickness away (chosen):** the offset `v` with `v · n = 1` for each polygon facing `n`, found by least squares weighted by the corner angles when they can't all be met (a cone's tip). On a flat part this is the facing; along a box's edge the half-way direction, longer by 1 / cos(half the angle); at a box's corner (1, 1, 1), so every wall of a thickened box is exactly the thickness.
2. The angle-weighted point normal lengthened by the mean of 1 / cos of its angles to the polygons (Blender's way): close, but not exact where three or more different polygons meet at a corner.

How far a corner may move:
1. **At most 5 times the thickness (chosen):** a spike sharper than about 23° is held back, so a needle-thin spike or a fold can't shoot a point off into the distance. Less sharp corners (a box's: 1.73 times) are exact.
2. No limit: one stray sliver polygon could fling points kilometres away.

Which side keeps the ids:
1. **The surface in front (the original's facing side) keeps its points' and polygons' ids (chosen),** as Array's first copy does: selections, recorded edits and Scatter's chosen polygons still find their way. The layer behind and the rims get new ids after them.

The rims' UVs:
1. **Each rim strip laid flat in a row of its own just above the texture square (chosen),** at the shape's texel density (length along U, thickness up V), rows stacked upward: never overlapping the shape's own UVs, as sharp as the surface's texture, and easy to move with Pack after baking. The layer behind copies the surface's UVs (it shows the same texture, as the two sides of a sheet usually do).
2. The rims' UVs hung off the edge of each island: overlaps other islands.
3. Rims squeezed into the square: they would land on top of the shape's UVs.

Where a new Shell goes in the list:
1. **Before the first Live bevel, Subdivide or Weighted normals already there, else at the end (chosen),** the usual order (thicken, bevel, smooth, weighted normals last): the rims are bevelled and smoothed with the rest, and Weighted normals keeps its normals (a Shell after it would drop them, 0148). A new **Weld by distance** now also goes before a Shell, since a crack would grow a rim along it.
2. At the end like every other kind: added after Weighted normals it would show the amber "these normals are dropped" note at once.

## Decision

### The work (`mesh::shell`, layer 0)

- `ShellOptions` (`Copy`, `Default`, `clamped`): **Thickness** (centimetres, standard **1 cm**, 0 to 100 m; a negative one is its size, not a number is the standard), **Direction** (`ShellDirection::Inward` standard, `Outward`, `Both`), **Even thickness** (standard **on**), **Fill the rims** (standard **on**).
- `offsets::offsets(mesh, even)`: each point's offset for a thickness of 1, from its polygons' facings weighted by their corner angles (polygons with no area and sides with no length count for nothing; facings that cancel out leave the point where it is); even as above, worked in double precision, at most `MAX_STRETCH` (5) long.
- `shell(input, options, most)`: the surface (its ids kept) moved by the front share of the thickness (0, 1 or ½ along the offset); a copy of every point and polygon, turned to face the other way, moved by the back share (−1, 0 or −½), carrying corner, polygon and edge values through the builder (UVs, materials, hard edges, seams, creases, bevel weights); then, with Fill the rims, one quad per open border edge joining the two, its corners and material from the border's polygon. Each rim's edges along the two layers are marked hard and, if the shape has UVs, as seams, as is the cut where a closed border's strip starts (any rim edge across where the UVs split). Custom normals are dropped (0148). A thickness of 0, or no polygons, gives the input as it is; more than `most` polygons (`MAX_SHAPE_POLYGONS`) is refused with a reason. Non-manifold edges and edges whose polygons face opposite ways aren't borders and get no rim; nothing panics on n-gons, zero-length edges, flat polygons or folds. Deterministic.
- **Not fixed:** where the thickness is more than a part is wide, the two layers pass through each other (a closed shape thickened inward by more than half its width, a narrow slot). The manual says so, with the fix: a smaller thickness or Outward.

### In the scene

`ModifierKind::Shell(ShellOptions)`, family **Generate**, after Array on the palette and in Mesh → Modifiers, following decision 0142's recipe. It reads no other object, gives the viewport and exports the same, and carries no custom normals. `work` is the result's polygons ÷ 4 (10 000 polygons thickened in about 20 ms in a release build; 65 536 in 0.16 s), so from about 10 000 input polygons the viewport works it out on another thread. `goes_before`: a Shell before Live bevel, Subdivide and Weighted normals; Weld by distance before Shell.

### In the interface

`CommandId::Shell` ("**Shell**", its icon a bowl-shaped wall cut through, its two rims showing on top, drawn in code) in `OFF_THE_RING` and `modifiers::COMMANDS`: a click adds it to each selected shape. Its Attributes rows (`attributes/shell_rows.rs`): **Thickness** (in the project's units), **Direction** (Inward, Outward, Both sides), **Even thickness** and **Fill the rims**; one drag is one undo step. **No new keys or mouse actions.**

### Saving

Development file format **version 55**: version 54's model with `KindV43::Shell(ShellV55)` (thickness, direction as a number, anything unknown read as inward, even thickness, fill the rims; clamped on reading). New sample `sample-v55.avtl`; files before 55 never hold it.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- **No new keys or mouse actions.** The palette button, **Mesh → Modifiers → Shell** (under *Generate*) and command search.
- Standard **Thickness 1 cm**, **Inward** (the surface stays the outside, as in Blender; Outward and Both sides are in the list), **Even thickness on**, **Fill the rims on**.
- Even thickness holds a corner back at **5 times the thickness** at most.
- The **rims' UVs** sit in rows **above the texture square**, at the shape's texel density; the **layer behind shares the surface's UVs**.
- Rim edges are marked **hard** and as **seams**.
- A new Shell goes **before any Live bevel, Subdivide or Weighted normals**; a new Weld by distance goes before a Shell. Up and Down still move them anywhere.
- Self-intersections aren't fixed; the manual explains when they happen.

## Consequences

- Projects with a Shell need this version of the app or later.
- The rims' UVs lie outside the texture square: a texture that tiles shows on them as it is; to paint them in Substance Painter, bake into shape and Pack the UVs (or unwrap) first. The manual says so.
- Not yet: a different thickness per point (a vertex group or weight), a rim-only or "shell only the border" mode, a material of its own for the rims or the inside, and fixing self-intersections.
