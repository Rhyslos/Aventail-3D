# Seam blending plan

The working plan for seam blending in the texture painter ([decision 0293](../decisions/0293-blend-seams.md); the roadmap is in [DESIGN.md](../../DESIGN.md#after-phase-12s-milestones--seam-blending-painter), README → Further ahead). It records what the project lead asked for, the controls they chose, the design and architecture, the tests, and the plan for each milestone. Update it as milestones are built.

## What the project lead asked for (2026-11-07)

- "Mark an edge, and then smoothly blend the left and right side" of a texture seam.
- **Both a tool and a brush.** The tool blends whole seams in one go; the brush touches up as you stroke along a seam.
- **Pick seams by clicking them in the 3D view.**
- **The result goes in a new layer on top**, which remakes itself when the paint under it changes.
- Built after the material milestones (Phase 12, now done).

Why it is needed: strokes on the model already paint both sides of a UV seam at once (decision 0179). Paint laid flat on the texture doesn't know where the other side of a cut is: the UV Editor, UV-aligned strokes, projection, clone, pictures brought in, graph fills, gradients. There the cut shows as a hard line.

## Controls (chosen by the project lead, 2026-11-07)

- **Blend seams** is **key 9** in Paint mode, and a button in the tool row (after the bucket, before the colour picker). Key 9 was free in Paint mode (1 to 8 are the other paint tools); in the Material editor's painting view it says the tool works only on a model, as 3, 4 and 6 do there.
- **Click** a seam in the 3D view to mark it (in place of those marked); **Shift + click** marks more. Shift + click on a seam already marked unmarks it (the same gesture, so no new one).
- An **Every seam that shows** button (in the Paint tab while the tool is in use, and in the Seam blend layer's settings).
- The result goes in a new **Seam blend** layer that remakes itself.
- The **seam blend brush** (milestone 2) is a brush on the **shelf**, with **no key of its own**.
- Nothing else: no other keys or mouse actions (decision 0028).

## Where things stand

The standing OK carries on: each pull request is merged once its checks pass. The project lead tests everything at the end from the testing document.

| # | Milestone | PR | Decision | File format |
|---|---|---|---|---|
| 1 | The Blend seams tool, picking seams, the self-remaking Seam blend layer | see the PR list | [0293](../decisions/0293-blend-seams.md) | 111 |
| 2 | The seam blend brush on the shelf | | | |

Next decision: 0294. Next file format: 112 (graph and box files 7, packs 2, unchanged by milestone 1).

## Design

### Seams

A **seam** is an edge two triangles share by their points but not by their UVs: one place on the model, two places on the texture. It is found on the **shown shape** (the one the brush paints, `DrawItem::drawn`), per object and per material, by pairing triangle edges by their two point ids (`paint::seams::seams_of`). An edge shared by more than two triangles, or between two materials, isn't a seam of a texture and is left out.

Each seam has two **sides**: the triangle along it, the UVs at its two ends (in the order of the lower point id first, so both sides line up end to end however their pieces are turned or mirrored), the UV of the triangle's third corner (which way its piece lies) and its **piece** (UV island, `paint::fill::pieces`).

A seam is named by the object and its two point ids (`SeamId`). Point ids stay while the model is moved, posed, or its UVs are moved, packed or unwrapped, so marks survive those. A mark no longer on the model is kept (undo may bring the edge back) and counted; the panels say how many are missing.

**Clicking marks a seam line:** every seam edge between the same two pieces joined end to end with the one clicked (`seams::line_of`), which is what a person sees as one seam.

### Blending

For each marked seam and each of its sides (`paint::seam_blend::blend`):

1. The side's edge is laid out in texels: where it starts, which way it runs, how long it is, and which way its piece lies.
2. Texels within the width of the edge, **inside that side's piece** (its triangles rasterised with the painter's 0.75 texel reach, so the texels along the edge count), and painted underneath, are visited.
3. Each texel's place is measured from the edge: how far along and how far in. The **matching place on the other side** is the same distance along and in from the other side's edge, scaled by the two edges' lengths (a side drawn smaller is sampled at its own scale). That is where the same point of the model's surface would be if the cut were closed.
4. The other side's colour there is read (bilinear, from texels of that side's piece that are painted; nearer the seam when the other piece is thinner).
5. The texel is blended towards it by `0.5 × (1 − smoothstep(distance / width))`: half way on the seam, nothing at the width. Both sides meet in the same middle colour, so the join disappears.
6. A texel near two seams takes the nearer.

The layer holds the other side's colour with that strength as its see-through amount, so over the layers under it (Normal blend) it gives the blended texture. With the realism channels on, roughness, metal, glow and height are blended the same way from their own mixes.

**Mirrored and turned pieces** are handled by the sides' own edge directions and inward sides. **Overlapping pieces** (mirrored halves sharing texels) are told apart by their triangles, not the texels. **Padding** isn't read or written (only texels inside pieces), so the display and export padding (decision 0177) spreads the blended texels as usual. **Several texture sets:** a Seam blend layer belongs to one material's set and blends only its seams. **UVs past the 0 to 1 square** (trim repeats) blend only the part on the square.

### The layer remaking itself

A Seam blend layer is a made layer (`Layer::is_made`): strokes go to another layer. It keeps `SeamBlend::made`, a hash of what its pixels were made from:

- its seams and width, the texture size and realism switch;
- the material's layout key (`paint::follow::layout_key`) and where each of its seams lies on the texture;
- the settings of the layers **under it** (`TextureSet::below`: those before it in its group, and so on outwards), their strokes left out;
- their pixels on just the tiles its seams reach (`seam_blend::tiles_used`), hashed by tile and kept by the tile's pointer (tiles are shared and never changed in place, so only new tiles are read).

Whenever the scene or the drawing changes and nothing is held down, the first layer whose hash differs is made again on another thread (`workbench/seam_blending.rs`): the layers under it mixed on just those tiles (`composite::flatten_tiles`), then blended. The result joins the change's undo step (`Document::execute_with_last`), as graph fills do (decision 0261), if nothing changed meanwhile; otherwise it is worked out again. So:

- marking, unmarking and the width remake it once (after a drag, when let go);
- painting near a seam remakes it when the stroke ends; painting far from its seams, panning and orbiting, and changing layers above it make nothing;
- undo puts the settings and pixels back together (the layer undone carries its old hash).

### Showing the seams

While the tool is in use the views draw the material's seams (the material shown in the Paint layers panel) faintly, the marked ones in the accent colour, and the seam line under the mouse in white (`viewport/seam_lines.rs`). A seam both of whose sides face away from the camera isn't drawn. The faint lines are worked out once per drawing, material and marks; only the line under the mouse is found each frame (seams near the point the mouse hits, within 10 points on screen).

## Architecture

- **`paint`** (core): `seams` (finding seams and lines, `showing`), `seam_blend` (`SeamBlend` settings, `BlendShape`, `resolve`, `tiles_used`, `blend`), `Layer::seam_blend`, `TextureSet::below`, `composite::flatten_tiles`; `advice::painted_differently` shared with "Why does this look wrong?".
- **`fileio`**: file format 111 (`project/v111.rs`, `LayerV64::seam_blend`); style presets keep the width, not the seams.
- **`ui`**: `PaintTool::Seams` and `PaintToolButton::Seams` (icon `PaintSeams`), key 9 (`brushing::paint_keys`, `shortcuts`); `workbench/seam_picking.rs` (the tool: picking, marking, the asks, the lines shown); `workbench/seam_blending.rs` (making the layers, seams kept per object and material, tile hashes); `paint_layers/seam_rows.rs` (the rows shared by the Paint tab and the Paint layers panel); `attributes/seam_section.rs` (the Paint tab's section while the tool is in use); `viewport/seam_lines.rs`.
- Tools never change the scene directly: every change is a `Command::SetTextureSet` (one undo step each, the width's drag merged), and the made pixels join it.

## Tests

- **paint:** seams found on a two-piece square and the cube, not on matching UVs, other materials or open edges; lines between the same pieces; seams that show; blending meets in the middle, fades over the width, mirrored pieces, matching colours unchanged, wider reaches farther, unpainted sides and missing seams blend nothing, `tiles_used` covers what is written and blending from just those tiles gives the same, settings kept sorted and in range, the layers under a layer (groups), odd UVs never crash (fuzz).
- **fileio:** round trip, nothing new stored without a Seam blend, seams tidied and bad widths or edges refused, flipped bytes never crash (fuzz), the style preset keeps the width only, the version 111 sample, version 110 files open without Seam blends.
- **ui (workbench):** 9 and the button choose it and show the seams; a click marks the line on a new layer, made on another thread in the same undo step; Shift + click adds and unmarks; a click off the seams says why and paints nothing; Every seam that shows and Unmark all; the width remakes it and undo puts both back; orbiting and a layer above make nothing, paint under it remakes it; missing seams counted; realism channels blended; the Paint tab and the layer list show the rows.

## Milestone 1 (built): the Blend seams tool and the Seam blend layer

Everything above. See [decision 0293](../decisions/0293-blend-seams.md).

### What milestone 1 built, for milestone 2

- **Finding seams near a point:** `Workbench::pick_seam` (in `seam_picking.rs`) finds the seam edge nearest the mouse on the shape hit; `Workbench::seams_of(object, material, surface, slots)` gives a shape's seams, kept while its surface is the same (`seam_blends.seams`). `Workbench::seam_shapes(material, drawing)` gives every shown shape's.
- **The matching place across a seam** is worked out in `paint::seam_blend` (`Edge`, the mapping in `blend_side`): `along` and `across` from one side's edge, mapped through the other side's edge scaled by their lengths. The brush can reuse it: expose a small `paint::seam_blend::across(seam, side, uv) -> Option<Vec2>` (the matching texel on the other side) rather than copying the maths.
- **Which piece a texel belongs to** near a seam: `Cover` (private in `seam_blend.rs`) rasterises one piece's triangles over an area. The brush needs the same to keep to one side; make it `pub(crate)` or move it to `paint::seams` if the brush lives in `paint`.
- **Left out of milestone 1:** picking seams in the UV Editor (a click there says to click in the 3D view); seams across two materials; a choice of which channels to blend; width in centimetres (DESIGN.md's draft; pixels chosen, decision 0293); blending past the 0 to 1 square for trim repeats; the testing document wasn't updated from this session (see the pull request's How to test).

## Milestone 2 (to build): the seam blend brush

- **What:** a brush on the **shelf** (Brushes tab, no key of its own) that evens out the two sides of a seam as you stroke along it, for touch-ups where the tool's even blend isn't wanted (a narrow spot, a seam only partly showing).
- **How it should work:** a new `BrushKind::SeamBlend` (an effect brush like Blend and Blur, decision 0186), offered as a ready-made brush on the shelf and in the brush bar's list (`brush_bar::BrushPreset`). Each dab, for the texels under it near a seam (the shape's seams near the dab, `seams_of`), it mixes each texel towards its match across the seam (the milestone 1 mapping) by the brush's flow and falloff, on the layer being painted, in both sides at once, so a stroke along a seam closes it. Away from any seam it does nothing (or blends like Blend: ask the project lead).
- **Where it paints:** into the chosen paint layer, like the other effect brushes (strokes kept for replay, decision 0179), not into the Seam blend layer, which stays made.
- **Controls:** none new (shelf only, chosen 2026-11-07). The usual brush keys and Ctrl + drags apply.
- **Tests:** the dab closes a seam on the two-piece square in `paint`; on the cube in the workbench a stroke along a seam lowers the difference across it; away from seams nothing changes; undo is one step; the shelf shows it; replaying the stroke (resize, decision 0191) gives the same.
- **Docs:** the manual's *Blending seams* section in `painting-textures.md` gains the brush; the decision for it is 0294.
