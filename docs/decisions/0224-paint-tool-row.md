# 0224. The Paint page's tool row

- **Status:** Accepted
- **Date:** 2026-10-29

## Problem

The mockup's Paint page ([0223](0223-paint-page-like-the-mockup.md)) has one row along the bottom of the view: the view's controls (☰, 3D, Lit, Show), then select, move, rotate and scale, then the paint tools, among them three the app had no button for: blur, a paint bucket and a colour picker. The view's menu line, the view dice, and the shading and navigation buttons go.

## Decision

- **View controls**, along the bottom left of the view: **☰** (the view's menus, View, Cameras, Display, Options and Panel, and the navigation picker's list), **3D** (the camera: it says 3D for the perspective view, otherwise its name), **Lit** (shaded or wireframe, the paint preview, the player view; it says the preview in use) and **Show** (Edges, Grid, Cavity, hard edges, seams, normals, collision, triangles).
- **Tools**, right after them (above them when the row would run over the UV Editor's notes or past the painting area): **Select (Q)**, **Move (W)**, **Rotate (R)**, **Scale (S)**, a line, then **Paint brush (1)**, **Eraser (2)**, **Projection (3)**, **Polygon fill (4)**, **Smudge (5)**, **Blur (7)**, **Clone (6)**, **Fill the layer (8)** and **Pick a colour (P)**. The project lead's choices:
  - **7 is blur, 8 the bucket, P the picker** (P keeps picking under the mouse at once; the button makes the next click pick).
  - **The bucket fills the layer:** one click fills every polygon of the material clicked, whatever Polygon fill is set to fill.
  - **Move, rotate and scale put the brush down:** a drag moves the selected objects with the gizmo; the select arrow makes clicks select. A paint tool's button or key (1 to 8) picks the brush up again.
- The left bar's tools (when Settings puts them there) get the same buttons.

## Consequences

- No new mouse actions; two new keys (7, 8), both only while painting.
- The bucket's fills are kept with the strokes as Material fills, so they're painted again when the texture size or the model changes, as other fills are (no new file format).
