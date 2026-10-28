# 0191. Change texture size without blur

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's thirteenth milestone ([0177](0177-texture-painter-plan.md)). Until now, changing the project's texture size (UV → Texture) left texture sets at the size they were painted. Enlarging paint as a picture softens it: hand-painted edges smear over several pixels. The strokes kept with each layer ([0179](0179-the-round-brush.md)) can be painted again at any size, crisp.

## Decision

- **Sets follow the project's texture size.** After the size changes, every texture set is made that size, in the same undo step (as paint follows the model, [0189](0189-paint-follows-the-model.md)).
- **Painted again where the strokes explain the layer.** For each paint layer and mask, the kept strokes are painted again at the old size first. If that gives what the layer shows now (nearly every texel within a few levels), the layer is painted again from them at the new size, crisp at any size. Otherwise the layer is resized as a picture. That covers:
  - layers with imported pictures;
  - layers whose paint was moved with its polygons;
  - merged layers;
  - layers with strokes that depend on more than is kept: blur, blend, smudge, hue shift, projection and clone.
- **How strokes are painted again:**
  - A brush dab on the model is painted seen straight on from where the camera was (its kept way towards the camera), at its kept place, size and strength.
  - A dab painted in the UV Editor goes at its kept place on the texture.
  - Polygon fills fill their polygon, UV piece or material again.
  - Paint and erase strokes are painted again; other kinds make the layer a picture.
- **Only one object per set.** Dabs on the model are kept in their object's own space, but not which object. When several objects share a material, its layers are resized as pictures.
- **As a picture:** made smaller, each pixel averages the old pixels it covers; made bigger, the paint is read smoothly. Both are weighted by coverage, so edges don't darken.
- **Strokes are kept**, so the next size change can paint them again too.
- **Textures bigger than the graphics card takes** (its largest image, often 8192 or 16384 pixels; 2048 on some older cards) no longer stop the app:
  - The views get the largest copy that fits; the full-size paint is kept for painting and export.
  - The UV Editor shows a copy made smaller in the same way.

## Consequences

- Block in colours on a small texture, then raise the size for detail, and brush and fill paint stays crisp.
- Projection, clone and effect strokes, and pictures, still soften when enlarged. A later milestone could keep what they need (the stroke's angle, the projected picture) to paint them again too.
- Painting again takes longer than resizing a picture: a set with thousands of strokes may take a moment to change size.
