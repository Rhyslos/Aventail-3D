# Seam blending plan

The working plan for seam blending in the texture painter ([decision 0293](../decisions/0293-blend-seams.md); the roadmap is in [DESIGN.md](../../DESIGN.md#after-phase-12s-milestones--seam-blending-painter), README → Further ahead). It records what the project lead asked for, the controls they chose, the design and architecture, the tests, and the plan for each milestone. Update it as milestones are built.

## What the project lead asked for (2026-11-07)

- "Mark an edge, and then smoothly blend the left and right side" of a texture seam.
- **Both a tool and a brush.** The tool blends whole seams in one go; the brush touches up as you stroke along a seam.
- **Pick seams by clicking them in the 3D view.**
- **The result goes in a new layer on top**, which remakes itself when the paint under it changes.
- Built after the material milestones (Phase 12, now done).
- **Asked for on 2026-11-13**, the two seam features milestone 1 left out: **picking seams in the UV Editor** (milestone 3) and **blending seams where two materials meet** (milestone 4).

Why it is needed: strokes on the model already paint both sides of a UV seam at once (decision 0179). Paint laid flat on the texture doesn't know where the other side of a cut is: the UV Editor, UV-aligned strokes, projection, clone, pictures brought in, graph fills, gradients. There the cut shows as a hard line.

## Controls (chosen by the project lead, 2026-11-07)

- **Blend seams** is **key 9** in Paint mode, and a button in the tool row (after the bucket, before the colour picker). Key 9 was free in Paint mode (1 to 8 are the other paint tools); in the Material editor's painting view it says the tool works only on a model, as 3, 4 and 6 do there.
- **Click** a seam in the 3D view to mark it (in place of those marked); **Shift + click** marks more. Shift + click on a seam already marked unmarks it (the same gesture, so no new one).
- An **Every seam that shows** button (in the Paint tab while the tool is in use, and in the Seam blend layer's settings).
- The result goes in a new **Seam blend** layer that remakes itself.
- The **seam blend brush** (milestone 2) is a brush on the **shelf**, with **no key of its own**.
- **In the UV Editor** (milestone 3) the same gestures: click near either of a seam's two lines marks it, Shift + click marks more or unmarks; the same buttons.
- Nothing else: no other keys or mouse actions (decision 0028).

## Where things stand

The standing OK carries on: each pull request is merged once its checks pass. The project lead tests everything at the end from the testing document.

| # | Milestone | PR | Decision | File format |
|---|---|---|---|---|
| 1 | The Blend seams tool, picking seams, the self-remaking Seam blend layer | see the PR list | [0293](../decisions/0293-blend-seams.md) | 111 |
| 2 | The seam blend brush on the shelf | see the PR list | [0294](../decisions/0294-seam-blend-brush.md) | 111 (unchanged) |
| 3 | Picking seams in the UV Editor | see the PR list | [0295](../decisions/0295-picking-seams-in-the-uv-editor.md) | 111 (unchanged) |
| 4 | Blending seams where two materials meet | — | 0296 | 112 if marks need it |

Milestones 1 to 3 are built; **milestone 4 is next**. Next decision: 0296. Next file format: 112 (graph and box files 7, packs 2, unchanged by milestones 1 to 3).

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

**In the UV Editor** (milestone 3) each seam is drawn as its two sides, one along each piece (`uv_editor/seam_lines.rs`), for the objects the editor shows (the selected ones). Its faint lines are worked out once per drawing, material, marks and objects shown. Pointing at a seam in either view lights the line in both: on the model, and both its sides on the texture.

## Architecture

- **`paint`** (core): `seams` (finding seams and lines, `showing`), `seam_blend` (`SeamBlend` settings, `BlendShape`, `resolve`, `tiles_used`, `blend`), `Layer::seam_blend`, `TextureSet::below`, `composite::flatten_tiles`; `advice::painted_differently` shared with "Why does this look wrong?".
- **`fileio`**: file format 111 (`project/v111.rs`, `LayerV64::seam_blend`); style presets keep the width, not the seams.
- **`paint`** (milestone 2): `BrushKind::SeamBlend` and `Brush::paints_through`; `seam_brush` (`SeamBrushShape`: a shape's seams, its `SeamLayout` and seams by triangle, `seams_near`; the stroke's pulls behind `StrokeCanvas::set_seams` and `reach_seams`); `seam_blend::blend_side` shared through a `Sink`, `SeamLayout` (pieces and triangles by tile).
- **`ui`**: `PaintTool::Seams` and `PaintToolButton::Seams` (icon `PaintSeams`), key 9 (`brushing::paint_keys`, `shortcuts`); `workbench/seam_picking.rs` (the tool: picking, marking, the asks, the lines shown); `workbench/seam_blending.rs` (making the layers, seams kept per object and material, tile hashes); `paint_layers/seam_rows.rs` (the rows shared by the Paint tab and the Paint layers panel); `attributes/seam_section.rs` (the Paint tab's section while the tool is in use); `viewport/seam_lines.rs`.
- **`ui`** (milestone 3): `seam_picking::pick_flat_seam`, `flat_seam_lines`, `flat_seam_shapes`, `Picked::flat`, `NO_SEAM_ON_TEXTURE`; `uv_editor/seam_lines.rs` (`UvSeamLine`, `UvSeamLines`, `UvEditor::set_seam_lines`, `paint_seams`).
- Tools never change the scene directly: every change is a `Command::SetTextureSet` (one undo step each, the width's drag merged), and the made pixels join it.

## Tests

- **paint:** seams found on a two-piece square and the cube, not on matching UVs, other materials or open edges; lines between the same pieces; seams that show; blending meets in the middle, fades over the width, mirrored pieces, matching colours unchanged, wider reaches farther, unpainted sides and missing seams blend nothing, `tiles_used` covers what is written and blending from just those tiles gives the same, settings kept sorted and in range, the layers under a layer (groups), odd UVs never crash (fuzz).
- **fileio:** round trip, nothing new stored without a Seam blend, seams tidied and bad widths or edges refused, flipped bytes never crash (fuzz), the style preset keeps the width only, the version 111 sample, version 110 files open without Seam blends.
- **ui (workbench):** 9 and the button choose it and show the seams; a click marks the line on a new layer, made on another thread in the same undo step; Shift + click adds and unmarks; a click off the seams says why and paints nothing; Every seam that shows and Unmark all; the width remakes it and undo puts both back; orbiting and a layer above make nothing, paint under it remakes it; missing seams counted; realism channels blended; the Paint tab and the layer list show the rows.

- **ui (milestone 3, `workbench/seam_uv_tests.rs`):** both sides of every seam shown in the UV Editor while the tool is in use, gone with another tool; pointing at one side lights both and the line on the model; pointing on the model lights both sides in the editor; a click on either side marks the line, made, one undo step; Shift + click marks more and unmarks from the other side, a plain click replaces; a click off the lines says why and paints nothing; nothing selected shows none in the editor; odd places and zooms never crash and a click on a side picks its seam (fuzz).

## Milestone 1 (built): the Blend seams tool and the Seam blend layer

Everything above. See [decision 0293](../decisions/0293-blend-seams.md).

### What milestone 1 built, for milestone 2

- **Finding seams near a point:** `Workbench::pick_seam` (in `seam_picking.rs`) finds the seam edge nearest the mouse on the shape hit; `Workbench::seams_of(object, material, surface, slots)` gives a shape's seams, kept while its surface is the same (`seam_blends.seams`). `Workbench::seam_shapes(material, drawing)` gives every shown shape's.
- **The matching place across a seam** is worked out in `paint::seam_blend` (`Edge`, the mapping in `blend_side`): `along` and `across` from one side's edge, mapped through the other side's edge scaled by their lengths. The brush can reuse it: expose a small `paint::seam_blend::across(seam, side, uv) -> Option<Vec2>` (the matching texel on the other side) rather than copying the maths.
- **Which piece a texel belongs to** near a seam: `Cover` (private in `seam_blend.rs`) rasterises one piece's triangles over an area. The brush needs the same to keep to one side; make it `pub(crate)` or move it to `paint::seams` if the brush lives in `paint`.
- **Left out of milestone 1:** picking seams in the UV Editor (a click there says to click in the 3D view); seams across two materials; a choice of which channels to blend; width in centimetres (DESIGN.md's draft; pixels chosen, decision 0293); blending past the 0 to 1 square for trim repeats; the testing document wasn't updated from this session (see the pull request's How to test).

## Milestone 2 (built): the seam blend brush

See [decision 0294](../decisions/0294-seam-blend-brush.md).

- **What:** a **Seam blend** brush on the shelf (Effects) and in the brush bar's list, no key (`brush_bar::built_in`: hardness 0.5, flow 1). `BrushKind::SeamBlend`, an effect brush, saved as kind 8 (no new file format: an older version refuses a stroke kind it doesn't know as "made by a newer version").
- **How it works:** at the stroke's start the workbench gives the canvas the shape (`Workbench::seam_brush_shape`, kept while the shape, its seams and the texture size stay the same; `StrokeCanvas::set_seams`). Before each dab is painted, `StrokeCanvas::reach_seams` readies the seams within twice the dab's radius (found through `PaintSurface::near` and the seams by triangle), each the first time the stroke comes near it: `seam_blend::blend_side` (the milestone 1 mapping, shared through its `Sink`) gives each painted texel of each side within the width its colour across the seam (from the layer as the stroke began) and strength (0.5 on the seam, smoothstep to 0 at the width). The **width is the dab's radius** in that side's texels (1 to 64). Painting a texel mixes it towards that colour by weight × flow × strength (`Band::cover_at`); a texel no seam reaches is left alone and uncovered, so a later dab reaching a seam still blends it.
- **Away from seams it does nothing** (the project lead, 2026-11-13); the status bar says *No seam under the brush* while a frame's dabs find none (`brushing::NO_SEAM_UNDER_BRUSH`), cleared when one does.
- **Sides facing away:** `Brush::paints_through` is true for it, so a side round a corner (edge on, or turned away) moves with the side facing the camera; only texels near seams near the dab change.
- **Where:** the chosen paint layer, strokes kept with it; one undo step a stroke ("Seam blend stroke"). The seams are drawn faintly while it is in use (`update_seam_tool`, `seam_blend_brush_on`). In the UV Editor it says to stroke in the 3D view; on a Material editor painting it says it works on a model. Resizing the texture resizes such a layer as a picture (effect strokes aren't painted again, decision 0191).
- **Speed:** `seam_brush::tests::seam_brush_speed` (ignored timing, 4096 texture): about as quick as a Blend dab or quicker (2.1 ms against 2.0 ms at 5 cm, 11 ms against 16 ms at 20 cm, release, two cores).
- **Tests:** paint (`seam_brush/tests.rs`): a dab on a seam closes it from both sides and fades away from it; weaker flow closes less; away from seams nothing changes; a stroke reaching a seam later still blends what it crosses; matching colours and unpainted sides unchanged; no shape or another size does nothing; seams near a point; a side turned away round a corner blended too; resize falls back to a picture; odd UVs and radii never crash (fuzz). ui (`workbench/seam_brush_tests.rs`): the shelf chooses it with no key and the seams show; a stroke along a cube seam lowers the difference across it to under half, on the paint layer, in one undo step; away from seams nothing changes and the status bar says so.
- **Left out:** painting in the UV Editor (a dab there reaches one side only); blending paint that is on other layers (it works on the chosen layer's own paint, as Blend does); a width setting of its own (the brush's size is the width). Seams across two materials and trim repeats past the square are still left out, as in milestone 1.

## Milestone 3 (built): picking seams in the UV Editor

See [decision 0295](../decisions/0295-picking-seams-in-the-uv-editor.md). Asked for by the project lead on 2026-11-13.

- **What:** while Blend seams is in use, the UV Editor draws both sides of each seam of the objects it shows (faint, accent when marked, white under the mouse). A click near either side marks the seam's line, Shift + click marks more or unmarks a marked one, exactly as on the model (`carry_out_seam_clicks` takes the editor's brush events; `mark_seams` unchanged, so one `SetTextureSet` undo step with the blend joining it). Every seam that shows and Unmark all work as before (they don't depend on a view). A click away from the lines says `NO_SEAM_ON_TEXTURE`.
- **Lit together:** the line picked in either view is lit in both (`Picked::lines` on the model, `Picked::flat` both sides on the texture; the 3D pick lights the editor only for an object it shows).
- **Which material:** the piece under the mouse's (`flat_hit`), then the Paint layers panel's; the faint lines are the panel's material's.
- **Speed:** no seams worked out per frame. The editor's lines are kept by drawing version, material, marks and objects shown (`SeamPicking::flat_lines`); the hover tests every side of the shown objects' kept seams against the mouse (as the 3D view does for the shape hit).
- **Left out:** while the editor isolates the selection's pieces, seam lines of hidden pieces still show; the seam blend brush still strokes only on the model; seams across two materials (milestone 4).

### Notes for milestone 4

- **Where it stands now:** `paint::seams::seams_of(surface, slots)` drops an edge whose two triangles are in different materials (`in_material`), and drops an edge whose two sides' UVs match. Between two materials, **matching UVs still make a join** (two textures), so milestone 4 needs its own finder, e.g. `seams_between(surface, slots_a, slots_b)`, keeping edges with one triangle in each, whatever the UVs.
- **Two textures:** each side is on its own material's texture set, possibly of different sizes. `seam_blend::blend_side` lays each side's edge out in its own texels (`Edge`); give it each side's size and read the other side from the other set's image. Each material's Seam blend layer would write only its own side.
- **No loops:** blend from the layers *under* each side's Seam blend layer (`TextureSet::below`), never from the other set's Seam blend layer, or A's remake changes B's input and back. The remake hash (`seam_blend_key`) must then include the other set's below-layers' tiles along those seams (`tiles_used` per side), so painting on B remakes A's layer.
- **Marks:** a cross-material seam belongs to two sets. Either mark it in both sets' Seam blend layers (one undo step: `Document::execute_all_labelled` takes several commands) or keep it once with the other material's name; either needs `SeamBlend` to store which material the other side is on, so likely file format 112 (`project/v112.rs`), with the fuzz and old-file tests as in milestone 1.
- **Picking:** `pick_seam` and `pick_flat_seam` look at one material's seams; they need the cross seams of the material hit too. In the UV Editor both sides can sit at the same place (different textures), so lighting both is still right; say in the status bar which two materials meet.
- **Showing:** `seams::showing` compares one image; a cross seam compares the two sets' composites (`advice::painted_differently` takes two images or a closure).
- **Testing:** a cube with two materials (side by side, matching UVs) painted two colours: a click on the shared edge marks it, both sets' layers blend towards the middle, undo puts both back, painting one side remakes the other.

