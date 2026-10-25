# 0159. Struts (the Wireframe modifier)

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack and its recipe), [0154](0154-shell.md) (Shell: hard edges and UVs for what a modifier makes, and where a new one goes), [0150](0150-quadify.md) and [0152](0152-weld-by-distance.md) (Quadify and Weld by distance go before what reads the edges), [0063](0063-wireframe.md) (the viewport's Wireframe look), [0043](0043-attribute-layers-and-the-builder.md) (materials and values carried to new polygons), [0053](0053-hard-and-soft-edges.md) (hard edges), [0028](0028-ask-about-controls.md) (ask about controls), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0158](0158-bend-twist-taper.md) (the development file format moves to version 60)

## Problem

Phase 10's twenty-first milestone is a **Wireframe** modifier: every edge becomes a strut with a **Thickness**, **Even thickness**, **Keep the faces** (off) and **Sides** (4 a square strut, more for round), with joints at the points. Each strut is closed, with UVs per strut; hard edges are the profile's corners.

Frames, cages, grilles, scaffolding, lattice towers, a crate's metal edging and geodesic domes are all bars along the edges of a simple shape. Modelling each bar by hand is slow and can't be changed afterwards.

Left to decide: the name (the viewport already has a *Wireframe* look, decision 0063), whether the struts are welded into one surface or are separate pieces, what a joint is and how big, which way a strut's profile turns, what *Even thickness* means, the standard settings, hard edges, UVs, materials, ids, where a new one goes in the list, the controls, cost and saving.

## Options

The name:
1. **Struts (chosen):** says what it makes (bars), and can't be mixed up with **Display → Wireframe**, which only draws edges and exports nothing. The manual, the concepts page and the modifier's tooltip each say how the two differ.
2. *Wireframe* (Blender's and the list's name): two different things called Wireframe in one app, one a look and one a modifier, with opposite effects on the export.
3. *Wireframe solid*: clearer than plain Wireframe, but still reads as a viewing mode at a glance, and is long for the palette's tooltip and the list.
4. *Lattice* (3ds Max's name for this): taken by the lattice cage (decision 0144).

Welded or separate pieces:
1. **Separate closed pieces, overlapping at the joints (chosen),** as 3ds Max's *Lattice* modifier: a prism per edge and a ball per point. Works on any input (n-gons, non-manifold edges, open borders, folds, zero-length edges), can't fold or tear however edges meet, every piece is closed (renders from any side, bakes cleanly), and a strut's sides unroll exactly for its UVs. To join them, Remesh after it.
2. One welded frame, as Blender's *Wireframe* (each polygon's border inset and thickened): one manifold surface, but the strut's profile is set by the polygons (no *Sides*: always four-sided), it folds where polygons are thin or edges meet at sharp angles, and needs a manifold, consistently facing input.

The joints:
1. **A closed faceted ball round each point that a polygon uses, every face exactly the struts' corner radius from the point (chosen):** just big enough to hide every strut's end (each end is a polygon that size round the point). As many sides round as the struts, and half as many rings (at least two), so square struts meet in a cube and round ones in a ball. Its rings lie along the facing of the point's first polygon (lowest id) and one flat faces the point's first edge, so on a box the joints are square with it, and on a flat grid with the struts.
2. A separate *joint size*: one more setting for little gain; the struts' ends would show with a smaller one.
3. No joints: the struts' square ends stick out past each other at every corner.

Which way a strut's profile turns:
1. **A flat facing the way the edge's first polygon (lowest id) faces (chosen):** on a flat grid the frame lies flat; on a box, a square strut's two flats face the box's two sides (the box's corners are square with its sides).
2. The mean of the edge's polygons' facings: on a box this turns square struts 45°, diamond-wise to its sides.

Even thickness:
1. **On: Thickness is measured across the profile's flats; off: across its corners (chosen).** On, every strut is exactly Thickness wide whatever its sides (a 2 cm square bar is 2 cm across); off, it fits inside a round rod of that thickness (as a cylinder primitive fits its radius), for struts that must go through a round hole. Separate pieces need no other evening out: a strut's walls are the same thickness all along.
2. Blender's meaning (corners of a welded frame pushed out by the angle): applies only to a welded frame, which wasn't chosen.

UVs:
1. **Each strut and each joint an island of its own (chosen):** a piece's rings laid flat as rows (each as long as its ring is round, centred, as far apart as the bands between them are high), so a strut's sides unroll into an exact rectangle and a joint's into a lens; each end polygon laid flat on its own plane and hung off the first side of its row. Windings kept, so normal maps bake the right way round. All islands at one scale (even texel density), shelf-packed tallest first with a small gap, into the texture square.
2. One box projection over everything (as Remesh): overlapping and stretched on slanted struts.

## Decision

### The work (`mesh::struts`, layer 0)

