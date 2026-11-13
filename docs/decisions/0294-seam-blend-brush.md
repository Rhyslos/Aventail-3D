# 0294. The seam blend brush

- **Status:** Accepted
- **Date:** 2026-11-13
- **Follows:** [0293](0293-blend-seams.md) (Blend seams and the Seam blend layer), [0186](0186-effect-brushes-projection-clone.md) (effect brushes), [0179](0179-the-round-brush.md) (painting both sides of a seam), [0191](0191-texture-size-without-blur.md) (strokes painted again at a new size), [0028](0028-ask-about-controls.md) (asking about controls)

## Problem

The project lead asked on 2026-11-07 for seam blending as a tool and as a brush. The tool (milestone 1, decision 0293) blends whole seams evenly into a layer of its own. For touch-ups (a narrow spot, a seam only partly showing, a join wanted softer in one place) a brush is wanted that evens out the two sides of a seam where it is stroked. This is milestone 2 of the [seam blending plan](../plans/seam-blending.md).

## Options

- **Controls** (the project lead, 2026-11-07): a brush on the **shelf** (and the brush bar's list), **no key of its own**; no other keys or mouse actions. The usual brush keys and Ctrl + drags apply.
- **Away from seams** (asked on 2026-11-13): (a) blend like the Blend brush, or (b) **do nothing** (the project lead's answer): it can be swept loosely along a seam without smudging the paint beside it, and the status bar says when no seam is under it.
- **Where it paints** (the plan): into the **chosen paint layer**, as a new effect brush kind like Blend and Blur, its strokes kept with the layer; not into the Seam blend layer, which stays made.
- **How far it reaches from a seam.** (a) A width of its own in the Paint tab: one more setting. (b) **As far as the brush reaches** (chosen): the dab's radius on the surface, in each side's own texture pixels (1 to 64), the blend fading out by the same smoothstep as the layer's. A bigger brush blends a wider band; no new setting.
- **Sides facing away.** Seams often run round corners, where one side faces away from the camera. Painting only what faces the camera moved one side half way and left a step. The seam blend brush **paints through** facing and hidden tests: it changes only texels near seams near the dab (in 3D), so nothing else on the far side is touched.
- **In the UV Editor** a dab reaches only one side of a seam (they lie apart on the texture), so it couldn't close a join. It says to stroke in the 3D view instead, as Blend seams does for clicks. On a painting in the Material editor it says the tool works on a model, as projection and clone do.
- **Keeping painting smooth.** Working out every seam of a shape when a stroke begins is wasted on a heavy shape. Each dab readies only the seams near it, the first time a stroke comes near each (seams found through the triangles near the dab), and works out for each texel near them the colour across the seam it is pulled towards, from the layer as the stroke began; painting a texel is then one lookup and one mix. The shape's layout (pieces, triangles by tile) is kept while the shape and texture size stay the same. Timed on a 4096 texture, a seam blend dab is as quick as a Blend dab or quicker (`seam_brush::tests::seam_brush_speed`).

## Decision

- `paint::BrushKind::SeamBlend` (saved as kind 8; an older version opening a file with such a stroke says it was made by a newer version, so no new file format), an effect brush; `Brush::paints_through`.
- `paint::seam_brush`: `SeamBrushShape` (a shape's seams, its `seam_blend::SeamLayout` and its seams by triangle; `seams_near`), and the stroke's pulls (`StrokeCanvas::set_seams`, `StrokeCanvas::reach_seams`); a texel away from every seam is left alone and uncovered.
- `paint::seam_blend::blend_side` is shared by the layer and the brush through a `Sink` (the layer keeps the other side's colour as see-through as the blend is strong; the brush keeps the colour to pull towards and how strongly), with `SeamLayout` (pieces and triangles by tile) in place of the private shape it used.
- `ui`: the **Seam blend** brush in `brush_bar::built_in` (hardness 0.5, flow 1); `Workbench::seam_brush_shape` keeps the last shape; the seams drawn faintly while the brush is in use; the status bar's *No seam under the brush*.

## Consequences

Seams can be touched up by hand on the paint itself, with the brush's size, falloff, flow and pen pressure, and strokes that stray off a seam change nothing. Like the other effect brushes, its strokes aren't painted again at a new texture size: such a layer is resized as a picture. It works on the chosen layer's own paint, so paint on another layer isn't blended (the manual says to choose the layer the paint is on).
