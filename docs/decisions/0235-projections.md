# 0235. Projections and world-aligned UVs

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 7. UVs could only be made by unwrapping along seams, by the shapes' ready-made UVs, or by hand. Flat signs, boxy kit parts, pipes and balls are quicker to texture with a projection aimed at them, and modular kit pieces need a tiling texture that lines up across their joins wherever they stand.

## Decision

- **`uv::project`**: `project(mesh, faces, ProjectOptions { kind, placement, world })`. `placement` takes the shape's own space to the projection's, where the projection shape is the box from −1 to 1. **Planar** lays X and Y; **Box** each polygon by the side its (Newell) normal faces most, turned so sides read upright and nothing is mirrored; **Cylinder** the angle round Z (0 at −Y, once round = 1) and the height, each polygon's corners kept within half a turn of each other and moved a turn on if below 0; **Sphere** the angle round and the angle up (0 bottom, 1 top), pole corners taking their polygon's middle angle. Without `world` the box from −1 to 1 becomes the square 0 to 1 (the cylinder's angle is already 0 to 1); with `world`, the coordinates are multiplied by it (texture widths per unit), not fitted. Box, cylinder and ball mark seams on edges where the projected polygons' UVs part. Locked pieces are skipped; broken placements (not invertible, not numbers) change nothing.
- **`Operation::ProjectUvs { faces, options, anchors }`**, recorded with its placement so it replays the same (decision 0110). **File format 86** stores it (`v86::ProjectEditV86`: the kind as a number, the 16 placement numbers, the world scale).
- **Placing** (`workbench::projecting`): the project lead chose UV → Project and the E ring (2026-11-05). UV → Project offers Flat, Box, Cylinder, Ball, Flat from the view and World-aligned. The E ring's Unwrap place becomes the group **Unwrap & project**: Unwrap, Unfold across, Unfold up and Project (flat from the view; the other shapes are in the menu, since a group holds at most five). Starting a projection places it round the selection in the world (Flat along the selection's thinnest axis, the others upright along Z; uniform size so the texture isn't stretched one way, 2% bigger), switches to the Move gizmo if no gizmo tool is in use, and records it. While placing, the viewport's gizmo sits on the projection: its drags change the projection's place, turn or size from where the drag began, and each change takes back the last record (`Document::abandon`) and records again under the same undo key, so the UVs follow live and the whole placing is one undo step. The shape is drawn with the tool lines. **Enter** or **Done** keeps it; **Esc** or **Cancel** takes it back (the controls chosen). An undo from outside ends the placing.
- **World-aligned**: a box projection from each object's shape place in the world (`Scene::shape_matrix`) at the project's texel density (`TextureSettings::uv_per_unit`), recorded at once.

## Consequences

- A projection doesn't follow the object if it moves later: world-aligned UVs are worked out where the pieces stand (the manual says to world-align after placing).
- Projections don't snap or show their numbers yet; milestone 9 (precise placement) may add both.
- The E ring's Unwrap now takes one more click; Unwrap is also in UV → Lay out.
