# 0313. Speed: 3 million polygons stay smooth (Phase 17a, milestone 6)

- **Status:** Accepted, built while the project lead was away; the new choices below wait for review in [docs/plans/phase-17.md](../plans/phase-17.md#choices-to-review) (65 to 68)
- **Date:** 2026-11-18
- **Follows:** [0300](0300-sculpting-plan.md) (the sculpting plan and its performance targets), [0304](0304-sculpt-mode-and-layers.md), [0306](0306-sculpt-brushes-for-forms.md), [0310](0310-sculpt-symmetry-and-masks.md), [0312](0312-remesh-for-sculpting.md), [0033](0033-heavy-mesh-safeguard.md) (the heavy-mesh check), [0035](0035-heavy-meshes.md) (heavy shapes)

## Problem

The last milestone of Phase 17a: measure every brush, the mask and remeshing on the 3-million-polygon heavy test shape against the plan's targets, fix what misses, give the heavy-mesh check its sculpting steps, and count each stroke's memory in the undo budget. The earlier milestones left: the first frame of a big stroke slow; undo and redo of a big stroke slow (about 100 ms); moving points and refitting the box tree on one core; symmetric strokes and Clay strips missing 30 frames a second; Elastic grab about 180 ms a dab; remeshing the heavy shape to a million polygons in about 6 s (target 5 s); and the mask redrawn by comparing every point.

Measuring showed three more costs that the earlier checks didn't see: the brush bar's polygon estimate worked out the whole shape's area on another thread after **every** stroke (one per stroke, piling up, each taking a core from the next stroke); the drawing, after every dab, checked whether any edge round the moved points turned sharp or soft (about half a second for a big dab on two cores) and started rebuilding the whole shape's drawing in the background whenever one did; and following a big dab's moved points in the drawing took about 60 ms on one core.

## Options

1. **Where the work runs.** Brushes on the graphics card (fastest for huge brushes, but much harder to keep exact, testable and undoable: the plan's choice 23 ruled it out); or every step of a dab on the processor's cores, in fixed blocks so the result is the same on any computer.
2. **Undo of a big stroke.** Work the stroke's points out again from the sculpt (as before: frames for every point); or keep where the stroke's points were and where it left them with its undo step (about 28 bytes a point moved, counted in the undo budget), so undo and redo just put them back.
3. **The drawing's shading by angle while sculpting a heavy shape.** Check every frame (as before); check once the shape has stayed still a moment; or draw sculpts smooth while sculpting.
4. **Elastic grab with a big brush.** Shrink its reach (choice 34: four brush sizes); or leave its reach and dab once a frame, straight to the mouse, when a frame's dabs run out of time.

## Decision

- **Every step of a dab on every core, in fixed blocks** (option 1, the second way; `sculpt::parts`): finding the points in reach (each polygon's corners checked on every core, then each point taken once in the order found, so the result doesn't change), the plane and facing under the brush (added up block by block, the blocks in order, so the same to the last bit on any number of cores), each point's new place, which points move, and **putting them in place at once** (`Mesh::set_positions`: each chunk of points copied once, not once a point, on every core for many). The box tree is refitted round the polygons the dab found (no looking each point's polygons up again), and **all at once on every core** when half the shape or more moved (`FaceTree::refit_faces`, `refit_all`). Finishing a stroke works each moved point's move out on every core.
- **A stroke keeps its places** (option 2, the second way): `Sculpt` holds, beside its moves, where the stroke that made it found its points and where it left them, with which sculpt it started from and on which shape (never saved, not part of what makes two sculpts equal). Undo and redo of a stroke made in this session put the points back from them, and taking a stroke back (Esc, a Line or stamp laid again) puts back where its points were. The bytes count in the undo step's memory (`Sculpt::memory_size` and `memory_apart`).
- **Shading checked once the shape stays still** (option 3, the second way): a heavy shape's drawing follows each frame's moved points with the shading it has; once its points have stayed still for 0.3 s, whether any edge turned sharp or soft is checked once (`mesh::moving_changes_hardness`: only the polygons with a moved corner, each polygon's facing worked out once, on every core), and the drawing is rebuilt in the background only then. Following moved points (`MeshGeometry::moved`) and shading them work on every core; ranges of vertices to upload come from a list of bits instead of a sort; a mask changed over many points is shaded in one pass.
- **Elastic grab keeps its reach and dabs once a frame when slow** (option 4, the second way): any grabbing brush whose last frame's dabs ran out of time goes straight to the mouse with one dab.
- **The polygon estimate waits** (a fix): a heavy shape's area is worked out again only once its shape has stayed the same for a second and no stroke is under way, one job per object; the last area stands in meanwhile.
- **Remesh for sculpting, quicker:** cutting the shape into triangles, finding open edges (sorted on every core, `mesh::parallel::sort`), the shape's area (`mesh::parallel::sum_in_blocks`) and sorting triangles into the grid's blocks run on every core; the distance from each grid point skips a triangle whose surrounding ball is farther than the nearest found so far.
- **The heavy-mesh check's sculpting steps** (`tools/heavy/heavy-check.sh`): every brush with the big brush (Elastic grab with the whole shape in reach, timed but not a target), undo and redo of a big stroke and the memory it keeps, the mask's actions and their undo, remeshing the heavy shape to about 1 and 3 million polygons and a 500 000-polygon sculpt to 1 million; `sculpt`'s `speed_benchmark` (each brush's dab, finishing, undo and redo); the drawing's update after a big dab, its shading check and a mask's shading (`render`'s heavy geometry check).
- **No new file format, no new keys.**

## Consequences

- See the heavy numbers in the plan's "What milestone 6 built" and the pull request: before and after, target by target, and which still miss and why.
- A stroke's undo step keeps about 28 bytes more a point it moved (a big stroke of 130 000 points: about 3.6 MB more), counted in the undo budget, so a long session forgets its oldest steps a little sooner.
- On a heavy shape, an edge a stroke turns sharp shows smooth until the stroke stops for a moment (then the drawing is rebuilt in the background, as before, but once instead of again and again).
- Work in fixed blocks gives the same result on any number of cores; some sums are added in a different order than before, so sculpted results differ from the earlier milestones' in the last bits (undo, saving and evaluating still agree exactly).
