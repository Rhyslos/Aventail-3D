# 0250. The baker: File → Bake…

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 25. The painter bakes guide maps from a shape itself (0192), but a game asset needs the classic bake: a high-detail model's bumps and bevels in a tangent-space normal map on the low-detail model, as Godot reads it, plus occlusion, curvature, thickness, colour ID, position and height, as files.

## Decision

- **Where:** **File → Bake…** (the project lead chose it, 2026-11-05), a window: **Onto** (the first selected object with a shape), **Bake from** (*The object itself* or any other object with a shape: the high-detail one, placed over the low one), the maps to bake, the **cage** push (as a share of the object's size, 2% standard), **Normal maps green down (DirectX)**, and **Bake into a folder…**, which asks for a folder. No keys.
- **In the existing `bake` crate** (`bake::maps`), not a new crate as the plan said: it already holds the rays, the covered texels and the painter's maps, and stays in the core layer (no UI or GPU).
- **Normal and height from the high shape:** for each covered texel, a ray from the cage (the low shape pushed out along its shading normal) back in through it, up to twice the push; the nearest hit gives the high shape's smooth facing (`bake::maps::High`: its triangles in a box tree, `Rays::of_triangles` and the new `Rays::nearest`) and its height. The normal is written in the texel's frame of the low shape's shading normal and its **MikkTSpace** tangent (`bevy_mikktspace`, zlib licence, already used by the exporter, 0073), the bitangent made per texel from the interpolated tangent as MikkTSpace asks, so Godot, which works out the same tangents on import, lights it the same. Green up (OpenGL, Godot, Blender) as standard. A ray that meets nothing leaves the flat normal and the middle height.
- **The other maps from the low shape**, through the painter's bakes (`bake::bake_watched`). Occlusion from the high shape can follow with the high-to-low milestone.
- **Files** (`fileio::bake_file::bake_png`): `<object>_<map>.png` at the project's texture size, the pieces padded outward by Pack's padding (the exporter's `pad`) and the rest a neutral colour (flat blue for normals, white for occlusion, grey for curvature and height, black otherwise), opaque.
- **On another thread**, with "Baking map 2 of 4… 35%" and **Cancel**; nothing in the scene changes, so there's nothing to undo.
- Both objects are baked in the world (each through its placement), so a high-detail object placed over the low one bakes where it is.

## Consequences

- One low object a bake; pairing many by name (_low / _high), exploding and the cage on the model are milestone 26.
- Missed rays aren't reported yet (milestone 27).
