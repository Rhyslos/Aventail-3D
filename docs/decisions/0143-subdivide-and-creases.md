# 0143. Subdivide, and creases

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack and its recipe), [0053](0053-hard-and-soft-edges.md) and [0126](0126-rework-and-first-use.md) (hard edges, and each object's own Smoothing), [0043](0043-attribute-layers-and-the-builder.md) (edge layers carried by the builder), [0032](0032-edits-find-their-points-again.md) (recorded edits find their elements again), [0035](0035-heavy-meshes.md) (heavy work off the interface thread), [0136](0136-more-modifiers.md) and [0137](0137-biggest-first.md) (Phase 10's list and order)
- **Replaces part of:** [0142](0142-modifier-stack-and-remesh.md) (the development file format moves to version 44)

## Problem

Phase 10's fifth milestone is a smoothing modifier: Catmull-Clark subdivision with **Levels** for the viewport and **Levels for export**, and **creases** per edge (0 to 1) to keep edges sharp, set in the Edges mode. Hard edges should count as fully creased ("Hard edges stay sharp", on). UVs are smoothed with their borders kept, n-gons handled, and it must stay quick enough to drag points on a 10 000-polygon shape at level 2.

Left to decide: the name (the app already has a per-object **Smoothing** setting, which only shades), the method's details (semi-sharp creases, open borders, corners, UVs), where creases are kept and how they're set, which hard edges count, the controls, and speed.

## Options

The name:
1. **Subdivide (chosen).** Says what it does, can't be mistaken for the Smoothing row (shading only, decision 0126), and is the word most programs' users know (subdivision surface, subD). The manual calls it *subdivision smoothing* and warns about the difference.
2. Smooth, or Smoothing modifier: reads the same as the per-object Smoothing row, which changes the shading but not the shape.
3. TurboSmooth, Subdivision Surface: other programs' product names.

Which hard edges count as creased:
1. **Only marked hard edges, and only on objects shaded by their marked edges (chosen).** New shapes are smoothed by angle (30°, decision 0126) and the primitives mark their box edges hard. On an object smoothed by angle every corner of a cage is sharper than the angle, so counting the angle's edges, or the primitives' invisible marks, would stop a cube from rounding at all: the first thing anyone tries. Counting marks only where the shading shows them means the edges that *look* hard are the ones kept sharp.
2. The edges the shading shows hard, whatever the object's smoothing: a cube with the standard settings would stay a cube.
3. Every marked hard edge: the same, through marks the user can't see.

Semi-sharp creases:
1. **Sharp for `crease × levels` levels (chosen):** 0.5 is sharp for half the levels and then smoothed; 1 is sharp at every level. The look stays in proportion between the viewport's and the export's levels.
2. Blender's `10 × crease²` levels: made for its limit surface; at 1 or 2 levels nearly every crease above 0.3 looks fully sharp.

How creases are set:
1. **A recorded edit, `Operation::SetCreases` (chosen),** like hard edges and seams: undone with the rest, mirrored with symmetry, finding its edges again after earlier edits change (anchors, decision 0032).
2. A per-object list beside the modifier: lost as soon as the edges it names change.

The E ring's place for the creases (the modelling ring's 15 places are all taken):
1. **Flip normals' place in the Edges mode (chosen),** where Flip normals (polygons only) leaves a gap, right beside Hard and Soft edges: a group, **Creases**, opening **Crease edges** and **Uncrease** around it, as the UV ring's turns do (decision 0109). Every other command keeps its place.
2. Swapping Hard and Soft edges for a shading group: moves commands people already know.

## Decision

### The modifier

`ModifierKind::Subdivide(SubdivideOptions)` (family Generate, first on the palette and in Mesh → Modifiers), following decision 0142's recipe. `SubdivideOptions` (in `mesh::subdivide`): **Levels** (standard **1**), **Levels for export** (standard **2**), both 0 to 5, and **Hard edges stay sharp** (standard **on**). `scene::subdivide_levels(options, purpose)` picks the level for the viewport or an export; Bake into shape uses the export's. A result over 4 million polygons is refused with a reason ("Lower the levels"), and the shape goes on unsmoothed.

`ModifierContext` gains `hardness`, the object's Smoothing (decision 0126), and the stack's cache remembers it per step, so changing the Smoothing works the modifier out again. *Hard edges stay sharp* applies when the hardness is `Marked`. Other modifiers that treat hard edges specially (decimate, weighted normals) read the same field.

`ModifierKind::differs_by_purpose` tells the Attributes panel to check an export's result too: a Subdivide whose export levels can't be made says so, even when the viewport's can.

### The method

`mesh::subdivide::subdivide(mesh, levels, hard_sharp)` (layer 0), Catmull-Clark, each level on plain lists (`subdivide/step.rs`), the mesh built once at the end:

