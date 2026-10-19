# 0114. Layout snapshots

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0108](0108-uv-powerhouse-and-baking.md) (the Phase 8 plan), [0110](0110-controllable-automation.md) (edits replay the same), [0113](0113-uv-menu-in-groups.md) (the UV menu in groups)

## Problem

Phase 8's milestone 18: named versions of a UV layout to switch between, compared before and after. The project lead asked for it as the first of six milestones in least-work order, and had already chosen its controls: in the UV Editor's menus, no keys.

## Options

- **What a snapshot holds:**
  1. **Every polygon's corner UVs and the seams, by polygon id (chosen).** Small, plain data; putting it back needs nothing but the shape.
  2. A copy of the whole shape. Restoring would undo modelling too, which a UV tool must never do.
  3. A position in the undo history. Lost when the history is trimmed or the project closed.
- **How putting one back is recorded:**
  1. **A recorded edit on each object, `RestoreUvs`, holding the layout (chosen).** It replays like any UV edit (decision 0110), so changing an earlier modelling edit later still gives the same result, and undo is the usual one step.
  2. Writing the UVs straight onto the shape. Not allowed: tools never change shapes directly (decision 0006).
- **Polygons the snapshot doesn't know** (made since, or with a different number of corners): **left as they are (chosen)**, and the seams next to them too. Only seams between polygons put back are set to the snapshot's.
- **Comparing:** **its outlines drawn faintly under the current UVs, in a teal no other mark uses (chosen)**, with a note at the top left. Side-by-side panels would halve the editor.

## Decision

1. `mesh::uvs::UvLayout { faces, seams }` (`of`, `is_real`, `polygon_count`, `memory_size`) and `restore_layout`, which sets the UVs of known polygons with the same number of corners and the seams between them, and reports the rest as missing or skipped.
2. `scene`:
   - `snapshot::UvSnapshot { name, objects }`, each object's layout shared (`Arc`) so history copies cost little; `take` (objects with their own shape and UVs), `restore`, `layout`;
   - `MAX_SNAPSHOTS` 32 and `MAX_SNAPSHOT_NAME` 64 bytes; `snapshots_are_valid` (names not blank, one line, all different; UVs real) and `free_name`;
   - `Scene::snapshots`, `Command::SetSnapshots` (an undoable step, refused when not valid);
   - `Operation::RestoreUvs { layout }` (not mirrored; checked by `numbers_are_real`).
3. File format 38: `ProjectV38` (version 35's model and the snapshots) and `OperationV38::RestoreUvs`.
4. `ui`: the UV Editor's **Snapshots** menu (`uv_editor::snapshots`): Take a snapshot (the objects shown), then each snapshot with its name to edit, Restore, Compare, Update and ×. The menu stays open until a click outside it, so its notes can be read. Restoring several objects is one undo step. Compare is remembered by name (`theme::UV_SNAPSHOT`).

## Consequences

- No keys.
- Snapshots make project files bigger in proportion to the polygons they hold, so they're limited to 32.
- Restoring after modelling gives old polygons their old UVs and leaves new ones alone; the manual explains how to follow up.
- Later milestones (several UV sets, milestone 13) can store a snapshot per UV set by adding the set to `UvLayout`.