- `StrutsOptions` (`Copy`, `Default`, `clamped`): **Thickness** (centimetres, standard **2 cm**, 0 to 100 m; a negative one is its size, not a number is the standard), **Sides** (standard **4**, 3 to 24), **Even thickness** (standard **on**), **Keep the faces** (standard **off**).
- `struts(input, options, most)`: a strut for every edge of every polygon (`pieces::strut`: two rings of the profile round its ends, its corners the corner radius from the edge, a quad per side and a polygon closing each end) and a joint for every point a polygon uses (`pieces::joint`). Edges with no length get no strut; points no polygon uses (loose points) get no joint. With Keep the faces, the input comes first with its ids, UVs and values (custom normals dropped, decision 0148); without, the result is new. A thickness of 0, or no polygons, gives the input as it is; more than `most` polygons (`MAX_SHAPE_POLYGONS`) is refused with a reason ("fewer sides, or a simpler shape"). Deterministic.
- **Hard edges:** profiles of **6 sides or fewer** (`HARD_UP_TO`) have hard corners along the struts and hard joints; every strut's ends are hard where they meet its sides; round struts (7 or more) and their joints are smooth.
- **Materials:** a strut takes the material of its edge's first polygon, a joint of its point's first polygon.
- **UVs** (`struts/layout.rs`) as above; with Keep the faces, the struts' UVs go in the square **above** the texture square, so the shape's own stay as they were (as Shell's rims sit above it, decision 0154). Seams are marked on every new edge where the UVs split (each piece's border).
- `polygon_count` and `work` (polygons ÷ 4, as Shell).

### In the scene

`ModifierKind::Struts(StrutsOptions)`, family **Generate**, after Shell on the palette and in Mesh → Modifiers, following decision 0142's recipe. It reads no other object, gives the viewport and exports the same, and carries no custom normals. `goes_before`: a new Struts goes before any Live bevel, Subdivide or Weighted normals (a bevel rounds the struts' corners, Subdivide makes square struts soft; Weighted normals shades best last); a new **Weld by distance** goes before it (a crack would give two struts along one edge), and so does a new **Quadify** (a triangulated shape would give a strut across every quad).

### In the interface

`CommandId::Struts` ("**Struts**", its icon a box made of bars with a block at each corner, drawn in code) in `OFF_THE_RING` and `modifiers::COMMANDS`: a click adds it to each selected shape. Its tooltip says it isn't the Wireframe look. Its Attributes rows (`attributes/struts_rows.rs`): **Thickness** (in the project's units), **Sides**, **Even thickness** and **Keep the faces**; one drag is one undo step. **No new keys or mouse actions.** (It isn't a tool in use, so decision 0048's Ctrl + wheel doesn't apply: Sides is a field with step arrows.)

With its new button, the **modifiers palette** no longer fits a 900-point-high window (it was already cut off at 800). It now **scrolls with the mouse wheel** when it's too short (an egui scroll area with no scroll bar, as the column is too thin for one), so its last buttons (Weighted normals, Game bevel) can always be reached.

### Saving

Development file format **version 60**: version 59's model with `KindV43::Struts(v60::StrutsV60 { thickness, sides, even, keep_faces })`, clamped on reading. New sample `sample-v60.avtl` (a cube with standard struts, and a cube with every setting changed, hidden in the viewport), whose shapes the tests check come out exactly the same.

### Timings

Release build, a sphere of 64 sides by 32 rings (4 032 edges): square struts 60 ms (36 108 polygons), 8 sides 127 ms (91 956 polygons). `work` counts that as about 9 000 and 23 000, so both are worked out on another thread in the viewport (from 5 000).

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- The name **Struts**, not "Wireframe" (which stays the viewport's look), as recommended. "Wireframe solid" was the alternative.
- **No new keys or mouse actions.** The palette button (after Shell), **Mesh → Modifiers → Struts** (under *Generate*) and command search.
- Standard **Thickness 2 cm**, **Sides 4**, **Even thickness on**, **Keep the faces off**.
- **Separate pieces** overlapping at the joints, not one welded frame (Remesh after it joins them).
- **Joints always**, just covering the struts' ends: cubes for square struts, balls for round ones. No setting for their size.
- **Even thickness** means measured across the flats (on) or the corners (off).
- **Hard corners up to 6 sides**; 7 or more shade round.
- Each strut and joint its own **UV island**, packed into the texture square; with Keep the faces, **in the square above it**.
- A strut's flat faces its edge's **first polygon** (square with a box's sides); a joint lines up with its point's first polygon and first edge.
- A new Struts goes **before any Live bevel, Subdivide or Weighted normals**; a new **Weld by distance** or **Quadify** goes before it.
- The modifiers palette **scrolls with the mouse wheel**, with no scroll bar, when the window is too short for every button.

## Consequences

- Projects with Struts need this version of the app or later.
- The struts overlap at the joints: fine for rendering, baking and game engines; a 3D print or a boolean wants one surface (Remesh after it).
- Polygons add up quickly (a square strut and a cube joint are 6 polygons each, a round one many more); the manual says to keep Sides low for games.
- Not yet: a joint size or no joints, struts only on chosen or hard edges, a thickness per point, a welded (Blender-style) frame.