- **Points:** a polygon's middle; an edge point (the middle of its ends and its two polygons' middles; its own middle where sharp; blended for a partial sharpness); each old point by the smooth rule `(Q + 2R + (n − 3)P) / n`, the crease rule `(E₁ + 6P + E₂) / 8` with two sharp edges, kept with three or more, blended by the average sharpness of its sharp edges below 1.
- **Sharpness:** an edge's crease gives it `crease × levels` levels (1 never wears off), less one each level; marked hard edges, with `hard_sharp`, never wear off. Open borders and edges shared by three or more polygons are always sharp.
- **Corners kept:** a point on an open border with only one polygon (a sheet's corner) stays put.
- **N-gons** give one quad per corner; triangles three. Zero-length edges and flat polygons give real numbers; loose points are left out.
- **UVs** are subdivided as their own surface: corners at the same point with the same UV (to 0.00001) are one UV point; where the UVs split (a seam) each side is an open border there, so it is smoothed along itself and each island keeps to itself; a UV corner with one polygon stays put (Blender's *keep corners*). They use the edges' sharpness too, so they follow the shape.
- **Carried over:** every polygon value (material and the UV tools' own) from the old polygon; hard edges, seams and full creases on both halves of their edges; partial creases are used up. Point values and other corner values aren't carried.
- **Edges** of each level are found by a counting sort on their lower point (no hash map); a level's UVs are worked out on a second thread from 20 000 corners.

Timings (release build, a sphere of 10 240 polygons with UVs): level 1, 40 704 polygons in 14 ms; level 2, 162 816 in 43 ms; level 3, 651 264 in 170 ms. `ModifierKind::work` counts 16 new polygons as one polygon of a boolean's worth, so from about 80 000 new polygons (a 5 000-polygon shape at level 2, a 20 000 one at level 1) the viewport works it out on another thread, showing the last result meanwhile (decision 0142). Dragging points on a 10 000-polygon shape at level 2 keeps the window smooth; the smoothed shape follows about 45 ms behind.

### Creases

- **Kept** in the mesh's `crease` edge layer (`mesh::crease::CREASE`, a number 0 to 1 per edge, only where not 0; decision 0043 named it so from the start), carried to new edges by the builder like hard edges and seams, and saved with plain meshes as every layer is.
- **Set** by `Operation::SetCreases { edges, crease, anchors }` (`mesh::crease::set_creases`): 0 takes them away, values are brought within 0 to 1, and an edit with a crease outside 0 to 1 (or not a number) is refused. Undo labels "Crease 3 edges", "Uncrease 3 edges". Mirrored with symmetry like hard edges.
- **The commands:** **Crease edges** (crease 1) and **Uncrease** (0), for the selected edges in the Edges mode, in the E ring's **Creases** group (Flip normals' place, Edges mode only, `commands::EDGES_ONLY`), **Mesh → Shading** and command search. Elsewhere they say to press 2.
- **The Crease field:** the Attributes panel's **Selected edges** section (Edges mode, edges selected) shows their count and crease (the first's, with a note when they differ), 0 to 1. A drag records one edit at its start and changes it as it goes (`Command::ReplaceOperation` in the same undo step), so one drag is one edit and one undo step.
- **Drawn** magenta (`OverlayStyle::Crease`) in the Points, Edges and Polygons modes, as thick as hard edges.

### Saving

Development file format **version 44**: version 43's model with `KindV43::Subdivide { levels, export_levels, hard_edges_sharp }` and `OperationV37::SetCreases { edges, crease, anchors }`; files before 44 hold neither. Levels are clamped on reading; a crease outside 0 to 1, or an edge from a point to itself, is refused as damaged. New sample `sample-v44.avtl`.

## Choices to check

Made for the project lead (decision 0028), each the common choice that clashes with nothing:

- The name **Subdivide**, with the manual explaining it as subdivision smoothing, not the Smoothing row.
- **Levels 1** and **Levels for export 2**; at most 5 levels and 4 million polygons.
- **Hard edges stay sharp** on, counting only edges marked hard, and only on objects shaded by their marked edges (on objects smoothed by angle, the standard, nothing would round).
- A partial crease is sharp for `crease × levels` levels (0.5 for half of them).
- Open borders stay where they are, smoothed along themselves, and a sheet's corners stay put; the same for UVs (*keep corners*).
- **No keys:** Crease edges and Uncrease in the E ring's Creases group (Flip normals' place in the Edges mode), Mesh → Shading and command search; the Crease field in the Attributes panel. "Crease edges" sets a full crease at once (no "…": nothing more is asked); the field sets softer ones.
- Creased edges drawn magenta whenever points, edges or polygons are edited, with no switch.

## Consequences

- Projects with Subdivide or creases need this version of the app or later.
- Creases are edits like hard edges: they survive later edits and edit layers, and show in the edit list.
- Every modifier now gets the object's Smoothing in its context; a modifier's cached result is redone when it changes.
- Displace, weighted normals, live bevel and the others still to come can follow the same pattern for an edge layer of their own (bevel weights): a mesh layer, a recorded edit, an Attributes field for the selected edges.
