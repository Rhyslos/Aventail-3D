# 0276. Foliage output: opacity, translucency, leaf sheets and alpha cards

- **Status:** Accepted
- **Date:** 2026-11-13

## Problem

Milestone 14 of Phase 12 ([the plan](../plans/phase-12.md)): foliage needs **alpha cards**, flat quads in a game textured with leaves and see-through around them. That takes an **opacity** (alpha) map from the drawn shapes, a **translucency** map (light passing through a leaf from behind), normals from the rounded height, several leaves or twigs laid out on one texture (an **atlas**, or leaf sheet) with the app's packer, and an export that sets alpha up right for each engine. Whole materials had no opacity at all: every box handing on a material, the preview, the export and the texture budget had to learn it without changing what any existing graph makes. The Leaves starter left over from milestone 8 was to be written once this worked.

## Options

**Where opacity lives.** (a) A new value kind (an "alpha card") beside shape, height, colour and material; (b) **optional maps on the whole material** (chosen): `MaterialMaps::opacity` and `translucency`, both `Option`. `None` means solid and no light through, which is what every material was before, so existing graphs, their caches and their files are unchanged, and nothing is saved differently (no file format change). Boxes handing a material on keep them (Transform, Repeat, Slope blur, Fill the gaps, the painter's laying out); Mix materials mixes them, a missing one standing in as solid or as no light; Scatter keeps copies of a card see-through and makes the gaps between them see-through, so leaves scatter onto a card. (a) would have needed new wires, conversions and refusals everywhere for one use.

**How a material becomes a card.** (a) A setting on Material out or Material from height; (b) **a box of its own, Opacity** (Finish group, chosen): Material, Opacity (a shape: the Shape box's Mask), Thickness (a height, optional). It makes the opacity from the shape, and the translucency as *Translucency × opacity × (1 − Thick parts block light × thickness)*, the thickness being the material's own height when nothing is plugged in (so veins and stems stay darker). Translucency 0 makes no translucency map. It also **fills the colour under the cut-away parts from the nearest solid colour** (a pull-push over ever smaller copies): games blur textures a little and read smaller copies far away, and black there would leave a dark fringe round every leaf.

**Where the atlas is made.** (a) A box taking several materials and packing their pixels; (b) **the Shape box lays its drawing out as a leaf sheet** (chosen): Layout → **Leaf sheet**, with **Gap between cards** and **Turn cards to fit**. Each group, and pieces touching each other, is a card; the cards are packed by the UV packer (`uv::pack`, decision 0101) by the hulls round their outlines, as large as they all fit, at one scale (leaves keep their sizes against each other), a gap apart and half a gap from the edges. Only the finished outlines move (each piece's edge, variation and mirror image worked out first), so the sheet stays vectors: sharp at any size, turned freely without blurring, and the same at every size. (a) would have resampled pictures (blurring them when turned or sized) and needed several graphs. Normals come from Material from height on the sheet's rounded height, as for any material.

**Wrapping.** On a sheet nothing reaches an edge, so nothing wraps; tiled drawings still wrap. The seam check now judges an alpha card's colour only where it is solid, and its opacity too.

**Export, per engine** (the Material mode's Export textures…, `fileio::export::graph_textures`): the **opacity goes in the colour map's alpha** for every engine (glTF, Godot, Unity's Base Map and Unreal Engine all read it there), and with separate maps also as `_Opacity`. The light through goes beside as `_Translucency` (the card's colour times how much light passes: Godot's backlight, Unreal Engine's subsurface colour), or for Unity (mask-map packing) as `_Thickness` (HDRP's transmission thickness: black thin, white thick). Godot's `.tres` gets `transparency = 2` (alpha scissor), `alpha_scissor_threshold = 0.5`, `cull_mode = 2` (both sides) and `backlight_enabled` with `backlight_texture` (added to a black `backlight`). These names and numbers were checked against Godot 4.3's class reference (`BaseMaterial3D`, from the engine's own documentation source); they weren't tried in Godot itself in this session. Alpha scissor rather than blending: sharp edges, shadows, no sorting problems, and the usual choice for foliage.

**The preview.** The material's opacity is the preview texture's alpha; a new `DrawItem::cut_out` makes the viewport's shader discard the surface where it is under a half (as the alpha scissor does), so the ball, the cube and the selected model show holes. Painted sets in the scene are never cut: their alpha says where paint is. Flat shows the card over a checker; the box pictures and the processor-drawn ball show it see-through. The preview doesn't show the translucency (it would need a light behind the shape).

**The texture budget** counts the new maps through the same rule as the export (`maps_going_out`, now with `MapsUsed::opacity` and `translucency`), from the material last worked out.

## Decision

- **matgraph:** `MaterialMaps::opacity` / `translucency` (`opacity_at`, `is_cut_out`); the **Opacity** box (`boxes::opacity`, id `opacity`, inputs `material`, `shape`, `thickness`, output `material_out`, settings `translucency` 0.5, `thick` 0.6; `bled`); the Shape box's `layout` (Tiled, as drawn / Leaf sheet), `gap` (0 to 0.1, 0.02) and `turn` (on) settings, `drawn_on`; boxes handing materials on keep or mix the new maps; the seam check weighs an alpha card's colour by its opacity.
- **shapes:** `sheet` (`Sheet`, `SheetOptions`, `cards`, `MAX_GAP`), `raster::mask_on`, `layers_on`, `lines_on`, `veins_on`; depends on `uv` (same layer).
- **fileio:** `TextureMap::Opacity`, `Translucency`, `Thickness`; `MapsUsed::opacity`, `translucency`; `graph_textures::CardMaps`, `SetOptions::card`, `set_maps(packing, card)`, `write_set(…, card)`; `bake_file::godot_card_material` and `GodotCard`; the **Leaves** starter (tenth); card pictures show an alpha card over a dark checker.
- **render:** `SurfaceDraw::Painted::cut_out` (the object's marks' second bit), discarding where the painted texture's alpha is under a half.
- **ui:** `DrawItem::cut_out`; the preview, box pictures, budget and Export textures… learn alpha cards; the shape canvas's strip shows the sheet when Layout is Leaf sheet.
- **No file format change** and **no new keys or mouse actions** (settings only).

## Consequences

- A leaf drawn once, copied linked and varied, comes out of Export textures… as an alpha card that Godot cuts out and lights from behind with no extra steps; Unity and Unreal Engine get the maps they read, with the alpha in the colour.
- A graph material **given to a shape** (a graph fill, decision 0262) is still solid: paint layers have no opacity channel, so a model export carries no alpha. Alpha cards go out through Export textures…; cutting a painted model is a later step.
- The sheet packs by hulls, so a crescent's dent stays empty; cards can't nest in each other's dents.
- Scatter puts later copies over earlier ones even where the later copy is see-through.
