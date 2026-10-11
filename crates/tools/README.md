# tools

**Layer 3 · Front end**

Interactive tools and the camera: navigation, selection, transform gizmos, and the modelling and UV tools.

## Purpose

Turns plain input (mouse, keys, what is under the cursor) into intentions: the viewport camera with a choice of navigation controls and cursor-point orbit and zoom, the selection methods, the move/rotate/scale gizmos, and every modelling tool (extrude, bevel, loop cut and so on) with live parameters. Each tool's result is an operation plus a command for `scene` to apply. See DESIGN.md §4.4 and §6.

## Must never

- Change the scene or a mesh directly: tools only produce operations and commands
- Depend on egui or the GPU: input arrives as plain data, so tools are testable without a window

## Depends on

`mesh` (bounding boxes; `scene`, `uv`, `config` planned). Outside libraries: `glam` (maths), `serde` (saving the camera).

## Status

- **`Camera`**: position, yaw, pitch and field of view; view and projection matrices (right-handed, Z up, depth 0 to 1); `project` (world to screen) and `ray_through` (screen to world). Opens looking at the origin from the front, right and above. With `ortho_height` set it is a flat (orthographic) camera showing that many centimetres from bottom to top; cameras saved before this load as perspective.
- **`ViewKind`**: Perspective, Top, Front and Right, with `name`, `is_flat` and `grid_plane` (the plane a view's grid lies in). `Navigator::for_view` opens a view of a kind and `Navigator::kind` tells which it is. Flat views never orbit; panning uses the shown height, and zooming changes that height, keeping the point under the mouse still.

- **`Navigator`** (`navigation`): the camera plus the pivot it looks at. Orbit around the pivot wherever it is on screen (it stays still while the view turns; the scene follows the mouse; never flips over the top), `set_pivot` (the point under the mouse becomes the pivot, without moving the camera), pan (points at the pivot's depth stay under the mouse), dolly (exponential, 2 cm to 1 km), and `dolly_towards`/`wheel_towards` (zoom towards a point, which stays under the mouse and becomes the pivot); `framed` (the same view moved so a `Bounds` fits, keeping its direction) and `blend` (part of the way between two views, for gliding); Plain input in, no egui. - **Navigation controls** (`controls`): five `Scheme`s (Alt + mouse, Mouse only, Mouse only middle zooms, Middle pans Alt orbits, Middle orbits) named by what they do, and `NavigationSettings` (scheme, Alt + mouse in every scheme, invert orbit, orbit and zoom around the mouse) that turn a button and keys into a `DragKind`. A plain left drag never navigates. Saved with serde.
- **Elements under the mouse** (`components`, [decision 0027](../../docs/decisions/0027-component-modes.md)): `PickScene` holds the shown objects' faces as world-space triangles plus the points and edges of the edited ones; `under(ComponentKind, camera, size, mouse)` returns a `Hovered` (object index and `Component`: point, edge or polygon). Points within `POINT_RADIUS` and edges within `EDGE_RADIUS` screen points, nearest first; polygons by the first face the mouse ray hits. Anything hidden behind a surface is skipped.
