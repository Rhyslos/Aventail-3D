# 0217. Brush alignment and size space

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

The project lead asked for two of Substance Painter's brush settings, which he relies on: how a dab is laid on the surface (Painter's "Alignment") and in which space its size is measured (Painter's "Size space"). The brush had one way to lay dabs, seen from the camera, and two ways to measure its size, on screen or on the surface.

Painter (its manual, 2026-10-29) offers four alignments: Camera (the stamp faces the view), Tangent | Wrap (aligned with the surface and bent to it; its default), Tangent | Planar (aligned with the surface, fading where it leaves it) and UV (laid by the UVs). Its size spaces are Object (the default: synced with the mesh), Viewport (linked to the screen) and Texture (linked to the texture).

## Decision

- **Alignment**, a Paint tab row (a list), kept on each brush and on each stroke:
  - **Camera** (standard, the way the brush always worked): the dab is round on screen; it stretches on surfaces turned from the camera. Its reach now follows that stretch (up to five times, where turned surfaces stop taking paint), so a camera dab on a turned surface is no longer cut to a circle.
  - **Wrap**: round on the surface, measured from the dab's middle by the straight distance (which follows a curve closely within a dab), along a frame lying on the surface's facing there (the facing of the triangles near the middle, larger and nearer counting more), turned to the screen; it bends round edges and curves without stretching.
  - **Planar**: the same frame, offsets measured on its flat; texels further off the flat than a quarter of the radius fade, gone by three quarters, so it doesn't run round edges.
  - **UV**: round in texels round the middle's texel (from the triangle it lands on), its radius the world radius at that triangle's texel density: it follows the UVs' stretch and stops at seams.
  - Camera stays the standard, so painting is unchanged until a person picks another; Painter's standard is Wrap.
- **Size space**, the Paint tab row that replaces Measured: **Viewport** (points on screen, the standard), **Object** (centimetres on the surface) and **Texture** (texels, from the texel density of the triangle under the brush: the same number of texture pixels wherever it lands). The brush bar's size field follows (px, cm or texels); [ ] and Ctrl + drag change the size in its own space. Vertex colours have no texture: measured on the texture they measure as on screen.
- Names: Camera, Wrap, Planar and UV; Viewport, Object and Texture (plain words, without Painter's "Tangent |").
- Saved brushes and brush preset files keep an alignment (none keeps the one in use, for the built-in brushes and older files). Each stroke keeps its alignment so it can be painted again (texture size, paint following the model): file format 83; older strokes are Camera.
- No keys or mouse actions.

## Consequences

- Texture painters coming from Painter find the two settings they expect.
- A stroke laid on the surface or the UVs is painted again the same way.
