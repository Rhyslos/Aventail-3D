# 0021. The object stack foundation: how edits are stored, evaluated and baked

- **Status:** Accepted
- **Date:** 2026-10-11

## Problem

[Decision 0006](0006-base-plus-operation-stack.md) settled that an object is a base plus a stack of recorded operations. Before the first modelling tool (Phase 3) records anything, the shape of that stack has to be built: where the stack lives, how edits are recorded and undone, how the final shape is worked out and cached, what happens when an edit no longer fits, and how an object becomes a plain mesh.

## Options and decisions

**Where the base and stack live.** The object's *kind* is its base: a primitive's settings, or a plain mesh (`ObjectKind::Mesh`, shared behind an `Arc` because meshes can be large and are never changed in place). The stack is a separate field on every object (`Object::stack`), always empty for groups. The alternative, a new "shaped object" kind wrapping base and stack, would have changed every place that looks at a primitive's settings for no gain.

**Recording.** `Command::Record { id, operation }` adds an operation to the top layer, making an `Edits` layer first if there is none. Until the Edit Layers panel (Phase 7) there is only ever that one layer, so the user sees a plain object. Each recording is its own undo step, made of small edits (`InsertLayer`, `InsertOperation` and their opposites), so undo stores only what changed, never a copy of the stack. Groups refuse edits (`SceneError::NoShape`).

**The first operation.** `Operation::MovePoints { points, offset }`, backed by the pure function `mesh::edit::move_points`. It is the simplest edit the Phase 3 tools will need, and enough to test recording, replay, warnings and baking end to end. More operations arrive with the tools that make them. No tool in the app records operations yet, so the stack is invisible in normal use until Phase 3.

**Evaluation and the cache.** The scene works out a shape on demand (`Scene::evaluated`): the base's mesh, then each switched-on layer's operations in order. Each layer's result is cached. On the next request the cache compares the base and each layer with what it last evaluated, and starts again only from the first layer that differs (or from the base, if that changed). The cache sits inside the scene behind a lock, so the shape can be worked out while the scene is only read; it is not part of what the scene *is* (a copy starts with an empty cache, and it never affects equality). Evaluation takes an empty context, ready for a point in time when animation arrives (DESIGN.md §4.5). Comparing layers costs a little per request; hashing would be faster but floats don't hash cleanly, and stacks are short.

**When edits no longer fit.** Operations name points by id. If an id is gone (a cube was given fewer segments), the operation applies to the ids that remain and the result carries a `StackWarning` (layer, operation, how many were missing). Nothing is deleted, and going back to the earlier settings brings the points back. The Attributes panel says how many edits are recorded and flags any that no longer fully fit.

**Make editable.** `Command::MakeEditable` bakes each shape's base and edits into a plain mesh, which becomes its new base, and empties its stack, as one undo step. Groups, and meshes with nothing recorded, are skipped. It is in the Mesh menu, the Object Manager's Object and right-click menus, and on `C`: the key most 3D programs use for this, and unused here ([decision 0019](0019-only-the-tool-keys-come-from-halberd.md) leaves non-tool keys to us). Pressing `C` with only groups selected explains why nothing happened. *Changed 2026-10-11 at the project lead's request: the key is `Ctrl + E`, so plain letters stay free for tools.*

## Consequences

- Phase 3 tools only need to produce operations; recording, undo, replay after a settings change, warnings and baking already work and are tested.
- Point ids are only stable while the base's topology is: changing a cube's segments renumbers its points, so edits recorded before can land on different points. This is the predictable compromise DESIGN.md §4.2 accepts; the warning makes it visible when points disappear, but not when they are renumbered. Better matching can come later without changing the stack. *(Done in [0032](0032-edits-find-their-points-again.md): edits also record where their elements were and find them again by position.)*
- Evaluated shapes, not bases, decide what is drawn and framed, so framing and the viewport see recorded edits.
- Saving (`.avtl`, a later Phase 2 milestone) must store each object's base and stack.
