# tools

**Layer 3 · Front end**

Interactive tools and the camera: navigation, selection, transform gizmos, and the modelling and UV tools.

## Purpose

Turns plain input (mouse, keys, what is under the cursor) into intentions: the viewport camera with a choice of navigation controls and cursor-point orbit and zoom, the selection methods, the move/rotate/scale gizmos, and every modelling tool (extrude, bevel, loop cut and so on) with live parameters. Each tool's result is an operation plus a command for `scene` to apply. See DESIGN.md §4.4 and §6.

## Must never

- Change the scene or a mesh directly: tools only produce operations and commands
- Depend on egui or the GPU: input arrives as plain data, so tools are testable without a window

## Depends on

No other app crates yet (`scene`, `mesh`, `uv`, `config` planned). Outside libraries: `glam` (maths), `serde` (saving the camera).

## Status

- **`Camera`**: position, yaw, pitch and field of view; view and projection matrices (right-handed, Z up, depth 0 to 1); `project` (world to screen) and `ray_through` (screen to world). Opens looking at the origin from the front, right and above. 
- **`Navigator`** (`navigation`): the camera plus the pivot it looks at. Orbit (the scene follows the mouse; never flips over the top), pan (points at the pivot's depth stay under the mouse), dolly (exponential, 2 cm to 1 km from the pivot) and wheel zoom; Plain input in, no egui. - **Navigation controls** (`controls`): five `Scheme`s (Alt + mouse, Mouse only, Mouse only middle zooms, Middle pans Alt orbits, Middle orbits) named by what they do, and `NavigationSettings` (scheme, Alt + mouse in every scheme, invert orbit) that turn a button and keys into a `DragKind`. A plain left drag never navigates. Saved with serde. Orbiting around the point under the cursor is the next milestone.
