# 0090. Stable element names

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0032](0032-edits-find-their-points-again.md) (finding points again), [0043](0043-attribute-layers-and-the-builder.md) (the builder), [0044](0044-edits-that-hold.md) (trusted ids); from the [whole-app review](../reviews/2026-10-13-whole-app-review.md) (A3)

## Problem

Every point and polygon is numbered in the order it was made. Change an earlier edit so it makes more or fewer elements (more bevel rows, say), and everything made after it is numbered differently. The later edits' recorded numbers then name the wrong elements. They fall back on finding their elements by place (0032), which works well, except where the place is ambiguous: two points on the same spot, such as a cap pushed out by nothing, or points welded back onto each other. There, "the nearest point" is a guess, and the wrong one moves. Phase 6's edit list will make changing earlier edits common. This is the "naming problem" of history-based modellers.

## Options

- **What a name says:**
  - **where the element came from, not when** (chosen): the edit that made it, and its source by the sources' own names: a copy of a point, a spot between two points, a spot inside a polygon (the builder's `PointFrom`, which every tool already declares, 0043). A polygon is named after its corners;
  - a counter per edit, which still shifts when an earlier edit changes.
- **The base shape:**
  - **a primitive's points named after where they sit in its box** (chosen), so a corner keeps its name when the cube gets more segments or another size; a plain mesh's points after their ids, which never change;
  - named after ids for primitives too, which more segments renumber.
- **How names are used when finding points again:**
  - **to tell apart points on the same spot** (chosen, for now);
  - **in place of the search by place**, whenever every name is found. This was built and tested. It changed what many existing kinds of edit chains produce: an edit that used to find nothing (and so did nothing) now acted on the named points. It would be right more often, but it changes projects people have already made. It waits for the edit list, where it can be tried on real changes of earlier edits and switched on with the project lead's say.
- **Saving:**
  - **names saved beside each edit** (chosen, file format 27), so a project behaves the same after saving and opening;
  - worked out again on opening, which can't be done: an edit's names come from the shape it was made on, gone by then.

## Decision

1. **`mesh::names`**:
   - Each mesh keeps names for its points and polygons by slot, shared between copies until one changes. They are not part of mesh equality.
   - `Mesh::name_base` names a base shape by place.
   - `begin_naming(salt)` / `end_naming` make an edit name what it makes:
     - the builder's `point` names each point after its `PointFrom`, counting repeats of the same origin;
     - `add_face` names each polygon after its corners (from the lowest name, so the first corner doesn't matter);
     - `add_vertex` gives a plain name.
   - `point_name`, `trusted_point_name` and `face_name` read the names. A mesh never named (a plain mesh) is named after its ids, and those names are written out before an edit adds names of its own.
   - Names are mixed with SplitMix64's finaliser: fixed, the same on every computer, because they are saved.
2. **The stack** names each edit's work after its `EditId`, and names a primitive's base shape when it is worked out.
3. **`Anchors::names`**: the anchored points' names, when every one has a trusted name.
   - Finding points again by place prefers, among points on the same spot, the one with the anchored name.
   - Anchors compare equal whatever names they carry.
4. **File format 27** stores, beside version 26's file, each edit's names (`NamesV27`: object, edit id, names). They are checked: one per anchored point, never 0, for edits and objects that are there, never twice. Older files open with no names and behave as before.

## Costs, measured (release build)

- **Bevelling every edge of a 32k-polygon sphere:** 0.93 s (target 5 s).
- **Dragging an extrude on the heavy shape:** 82 ms per frame (target 1 s), up from about 37 ms. Most of it is copying the names when the dragged edit renames its new points. It's within its target, and the first thing to make faster if dragging on heavy shapes feels slow.

## Consequences

- Edits on points that sit on the same spot as others now keep to the right ones when an earlier edit is changed.
- Every element made from now on carries a name. The Phase 6 edit list can lean on names more (finding by name first) once it is tried on real changes.
- Elements made by an earlier edit whose settings change (rows added to a bevel) can get new names themselves: no name says which of three rows is "the same" as which of two. Finding by place covers them, as before.
