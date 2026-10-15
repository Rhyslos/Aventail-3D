# 0078. Kit export and Godot collision shapes

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** [0072](0072-gltf-export.md) (glTF), [0075](0075-export-presets.md) (presets) and [0076](0076-engine-export-paths.md) (game project folders)

## Problem

A modular kit (walls, floors, doors, props) is many small pieces that a game engine places one by one. Each wants:

- its own file;
- its pivot at the file's origin, so it snaps where it is placed;
- in Godot, a collision shape, so the player bumps into it.

Exporting the whole scene into one file gives none of that, and making collision shapes by hand in Godot for every piece is slow (DESIGN.md §8, wish W16).

## Options

- **How pieces are chosen:**
  - every top-level object;
  - **the selected objects** (chosen), each with the objects inside it, so a piece can be a group.
- **Where a piece sits in its file:**
  - where it is in the scene;
  - **its pivot at the origin**, turned and sized as in the scene (chosen): only the place is taken away, so a piece modelled at an angle keeps it.
- **Collision in Godot:** Godot makes collision from a name's ending when it imports a glTF file. So the app can:
  - leave the naming to the modeller;
  - **give each object a Collision setting that the export turns into the endings** (chosen):
    - *The shape itself* → `-col` (exact, for parts that don't move);
    - *Wrapped (convex)* → `-convcol` (dents filled in: cheaper, and it can move);
    - *A box* → a box around the shape beside it, called `name-convcolonly` (the cheapest).
  - A shape of your own still works by naming it `…-colonly`, since names go out as they are.
- **Which exports carry the endings:** **only those made for Godot** (chosen). Other engines would show the endings as part of the name.

## Decision

1. **Scene:**
   - `Collision` (`None`, `Mesh`, `Convex`, `Box`) on every object, `None` as standard;
   - `Command::SetCollision` sets it on the objects with a shape, as an undo step.
2. **Attributes panel:** a **Collision** row under Visible for shapes, with what each choice is for on hover.
3. **Export:**
   - `ExportOptions::godot_names`, set by the Godot preset;
   - glTF names nodes with the endings, and writes a box node for *A box*.
4. **Kit:**
   - **File → Export kit…** exports each selected object (not one inside another selected one) as its own file, named after it. Characters a file name can't hold become `_`, and same names get a number.
   - Each piece's pivot is at the file's origin. The settings are the Export window's.
   - Pieces go into the game project folder if the project has one, otherwise into a folder you choose.
   - As with exports into a game project, folders are never created, and only files the app wrote are replaced. A piece that can't be written says why; the others still go.
5. **File format 20** stores each object's collision. Older files open with none.

## Consequences

- A kit exported into a Godot project imports as ready-to-place pieces with collision, with no work in Godot.
- Unity and Unreal Engine get the pieces without collision. Their own ways (Unity's mesh colliders, Unreal's `UCX_` shapes) can come later.
