# 0100. Texture size and texel density as project settings

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0095](0095-uvs-before-advanced-modelling.md) (the UV phase plan), [0099](0099-unwrap-relax-straighten.md) (unwrapping at the object's density), [0068](0068-pivot-editing-and-scene-units.md) (project settings saved with the project)

## Problem

UV work is measured in texture pixels: how big a piece is on the texture, how much padding goes between pieces (packing, next), whether two props look equally sharp in the game. The app needs to know the texture's size and the texel density aimed for, show the density of what is selected, and set or even out densities.

## Options

- **Where the settings live:** **with the project, saved in its file, changed as one undo step** (chosen; like units and the game project folder), because a texture belongs to the asset; a global setting would be wrong for the next project.
- **What they are:**
  - **a square texture size** from 256 to 16384 pixels (any size from 16 is kept; the menu offers the powers of two) and **a texel density in pixels per metre** (512 px/m as standard, 2048 px as standard) (chosen). Non-square textures are rare for game props and can come later;
  - a density in pixels per centimetre, or per scene unit: less common in game work.
- **Where they're changed:** the UV Editor's **Texture** menu and **UV → Texture** (chosen). A drag of the density field is one undo step.
- **Showing density:** the UV Editor's bottom right reads the texture size, the density of the selected polygons (or of everything shown) and the aim: "2048 px · selected 604 px/m (aim 512)". A coloured density view comes with the checker and stretch heatmap milestone.
- **Changing density:** **Set texel density** scales each chosen piece (about its own middle) to the aim; **Match texel density** scales them to their shared average (chosen). Recorded as one edit, `SetTexelDensity { faces, density }`, with the density in texture widths per scene unit, so the edit means the same if the texture settings change later.
- **Seeing pixels:** zoomed in until a texture pixel covers 8 screen points or more, the editor draws the pixel grid.
- **Snapping to pixels:** not yet; the gizmo still snaps moves to the square's tenths. Packing will place pieces on whole pixels.

## Decision

1. `scene::TextureSettings { size, density }` (`is_valid`, `uv_per_unit`, `pixels_per_metre`; the scene stores centimetres), `TEXTURE_SIZES`, `MAX_DENSITY`; `Scene::texture`, `with_texture`; `Command::SetTexture`, `Edit::SetTexture`.
2. `uv::density`: `areas`, `density`, `set_density`.
3. `scene::Operation::SetTexelDensity { faces, density, anchors }`.
4. **File format version 31** (`project/v31.rs`): `ProjectV31` = version 27's model (edits as `OperationV31`) plus `TextureV31`; older files open with the standard settings. Sample in `tests/data/sample-v31.avtl`.
5. UI: `uv_editor::texture_menu`, `density_text`, the pixel grid (`PIXELS_FROM`); `Workbench::run_texel_density`; the UV menu's entries.

## Consequences

- Packing can measure padding in real pixels, and the checker texture can use the real texture size.
- A texture size change doesn't move UVs; densities in pixels per metre change with it, as in other tools.
