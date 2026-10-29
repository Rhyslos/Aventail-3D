# 0214. Warning when paint is shared by accident

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

The project lead found that painting the selected sphere also painted the cube and the cylinder beside it, and every shape made afterwards. A texture set belongs to a material ([0178](0178-textures-on-materials.md)), every new shape has the Default material, and every new shape's UVs fill the same square: so they all share one texture over the same spot. It looked like a bug.

## Options

Research (2026-10-29): Substance Painter keeps one texture set per material too; its users meet this as "paints in multiple places at once", and the answer is always to redo the UVs, as Painter never warns. Blender shares the paint when objects use the same image. One texture per material is right for games (fewer textures, atlases, the export already works so), so it stays.

- **Every new shape gets its own material:** never shared by accident, but the material list grows with every shape, and kits that should share a texture get split.
- **Warn, with fixes (chosen):** keep the model, catch the case.

## Decision

- While painting one selected shape, the workbench looks (when the scene or the selection changes) for other shapes with one of its materials whose UV pieces lie over (or exactly on) its own, with `uv::check::check_sheet` at 256 cells across. Overlaps within one shape, pieces on the trim sheet and instances don't count.
- The **Paint layers** panel shows a yellow note naming them, with **Give it its own texture** (the shape's material renamed to an unused "Default – Sphere", the texture set copied under the new name, so nothing seen changes) and **Pack them onto one texture** (Pack of all their pieces together by material, whatever the per-object setting; paint follows the UVs, [0190](0190-repack-and-unwrap-keep-paint.md)). Each is one undo step.
- No keys or mouse actions.

## Consequences

- The surprise is explained where it happens, with the two usual fixes a click away; projects that share on purpose keep working.
- The project lead chose both fixes (2026-10-29).
