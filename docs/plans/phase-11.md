# Phase 11 plan: the texture painter

The working plan for Phase 11 ([decision 0177](../decisions/0177-texture-painter-plan.md); the roadmap is in [DESIGN.md](../../DESIGN.md#phase-11--texture-painting)). It was agreed with the project lead on 2026-10-28, after research into what game texture artists use, a written plan they reviewed, and four interface mockups (they chose mockup A). It records:

- what is built;
- how the work is being done;
- the controls and layout already chosen;
- the architecture;
- a plan for every milestone.

Update it as milestones are built: tick them off here and in the README tracker, and change a milestone's plan when its design changes.

## Where things stand

The project lead gave a standing OK (2026-10-28): **each pull request is merged as soon as its checks pass**, one milestone per pull request, in order, without pausing for feel checks. They test everything at the end from the testing document (the Claude Docs artifact "Aventail 3D: everything to test"), so **every milestone adds its test steps there**, in a "17. Texture painting" section.

| # | Milestone | PR | Decision | File format |
|---|---|---|---|---|
| 1 | Textures on materials | #178 | [0178](../decisions/0178-textures-on-materials.md) | 64 |
| 2 | Paint mode and the round brush | #179 | [0179](../decisions/0179-the-round-brush.md) | 65 |
| 3 | Painting in the UV Editor | #180 | [0180](../decisions/0180-painting-in-the-uv-editor.md) | 65 |
| 4 | The Paint layers panel and the Paint layout | #181 | [0181](../decisions/0181-paint-layers.md) | 66 |
| 5 | Masks and the polygon fill tool | #182 | [0182](../decisions/0182-masks-and-polygon-fill.md) | 67 |
| 6 | Colour: the picker, P and palettes | #183 | [0183](../decisions/0183-colour.md) | 68 |
| 7 | Brush feel: pens, curves, smoothing, lines | #184 | [0184](../decisions/0184-brush-feel.md) | 69 |
| 8 (first part) | The brush bar and tips | #185 | [0185](../decisions/0185-brush-bar-and-tips.md) | 70 |
| 8 (second part) | Effect brushes, projection and clone | #186 | [0186](../decisions/0186-effect-brushes-projection-clone.md) | 71 |
| 9 | Mirror painting | #187 | [0187](../decisions/0187-mirror-painting.md) | 71 |
| 10 | Speed and big textures | #188 | [0188](../decisions/0188-speed-with-big-textures.md) | 71 |
| 11 | Paint survives modelling | #189 | [0189](../decisions/0189-paint-follows-the-model.md) | 71 |
| 12 | Re-pack or re-unwrap without losing paint | #190 | [0190](../decisions/0190-repack-and-unwrap-keep-paint.md) | 71 |
| 13 | Change texture size without blur | #191 | [0191](../decisions/0191-texture-size-without-blur.md) | 71 |
| 14 | The baker | #192 | [0192](../decisions/0192-the-baker.md) | 71 |
| 15 | Bakes as guides | #193 | [0193](../decisions/0193-bakes-as-guides.md) | 71 |
| 16 | Gradient fills | #194 | [0194](../decisions/0194-gradient-fills.md) | 72 |
| 17 | Smart masks | #195 | [0195](../decisions/0195-smart-masks.md) | 73 |
| 18 | Light and shade | #196 | [0196](../decisions/0196-light-and-shade.md) | 74 |
| 19 | Edge highlights | #197 | [0197](../decisions/0197-edge-highlights.md) | 74 |
| 20 | Paint previews | #198 | [0198](../decisions/0198-paint-previews.md) | 74 |
| 16–18 (more) | Gradients across parts and UV pieces, Warm and cool | #199 | [0199](../decisions/0199-kit-finishing-touches.md) | 75 |
| 21 | See it as the player will | #200 | [0200](../decisions/0200-see-it-as-the-player-will.md) | 75 |
| 22 | Outlines | #201 | [0201](../decisions/0201-outlines.md) | 76 |
| 23 | Paint trim sheets and kits live | #202 | [0202](../decisions/0202-paint-trim-sheets-live.md) | 76 |
| 24 | "Why does this look wrong?" | #203 | [0203](../decisions/0203-why-does-this-look-wrong.md) | 76 |
| 25 | Texture export | #204 | [0204](../decisions/0204-texture-export.md) | 76 |
| 26 | Vertex colour painting | #205 | [0205](../decisions/0205-vertex-colour-painting.md) | 77 |
| 27 | Presets and starting layouts | #206 | [0206](../decisions/0206-presets-and-starting-layouts.md) | 77 |
| 28 | Realism channels and a lit preview | see the PR list | [0207](../decisions/0207-realism-channels.md) | 78 |

Next decision: 0208. Next file format: 79.

Milestone 8 is two pull requests: the brush bar and tips, then the effect brushes (smudge, blend, blur, hue shift), projection and clone.

**Left for later milestones:** other objects blocking the brush (milestone 10).

## Choices already made (2026-10-28)

- **Works like Substance Painter in most ways** (the project lead: "very similar to Substance Painter in most regards"), built the app's own way: settings live in the **Attributes panel** as everywhere else. This replaces the design's old "not a Painter copy" line.
- **Controls: if in doubt, follow Substance Painter** (the project lead). Already chosen:
  - **[** and **]** brush size; **Ctrl + right-drag**: sideways size, up and down hardness; **Ctrl + left-drag**: sideways flow, up and down rotation;
  - **Shift + click** a straight line from the last stroke; **D** smoothing (lazy mouse); **L** mirror painting; **P** picks the colour (or material) under the pointer;
  - in Paint mode, **1** paint, **2** eraser, **3** projection, **4** polygon fill, **5** smudge, **6** clone; the palette or **Esc** leaves Paint mode. Alt navigation, the F1–F4 views and Ctrl + Z stay the app's own;
  - Substance Painter's Shift + right-drag turns the light; in the app's "Mouse only" navigation right-drag orbits, so check for a clash when it's built (milestone 21).
- **Paint mode** in the left palette, after UV. Choosing it shows the paint panels; leaving it puts them away.
- **Layout (mockup A):** the 3D view and the UV Editor side by side; the **brush bar along the bottom, in the place of Coordinates** (not needed while painting); **no modifiers palette** in Paint mode (decision 0176 hides it in the UV mode too); on the right, **Attributes** (brush settings and the colour picker) over **Paint layers**.
- **Colour picker:** built on Nodelit's (`Rhyslos/Nodelit`, `Client/src/components/colorpicker`): Wheel (hue ring with a square or triangle centre) and Detailed (honeycomb and sliders; HSV sliders instead of its CMYK) modes, saved colours, Hex and RGB fields with copy; plus an **Even** mode (lightness changes evenly), New and Before colours, the lightness as a number, light-and-shade and harmony rows, and picking choices. **To rework:** the mockup's picker is too big; make it compact.
- **Pen:** Wacom first, through Wintab (`wintab_lite`, MIT), with pressure and tilt; other pens through Windows (winit already passes pen pressure as touch force; egui gives it as `Event::Touch { force }`).
- **Hand-painted first**, realism after. Texture sizes small for stylised work.
- **Left out:** the timelapse (the project lead: "a gimmick that won't do us much now").

## Architecture

### Crates

- **`paint`** (new, core layer, no UI or GPU; depends on `mesh` and `glam`): texture sets, layers, tiled images, brushes and dabs, rasterising dabs onto a shape's UVs, mixing layers, padding (dilation), undo tiles. Everything testable without a window.
- **`scene`** holds the project's paint data (`Scene::paint`) and the commands that change it (`Command::Paint…`, `Edit::Paint…`), so undo, saving and the future Edit Layers stack work as for everything else.
- **`fileio`** saves it (format 64+, as trailing fields on `ProjectV43` with serde defaults; pixels compressed with zstd per tile, empty tiles skipped; limits through `crate::limits`), imports images (`image` crate, MIT/Apache, already in the lock file) and exports textures.
- **`render`** draws painted surfaces: `SurfaceDraw::Painted`, one texture per texture set, bound per material slot.
- **`ui`** holds Paint mode, the brush bar, the Paint layers panel, the brush and colour settings in Attributes, and the composite cache that feeds both views.
- **`app`** (`gpu.rs`) uploads changed texture tiles to the GPU.

### Texture sets

A **texture set** belongs to a **material, by name** (as export: materials of the same name are one material across objects, `fileio::export::file_materials`). Its size comes from the project's Texture settings (`scene::TextureSettings::size`) unless set on the texture set. Every polygon whose material has that name paints into that set, through its UVs (one UV set per shape today: `mesh::attributes::UV`, per corner).

The painted shape is the **shown shape** (`DrawItem::drawn`: edit layers, booleans, mirror, modifiers), the same as export's `scene.exported_shape` in all but LOD preview. The UV Editor shows `scene.evaluated(id)`; when they differ (modifiers that change UVs), the UV Editor paints the evaluated UVs, which are the same pixels wherever the modifier keeps UVs.

### Images

`paint::Image`: square, `size` pixels, split into **64 × 64 tiles**, each `Option<Arc<Tile>>` (RGBA, 8 bits, straight alpha; an empty tile is transparent). Arc tiles make undo snapshots and saving cheap: a stroke copies only the tiles it touches. Masks are the same with one channel used (kept RGBA for simplicity at first). Colours are sRGB bytes, as the palette and the checker image.

### Painting on the model (approach chosen from the research)

**Per dab, rasterise the nearby triangles in texture space.** For a dab at world point `c` with screen radius `r`:

1. Find the triangles of the shown shape near `c` (a BVH over its triangles, `mesh::face_triangles` cut, the same as picking and export).
2. For each, find the part of its UV triangle the dab can reach, rasterise those texels, and for each texel work out its point on the surface (barycentric), project it to the screen and weigh it by distance to the dab centre (the brush's falloff, hardness) and by how much the surface faces the camera.
3. Skip texels hidden from the camera (a ray test against the shape) unless "paint through" is on.

Both sides of a UV seam sit at the same place on the surface, so both are painted and the join doesn't show. No per-texel map is kept, so memory stays small at 4K.

**Strokes** are dabs every `spacing × diameter` along the path, pressure interpolated. Within a stroke, coverage takes the maximum (paint doesn't pile up where dabs overlap); the stroke is mixed onto the layer with its flow when each dab lands (build-up across strokes), as Krita and Photoshop do.

**Dabs are kept** with each stroke (world point, normal, radius, pressure, colour, brush), for milestones 11–13: replaying them repaints at a new texture size and after modelling changes.

**Painting in the UV Editor** stamps the dab straight into texture space.

**Padding:** on display and export, painted texels spread outward by the packing padding into empty texels, so seams don't show in mipmaps; the layers stay clean.

### Undo

A stroke is one `scene::Command`: the tiles it touched, before and after (`Arc<Tile>`), on one layer of one texture set. Memory is counted in `scene::memory`.

### Showing it

A `paint::Composite` per texture set mixes the visible layers per tile, keeping which tiles changed. The viewport gets the composite as a texture per set (dirty tiles uploaded with `queue.write_texture`); the UV Editor as an egui texture (`TextureHandle::set_partial`). `MeshGeometry` sorts triangles by material slot so a draw can bind each slot's texture set.

## The milestones

Group A, the core:

1. **Textures on materials:** texture sets by material, the Painted surface view (Display → Surface and the UV Editor), Import image as a layer, saved with the project.
2. **Paint mode and the round brush:** Paint mode, the brush settings in Attributes, painting on the model without seam joins, hidden parts left alone, undo per stroke, dabs kept, brush size in centimetres with a warning past the texture's detail.
3. **Painting in the UV Editor:** the same brush, live in both views.
4. **The Paint layers panel:** paint and fill layers, opacity, show and hide, order, blend modes (Normal, Multiply, Screen, Overlay, Add, Subtract, Colour, Hue), merge down, clip to the layer below, groups.
5. **Masks:** a paintable mask on any layer, invert; the polygon fill tool (4) fills polygons, UV islands or a material.
6. **Colour:** the colour picker in Attributes (above), P picking, palettes saved with the project and as files.
7. **Brush feel:** Wacom pressure and tilt through Wintab, then Windows pens; curves for size and flow; smoothing; mouse defaults; the eraser.
8. **The brush bar and brush set:** the brush bar in Coordinates' place (tips, sample strokes, names, filters, search, + to save); soft and hard round, chalk, bristle, dry brush, spatter, smudge, blend, blur, hue shift; projection (3) and clone (6); brushes as presets.
9. **Mirror painting.**
10. **Speed and big textures:** 4K and 20 layers stay smooth; undo within a memory budget; the heavy-mesh check covers painting.

Group B, paint that follows the model:

11. **Paint survives modelling.**
12. **Re-pack or re-unwrap without losing paint.**
13. **Change texture size without blur.**

Group C, the bakes the painter needs (from the shape itself; the `bake` crate Phase 8's baker grows from):

14. **The baker:** position, normals, AO, curvature, ID, thickness.
15. **Bakes as guides.**

Group D, the hand-painted kit:

16. **Gradient fills.**
17. **Smart masks:** edges, cavities, facing up, height, noise breakup.
18. **The light-and-shadow helper**, with warm light and cool shade.
19. **Edge highlights.**
20. **Previews:** unlit, lit, toon, engine-true (Godot), value check and squint.
21. **See it as the player will.**
22. **Outlines.**
23. **Paint trim sheets and kits live.**
24. **"Why does this look wrong?"**

Group E, export and sharing:

25. **Texture export** (PNG, TGA, padding, linked into glTF and OBJ, presets, export on save, budget report).
26. **Vertex colour painting.**
27. **Presets and starting layouts.**

Gate: paint a stylised prop start to finish and see it right in Godot.

Group F, realism:

28. **Realism channels** and a lit preview.
29. **Engine channel packing.**
30. **Wear, dirt and rust masks.**
31. **Material layers.**
32. **Height and normal painting.**
33. **Live engine preview.**

The full description of each milestone is in the plan document the project lead reviewed; this list is kept in step with it.
