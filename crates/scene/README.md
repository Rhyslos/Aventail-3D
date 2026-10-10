# scene

**Layer 1 · Core**

The scene document: objects, their operation stacks, commands and undo history.

## Purpose

The single source of truth for an open project. Holds the object hierarchy (transforms, names, visibility), each object's base (a parametric primitive or a mesh) and its stack of recorded operations, evaluated and cached layer by layer. Every change is a command that can undo itself; history is bounded by a memory budget, not a step count. See DESIGN.md §4.2–4.3 and [decision 0006](../../docs/decisions/0006-base-plus-operation-stack.md).

## Must never

- Know about files, the GPU or UI
- Be changed except through commands
- Store world-space transforms: only local ones, with world transforms derived
- Assume the scene is frozen in time: evaluation takes a context, ready for animation later

## Depends on

`mesh`, `uv` (planned).

## Status

Empty. Objects, primitives and undo arrive in Phase 2.
