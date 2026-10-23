# 0135. Live booleans: union, subtract, intersect

- **Status:** Accepted
- **Date:** 2026-10-23
- **Builds on:** [0006](0006-base-plus-operation-stack.md) (base plus operation stack), [0033](0033-heavy-mesh-safeguard.md) and [0035](0035-heavy-meshes.md) (heavy work off the interface thread), [0088](0088-live-mirror.md) and [0126](0126-rework-and-first-use.md) (the live mirror on the object, Bake into shape, the development file format), [0132](0132-one-language-layer.md) (interface text through the language layer)
- **Replaces part of:** [0126](0126-rework-and-first-use.md) (one development file format: booleans get version 40)

## Problem

Phase 10's first milestone: booleans (union, subtract, intersect), the classic hard-surface tool for cutting holes, notches and slots, and for merging parts. The project lead chose (2026-10-23):

- **Live cutters.** The cutting shape stays an object of its own, drawn as an outline; moving, turning, scaling or editing it moves the cut. *Bake into shape* makes the cut permanent.
- **Started from the menu, no new keys or mouse actions:** select the shape to keep, Ctrl + click the cutters, and choose **Union**, **Subtract** or **Intersect** in the Mesh menu, the modifiers palette or command search. The first selected is kept; the others cut it.

Left to decide: which library, where the boolean lives, how the result is made into clean polygons, what happens with shapes that can't be cut, how the app knows which object was selected first, how heavy cuts keep the window responsive, saving, exporting and the panels.

## Options

For the library (DESIGN.md §3 named Manifold):
1. **`manifold-rust` (chosen).** A pure-Rust port of Manifold (Apache-2.0), so no C++ toolchain or bindings; robust (exact predicates, symbolic perturbation for coplanar faces), deterministic, and it keeps a face id on every triangle through the boolean. Its dependencies are permissive (clipper2-rust BSL-1.0, dashu MIT/Apache-2.0, num-modular Apache-2.0).
2. `manifold3d` through C++ bindings: the original, but a C++ build on every platform.
3. `csgrs` (MIT, BSP trees): simpler, but BSP booleans are slower and fragile on coplanar faces. Kept as the fallback had Manifold not worked.

For where the boolean lives:
1. **A list on the kept object (chosen),** like its live mirror: each boolean a kind and the cutters (by object id) that do it, in order, worked out when the shape is shown. Changed only through commands, so undo works.
2. An operation in the kept object's edit stack: the stack's edits find their elements by id, but a boolean's result is new geometry every time a cutter moves, so edits on top of it couldn't hold; and a cutter is another object, which the stack's operations never refer to.

For which object is kept:
1. **The one selected first (chosen).** The app didn't know the order things were selected in (the selection was a set, listed in the Objects panel's order), so the document now keeps the selection's order: what stays selected keeps its place, what is added goes at the end. Undo puts the order back too.
2. The highest in the Objects panel (as Join does): no new bookkeeping, but new objects go to the top of the list, so the usual "make a cube, then the cylinder to cut it" would keep the cylinder.

## Decision

**Where it lives.** `Object::booleans`, a list of `Boolean { kind, cutters }`, applied in order, each boolean's cutters one after another. `Command::AddBoolean` (from the menus; added to the last boolean when that is of the same kind, else a new one; selects the kept object) and `Command::SetBooleans` (the Attributes panel: change a kind, take a cutter out, remove a boolean) change it, through `Edit::SetBooleans`. Refused, with a reason: a group or an instance as the kept object, a group as a cutter, a shape cutting itself, a loop (the cutter's shape already uses the kept one, through its own booleans or as an instance), no cutter, more than 64.

