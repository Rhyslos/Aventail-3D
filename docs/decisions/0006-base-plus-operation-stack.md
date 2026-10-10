# 0006. Objects are a base plus an operation stack

- **Status:** Accepted
- **Date:** 2026-10-10

## Problem

Two of the project lead's top wishes are the same idea underneath: 3ds Max's stack of Edit Poly modifiers ("by far the most important tool"), and being able to change an object's base properties (such as a cube's segments) no matter how much it has been edited since. Adding this to a modeller that edits meshes destructively is close to a rewrite, so it must be decided before the first modelling tool exists.

## Options

1. **Destructive editing:** every tool changes the mesh directly. Simplest, but base properties are lost after the first edit, and a layered stack is impossible later.
2. **Base plus operation stack:** an object keeps its base (a parametric primitive or a mesh) and a list of layers, each holding recorded operations. The final mesh is evaluated from the base upward, with each layer's result cached.

## Decision

Option 2. Every tool produces an operation (and a command for undo) instead of changing a mesh. Until the Edit Layers interface arrives (Phase 7), every edit goes into one hidden layer, so the app behaves like Cinema 4D while the structure is already in place.

Operations refer to mesh elements by stable IDs, created deterministically, so replaying an unchanged history gives identical results. When an earlier change makes a reference invalid, the operation applies to whatever still matches and the layer shows a warning badge; nothing is silently destroyed. "Collapse" bakes a stack into a plain editable mesh at any time.

## Consequences

- Base properties stay editable after modelling (wish W4), and the Edit Layers stack (wish W5) only needs an interface later.
- Undo history and the stack are separate: undo is about time, the stack is about structure.
- Every tool is a little more work to write, since it must record an operation.
- Re-evaluating long stacks must stay fast; caching per layer and re-evaluating only from the changed layer upward handles this.
