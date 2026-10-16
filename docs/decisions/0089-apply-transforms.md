# 0089. Apply transforms: an object's placement made part of its shape

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0068](0068-pivot-editing-and-scene-units.md) (pivots), [0082](0082-edits-keep-their-shape-on-stretched-objects.md) (edits on stretched objects)

## Problem

Objects get moved, turned and scaled all the time. A scaled or turned object is harder to model on: edits have to make up for the stretch, the Coordinates panel mixes the shape's size with the scale, and game engines receive objects carrying a scale. The project lead asked for this first in Phase 6, as Blender's Apply: "nullify" the prop's rotation and scale so it can be modified without issues.

## Options

- **How the shape takes the change:**
  - **a recorded edit that places the whole shape by a matrix** (chosen). Tools never change shapes directly (0006), so the change is undoable like any edit, primitives keep their settings, and edits before it keep their meaning;
  - baking the object into a plain mesh, which loses a primitive's settings and the edit list.
- **Which parts:**
  - **rotation and scale together as the usual choice, and position, rotation, scale or all three as well** (chosen, as Blender);
  - only all three, which would also move every pivot to the world centre.
- **Objects inside:**
  - **placed again so they don't move** (chosen, as moving a pivot already does);
  - left with their numbers, which would make them jump.
- **Instances:**
  - **left alone** (chosen): their shape is their original's;
  - turned into copies first, which would surprise.
- **Keys:** the project lead was asleep, so the recommended choice was taken: **no key of its own** for now (Ctrl + A, Blender's, selects everything here). The command is in Mesh → Apply transforms, the Objects panel's Object menu, and the last place of the Model mode's tool menu (Apply rotation and scale). A key can be added when he picks one.

## Decision

1. **`mesh::parts::place(mesh, matrix)`** (re-exported from `edit`) moves every point by the matrix, keeping ids. A mirroring matrix also turns every polygon the right way round (`cleanup::flip_faces`). A flattening or broken matrix changes nothing.
2. **`Operation::PlaceShape { matrix }`** ("Apply the placement") records it. Its numbers must be real and must not flatten the shape.
3. **`Command::ApplyTransforms { ids, parts: ApplyParts }`** applies the objects, parents first. For each object:
   - the new placement keeps what isn't applied (`ApplyParts::kept`);
   - the shape is placed by `new⁻¹ · old · offset`, and the offset goes back to none;
   - its children are placed by `new⁻¹ · old · child`;
   - all of it is one undo step, named "Apply rotation and scale" and so on (`ApplyParts::name`).
   - Nothing to apply records nothing. A flattened object (scale 0) is refused.
4. **The interface:**
   - `CommandId::ApplyTransforms` (Apply rotation and scale) with `Icon::ApplyTransforms`, in the Model mode's tool menu (its 15th place);
   - an **Apply transforms** submenu (`objects::apply_entries`, `APPLY_CHOICES`) in the Mesh menu and the Objects panel's Object menu.
5. **File format 26** stores the edit (`OperationV26::PlaceShape`, its 16 numbers) beside everything version 25 stores. `ProjectV25` now takes any kind of entry.

## Consequences

- An object can be reset in one step, keeping its look. Later edits measure as they look, and exports carry no rotation or scale.
- A primitive's Width, Height and other settings still change it, measured before the applied change. Make editable bakes it all.
- Applying a turn to an object scaled unevenly leans its shape (the shape takes the skew). That is exactly what it looked like.
