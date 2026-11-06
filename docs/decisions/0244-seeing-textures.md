# 0244. Seeing textures: a picture of your own, the layout export, two stretch views and a mip preview

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 17. The checker shows whether UVs are even, but not how a real texture made elsewhere will sit on them; there was no way to take the layout out to paint over in another program; the Stretch view mixes two different problems (pieces stretched one way, and pieces with too much or too little texture); and nothing showed how a texture looks far away in a game, where small details blur together.

## Decision

- **The texture picture** (UV → Texture picture → Choose…): any image the app can read (the paint layers' reader, `fileio::image_file`), resampled square to a power of two at most 2048 across (`ui::viewport::Picture`). It is shown on the model as a new **Texture picture** surface in Display → Surface, drawn through the numbered checker's image slot (no new GPU binding), and behind the UVs in the UV Editor. With no picture chosen the surface shows the numbered checker. **Kept for the session, not saved in the project**: the picture is a check, not part of the model; saving its path in the project can follow if wanted.
- **Mip level** (0 to 6, in the same menu, `ViewOptions::mip_level`, remembered): the picture halved that many times, every pixel the average of four, as a game's mipmaps do. The UV Editor shows it with nearest pixels so the blocks show.
- **Layout export** (UV → Texture picture → Export UV layout as PNG… or SVG…): the selected objects' UV edges at the project's texture size, white antialiased lines on a clear background (PNG) or SVG lines, line width a pixel per 1024 (`fileio::uv_layout`). V runs down, as images do, so the export lies exactly over the texture.
- **Two more stretch views**, Display → Surface → **Stretch: angles** and **Stretch: size**: the same shader as Stretch, told which part to count (`SurfaceDraw::Stretch { part }`, 0 both, 1 the shape only, 2 the texel density only).
- No keys or mouse actions; everything is in the menus.

## Consequences

- One picture for the whole scene, not one per material; textures per material come with painting (the Painted surface).
- The picture's pixels are resampled once when chosen; a non-square picture is stretched over the square, as an engine stretches it over the UVs.
