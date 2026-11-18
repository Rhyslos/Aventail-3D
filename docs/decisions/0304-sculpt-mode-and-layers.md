# 0304. Sculpt mode, Sculpt layers, Draw and Smooth (Phase 17a, milestone 1)

- **Status:** Accepted, built while the project lead was away; the plan's choices and the new ones below wait for review in [docs/plans/phase-17.md](../plans/phase-17.md#choices-to-review) (2 changed; 26 to 30 new)
- **Date:** 2026-11-17
- **Follows:** [0300](0300-sculpting-plan.md) (the sculpting plan), [0006](0006-base-plus-operation-stack.md), [0035](0035-heavy-meshes.md), [0042](0042-fast-edits.md), [0091](0091-edit-layers.md), [0065](0065-cavity.md)

## Problem

Sculpting's first milestone: a Sculpt mode and layout, sculpted detail kept in a Sculpt layer of the object's stack, saved with the project, two brushes (Draw, and Smooth on Shift), and strokes that stay smooth on the 3-million-polygon heavy test shape. The plan (0300) sets the shape; building it needed a few choices the plan doesn't make, and one it made changed: on 2026-11-17 the project lead asked for sculpting to be built now, beside scripting, rather than after rigging and animation.

## Options

1. **Where a Sculpt layer's data lives.** In a store beside the paint (`Scene::sculpts`, a sculpt named by an id in the layer's one edit), as the plan drew it; or in the edit itself (`Operation::Sculpt { sculpt }`, a cheap shared handle). The store needs its own undo, saving and a way to tell the stack's cache that a sculpt changed; the edit gets all three from the stack as every edit does.
2. **How a changed shape under the sculpt is fitted again.** By point ids (cheap, wrong when a primitive's segments renumber every point); or by position, keeping where the sculpted shape's points were (12 bytes a point, shared by every stroke).
3. **Finding the surface and the points in reach while points move.** The picking tree (`mesh::bvh`, rebuilt after a change: too slow per frame on millions of polygons); a grid of points (stale as points move); or a tree of boxes round the polygons, built once and refitted round the polygons that moved.
4. **Whether evaluating the stack after each stroke works the Sculpt layer out again.** It always would; or the scene takes the live sculpt's shape as the layer's result (exactly what working it out gives) when the stroke is sent.
5. **Matcaps:** pictures made at start-up and sampled; or worked out in the shader from the facing on screen.

## Decision

- **The `sculpt` crate** (layer 1, `mesh`, `glam` and `words` only): `Sculpt` (the structure of the shape it was sculpted on, where its points were, and each point's move measured along its surface frame, in chunks of 16 384 points shared between copies; `apply`, `fitted_to`, `memory_apart`), `frames` (a point's facing, area-weighted, and a direction towards its first neighbour, worked out the same way everywhere), `brush` (Draw and Smooth, four falloffs, settings kept per brush), `tree::FaceTree` (boxes round polygons in Morton order, refitted round moved polygons), and `LiveSculpt` (the shape under a stroke: dabs on every core for big brushes, `finish` turning moved points into moves and setting the shape to exactly what evaluating them gives, `cancel`, `follow` for undo and redo, chunk by chunk).
- **In the scene:** `Operation::Sculpt { sculpt }`, the one edit of a **Sculpt layer** (option 1, the second way). `Command::Sculpt { id, sculpt }` puts a stroke's sculpt in the top layer when it is a switched-on Sculpt layer, or a new "Sculpt" layer on top; ordinary edits after sculpting go into a new "Edits" layer, and sculpting after them into "Sculpt 2". Merging a Sculpt layer is refused in plain words. Fitting by position, with the layer's "fitted" badge (option 2, the second way). The undo history counts only the chunks a stroke changed, before and after. `Scene::sculpt_target` gives the layer and the shape coming into it; `Scene::prime_sculpt` takes the live shape as the layer's result (option 4, the second way).
- **File format 112** (`fileio::project::v112`): the structure, the places and each moved chunk as little-endian numbers packed with zstd; untouched chunks aren't stored; every count checked before unpacking, moves that aren't numbers refused; a fuzz test, a sample file and the 111 sample opening without sculpts.
- **Drawing:** `render::SurfaceDraw::Matcap { look }` (shader mode 6: Clay, Red wax, Pale stone, worked out from the facing on screen, option 5 the second way), cavity on and edges off in Sculpt mode. During a stroke the drawing shows the live shape; the renderer's existing path updates only the points that moved.
- **The interface:** a **Sculpt** button in the left palette between Polygons and UV, a **Sculpt** layout button between Standard and UV Edit (the Sculpt tab in front of Attributes, Edit layers beside Objects, the shelf of brushes under the view, saved between sessions), the brush bar floating over the view (brush, Size, Strength, Spacing, Falloff), the Sculpt tab (what is sculpted, the brush, the look and Cavity). A shape of 200 000 polygons or more is made ready on another thread. The question about booleans, the live mirror and modifiers (plan choice 7) is asked once per object: **Bake them in** (Apply mirror and Make editable, one undo step) or **Sculpt under them** (drawn without them while sculpting).
- **Controls** (the plan's): **1** Draw, **hold Shift** smooth, **hold Ctrl** dig in, **[ ]** size, **Ctrl + right drag** size (sideways) and strength (up and down), **Esc** cancels a stroke or leaves the mode, **Tab** stays. New choices for review: **2 to 9** do nothing in Sculpt mode yet (kept for the brushes to come, rather than switching modes), and a press off the sculpted shape says to pick another shape in the Objects panel.
- **Choice 2 changed:** sculpting is built now, straight after and beside scripting, before rigging and animation.

## Consequences

- A sculpt keeps where its shape's points were: about 12 bytes a point beyond the moves (36 MB for the heavy test shape), shared by every stroke and packed in the file. In return a primitive given more segments keeps its sculpting.
- Scripts see Sculpt mode as the Model mode until scripting knows it.
- The brush circle is drawn on screen, not on the surface (left for milestone 17a.4 with the reflected circle); size is measured on screen only; pen pressure arrives with 17a.3.
- Heavy numbers for this milestone are in its pull request; the heavy-mesh check gains a sculpting check (`heavy_sculpt_check`).