**How the shape is worked out.** The kept object's edits still work on its uncut shape (`Scene::evaluated`): picking and the Points, Edges and Polygons modes see it as before. `Scene::cut_shape` is the cut: the evaluated shape with each cutter's whole shape (cut by its own booleans, with its live mirror) placed into the kept object's space, worked out by the new `booleans` crate (layer 1) and cached until the shapes, places, kinds or smoothing change. `Scene::shown_shape` (drawn, exported, applied) is the cut with the live mirror after it, so a hole cut in one half is mirrored too. An instance of a cut shape shows the cut.

**The result's polygons.** Manifold works on triangles, each tagged with the polygon it came from. The triangles from one polygon, if they still form a disc, become one polygon again (a quad cut by a cylinder becomes an n-gon round the cut); a ring round a hole becomes convex polygons (its triangles joined wherever the result stays convex). Each polygon keeps its source's material (a cutter's materials join the kept object's by name, else its first) and its corners' UVs worked out where they now are (exactly along original edges, blended inside); the cutter's faces bring the cutter's UVs. Edges along an original edge keep its values (hard, seam); the new edges along the cut are seams where there are UVs (the UVs part there) and hard where the surfaces meet more sharply than the kept object's smoothing angle (30° when it's smoothed by marked edges only).

**Shapes that can't be cut.** Manifold needs closed solids. A shape with holes in its surface, edges shared by more than two polygons, or a cutter squashed flat leaves the kept object shown uncut, and the Attributes panel says why, naming the cutter. Points at the same place along open edges are welded first, so a closed surface made of separate pieces still counts. Nothing panics: the library is called inside `catch_unwind` as a last resort.

**Cutters.** Drawn as an outline (edges only, no surfaces, never fading away) and left out of exports, as the shape they cut carries the result. A click picks a cutter only where no other shape is under the mouse, so clicking the result picks the kept object; a cutter can always be picked in the Objects panel or from the kept object's Attributes. Hidden cutters still cut. Deleting a cutter takes it out of every boolean in the same undo step. Duplicating a kept object with its cutters gives the copy the copied cutters.

**Bake into shape** bakes a working cut into the kept object's shape (with its edits, as before), ends its booleans and hides each cutter that no longer cuts anything (shown, it would cover the result as a solid shape again); the cutters are kept, so nothing is lost. **Apply mirror** bakes the cut too, as it bakes what is shown.

**Heavy cuts** (5 000 polygons or more, kept shape and cutters together) are worked out on another thread, as heavy shapes' drawing and picking are (decisions 0033, 0035): the scene hands out a `CutJob`, the drawing cache runs it and gives the result back (`Scene::store_cut`), showing the last cut meanwhile, one job per object at a time. Lighter cuts are worked out at once. A 2 048-polygon sphere cut by a cylinder takes about 20 ms in a release build (the boolean itself about 10 ms), so dragging a cutter stays interactive for ordinary game assets.

**Saving.** Development file format version 40: version 39's model plus each object's booleans, still CBOR. Booleans get a version of their own because an older app would skip them silently (showing the shapes uncut and the cutters solid, and losing the booleans on saving); with a new version it says it needs updating instead. Older files open. A file's booleans are checked like the menus' (no missing objects, no loops).

**The interface.** Mesh → Booleans, three buttons on the modifiers palette under Add edit layer (icons drawn in code: a square and a circle overlapping), and command search, each greyed out with the reason when the selection doesn't fit. No new keys. The Attributes panel's **Booleans** section lists the kept object's booleans (kind changeable, each cutter by name, a click selecting it, Take out, Remove) and tells a cutter what it cuts.

## Consequences

- Projects with booleans need this version of the app or later.
- `scene` depends on the new `booleans` crate (same layer); `manifold-rust` is the app's first geometry library.
- Hiding polygons of a kept object hides them from editing only: the cut is worked out from the whole shape.
- Join, Split off and the mesh check work on the uncut shape; bake first to work on the cut.
- A ring round a hole is several convex polygons, not one n-gon: the app's polygons can't have holes.
- The document now keeps the selection's order (`Document::selection_order`), which later tools can use for "the first selected".
