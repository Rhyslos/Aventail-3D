# 0242. Lightmap UVs

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 14. Godot's baked lighting needs a second UV layout with every piece unique (nothing stacked or mirrored), at real relative sizes, with room round each piece. Making one by hand means a second set, re-cutting, unwrapping and packing.

## Decision

- **`uv::lightmap::lightmap(mesh, hard, size, padding)`** works on a copy of the shape: locks and pins lifted (the lightmap is its own), the seams and the given hard edges cut, everything unwrapped with the size-keeping method (strips straightened), then packed into the square turning allowed, never stacking, `padding` pixels apart at `size`. It returns one UV per corner; the shape's own UVs are untouched.
- **`Operation::MakeLightmap { size, padding, hardness }`** writes that into the `lightmap` UV set (`mesh::uv_sets::write`, made if needed), with the hard edges worked out from the recorded smoothing as the edit replays. **File format 90** (`v90::LightmapEditV90`; the angle as `None` for marked hard edges).
- **UV → Lightmap UVs** (the project lead's choice: in the UV menu): the size (128 to 2048, 512 standard), the room round pieces (1 to 64 px, 4 standard), and **Make lightmap UVs** for every selected object, one undo step. Remembered with the UV settings.
- Export: `uv_sets::second` prefers the set called `lightmap`, so it is glTF's `TEXCOORD_1` (Godot's UV2).

## Consequences

- The lightmap is laid out again whenever the edit replays (a change to the shape before it); after editing later, the user makes it again (the manual says so).

## Amended 2026-11-06, objects with eight sets

An object with eight UV sets, none called `lightmap`, has no room for the lightmap: the edit made nothing yet was recorded, and the note said it made one. Such objects (`uv_sets::has_room_for`) are now left out and named in the note; with only those selected, nothing is recorded. Auto-unwrap's Lightmap UVs step does the same and says so in its window.
