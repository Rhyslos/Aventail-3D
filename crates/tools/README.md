# tools

**Layer 3 · Front end**

Interactive tools and the camera: navigation, selection, transform gizmos, and the modelling and UV tools.

## Purpose

Turns plain input (mouse, keys, what is under the cursor) into intentions: the C4D-style camera with cursor-point orbit and zoom, the selection methods, the move/rotate/scale gizmos, and every modelling tool (extrude, bevel, loop cut and so on) with live parameters. Each tool's result is an operation plus a command for `scene` to apply. See DESIGN.md §4.4 and §6.

## Must never

- Change the scene or a mesh directly: tools only produce operations and commands
- Depend on egui or the GPU: input arrives as plain data, so tools are testable without a window

## Depends on

`scene`, `mesh`, `uv`, `config` (planned).

## Status

Empty. Navigation arrives in Phase 1; selection and gizmos in Phase 3.
