# 0260. Material preview and checks

- **Status:** Accepted
- **Date:** 2026-11-07

## Problem

Phase 12's fourth milestone ([the plan](../plans/phase-12.md), [decision 0256](0256-material-graph-plan.md)) is "Preview and checks": the Material mode's preview should show the graph's material on the selected model, a ball, a cube or a flat tiled square, in the app's looks (unlit, lit, toon, realistic); a seam check should show where the tiles join; and the texture budget should say what the material costs at the size chosen. Milestone 2 ([decision 0258](0258-node-canvas.md)) had left a preview that lit a ball on the processor and showed the texture flat, two by two.

Things to decide: how the 3D shapes are drawn, how big the material is laid on them, how "do the joins show" is decided, and what the budget counts.

## Options

- **Drawing the shapes:** keep lighting them on the processor (no model, no cube, the looks only imitated), write a small renderer for the preview, or **use the views' own renderer** (chosen). The viewport already draws painted surfaces in every look the painter offers, with roughness, metal and height (decisions 0178, 0198, 0207, 0211). Drawing the preview with it means the preview looks exactly as the material will on a shape, and every later improvement to the looks shows in the preview too.
- **Where the preview draws:** in one of the four view images (it would swap the scene's shapes and textures out of the graphics card each frame, and a hidden viewport tab shares the slots), or **a second renderer of its own on the same device**, made the first time the preview is shown (chosen). It keeps its own shape and texture, so the views never lose theirs.
- **How big the material is:** a fixed number of repeats, or **its real-world size** (chosen): the material covers Material out's real-world size (`eval::real_size_cm`), and a shape's UVs say how many centimetres one texture width covers on it (the square root of its UV area over its real area, `mesh::uvs::average_density`, times the placement's scale). A 1 m stone material shows 1 m stones on any model, as it will once given to that model.
- **The seam check:** only drawing the joins, or **drawing them and checking them** (chosen): the average step in value across each join is compared with the average step between neighbouring pixels everywhere else. A join that jumps more than three times as much (and more than about two levels in 255) shows. Every channel is checked (a material's colour, height and roughness) and the worst counts.
- **The budget's maps:** a fixed list, or **the maps the export will write** (chosen): the painter's export already decides them from the engine preset and channel packing (decision 0208). That rule is now one function, `fileio::export::maps_going_out`, which both the export and the budget call, so they always agree.

## Decision

- **Preview shapes:** **Model** (the first selected object with a surface), **Ball** (the Sphere shape, 1 m across), **Cube** (the Cube shape, 1 m) and **Flat**. Ball is the standard. The 3D shapes are seen from the front right, a little above, framed whole, on the Studio look's backdrop with no grid.
- **Looks:** **Unlit**, **Lit**, **Toon** and **Realistic**, the painter's previews of those names. The material is laid into one texture (its colour; roughness, metal and height as the realism maps) repeated by its real-world size across the shape's UVs, from 256 to 2048 pixels (512 while something is held down), made again only when the material, the shape or its size changes. A model without UVs says so and offers Ball or Cube; with nothing selected, Model says to select one.
- **Renderer:** `ViewportRenderer::render_preview` draws a view apart from the four views; the app's renderer keeps a second `GpuViewport` for it. A renderer that can't (no graphics card) says why: the ball is then drawn on the processor as before, and the cube and model say the 3D preview can't be shown.
- **Seam check:** a switch under Flat. It shows the texture three by three with the joins marked (in the warning colour where they show) and says in plain words what it found (`matgraph::SeamCheck`).
- **Texture budget:** under the preview, for a size chosen from 256 to 4096 (the project's texture size at first). It lists the maps the export as set up now would write, what one and all of them take on the graphics card (four bytes a pixel and a third more for the smaller copies, as the Export window counts), and the sharpness in pixels a metre at the real-world size against the project's texel density. It warns when one material takes more than 128 MB, when it is less than half or more than twice the project's density (naming the size that would match), and when the export leaves textures out.
- **Giving a graph's material to a shape** is milestone 6's ("A graph material can also be given to a shape without painting"). The preview needs none of it: it lays the material on the selected model itself, without changing the scene. Nothing is saved, so no file format change.
- **Controls:** no new keys or mouse actions; the preview's choices are buttons, a switch and a list.

## Consequences

- The preview is as faithful as the viewport's looks: the material's own normal map isn't drawn (the views bump by the height, as painted height does), and occlusion and curvature aren't shown on the 3D shapes. Both go out with the export (milestone 6).
- The preview can't be turned round; the camera is fixed. Turning it would add a mouse action, which the controls agreed for Phase 12 don't have. It can be asked for later.
- A model whose UVs stretch shows the material stretched there, as the game will.
- The budget size isn't saved; the export's size is milestone 6's to choose, and it can start from this one.
- The Export window's texture budget and the Material mode's now share the map names (`material::budget::map_word`).
