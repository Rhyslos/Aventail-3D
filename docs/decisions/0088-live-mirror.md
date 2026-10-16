# 0088. The live mirror replaces Symmetry's mirrored edits

- **Status:** Accepted
- **Date:** 2026-10-16
- **Replaces:** [0054](0054-symmetry.md) and [0059](0059-symmetry-for-every-edit.md) for new work (old projects keep their mirrored edits)
- **Brings forward:** Phase 7's mirror generator

## Problem

Symmetry made every edit twice, once on each side of the shape. That worked for edits but not for moving the object: with symmetry across X, the project lead moved a cube to the side and expected its mirror to move the other way, like a live instance. It was a point of friction while modelling. He asked for the mirror to work like a live duplicator, and chose:

- the mirror stands at the **world centre**, and can be moved;
- the mirrored half is **part of the same object**;
- it **replaces** today's Symmetry;
- it is shown as a **faint red see-through plane** with a clearer line where it meets the ground.

## Options

- **What the other half is:**
  - **a copy worked out from the object's shape every time it is drawn or exported, welded at the seam** (chosen): edits are made once, on the object's own half, and the copy follows everything, including moves;
  - every edit made twice, as before, which can't follow moves of the whole object.
- **Where the mirror lives:**
  - **on each object, saved with the project** (chosen): an object's twin is part of it, and a project can mix mirrored and plain objects;
  - one switch for the whole app, as before, which would mirror everything or nothing.
- **Its planes:**
  - **the world's X, Y and Z planes through a point of the world** (chosen): moving or turning the object moves or turns its twin the mirrored way;
  - the object's own planes, which move with it and so never show a twin moving the other way.
- **The seam:**
  - **points of the object and its copy within a weld distance of each other near the plane become one, live** (chosen, 0.1 cm as standard): the halves join into one surface, and a point moved off the plane leaves it again;
  - welding once by hand (Symmetry's old Weld the middle now), which goes stale after the next edit.
- **Editing the twin:**
  - **not editable or clickable** (chosen): tools, selection and picking use the object's own shape, whose ids the edits record;
  - clicking the twin to edit the matching part of the real half, which can come later.

## Decision

1. **`mesh::mirror`**:
   - `with_copies(mesh, copies, on_seam, distance)` returns the shape with a copy placed by each mirroring matrix. The shape keeps its own points and polygons and their ids; the copies come after. Points `on_seam` picks are welded within the distance (`join::weld_close`).
   - `reflection` and `combinations` give the matrices for the chosen axes.
   - `parts::append` adds one placed copy into a shape; `combine` now uses it.
2. **`scene`**:
   - `Object::mirror: Option<LiveMirror { axes, centre, weld_distance }>` (`is_valid`, `across_x`).
   - `Command::SetMirror { ids, mirror }` ("Mirror", "Mirror off"); groups are left alone, and broken values are refused.
   - `Scene::mirror_copies` gives the copies' matrices in the shape's own space: the world reflection seen from the shape.
   - `Scene::with_mirror` / `shown_shape` give the shape with them. The weld is measured in the world.
   - `bounds_of` includes the twins, so framing and the scene's box see them.
3. **Drawing, exporting and counting use the shown shape:**
   - `DrawItem::drawn` is uploaded to the graphics card;
   - `fileio::export` exports it, and `summary` counts it.
   - Picking, selecting, the tools and the bake check keep the object's own shape (`shown`, `evaluated`).
4. **The interface:**
   - The Symmetry button gives the selected objects (and the shapes inside selected groups) a mirror with the menu's settings, or takes theirs away. It is lit when one has a mirror.
   - The Symmetry menu holds On, the axes, where the mirror stands (Move the mirror to X, Y, Z) and the weld distance. It changes the selected objects' mirrors as one undo step, and keeps the settings for the next mirror.
   - The tools no longer record `Operation::Mirrored`.
   - Removed: Weld the middle now, Mirror through (pivot, middle, world), and Don't cross the middle.
5. **The viewport** draws each mirror plane once (`SceneDrawing::mirrors`, `MirrorPlane`) as a faint red sheet around the objects using it, with a red line where an upright mirror meets the ground.
6. **File format 25** stores each object's mirror beside version 24's file. Older files open with none.

## Consequences

- A half is modelled once; the whole shows and exports. Moving an object moves its twin the other way.
- Mirroring a whole shape doubles it on top of itself. The manual says to delete the other half first. A one-click "cut the other half away" could come later.
- `Operation::Mirrored` and `WeldMiddle` stay in the scene for old projects, which open and change as before. Nothing new makes them.
- Picking the twin, mirroring a single edit, and keeping points from crossing the middle are left for later, if they are missed.
- The plane is drawn over the scene (not hidden behind shapes), kept faint so it doesn't hide them.
