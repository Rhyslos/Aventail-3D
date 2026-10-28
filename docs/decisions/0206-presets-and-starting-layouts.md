# 0206. Presets and starting layouts

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's twenty-seventh milestone ([0177](0177-texture-painter-plan.md)): "style, brush and material presets as files; a Hand-painted and a Realistic starting layout." A painter builds the same stack of layers again and again (a base, light and shade, edge highlights, dirt), and wants their brushes and looks on another project or another computer. Substance Painter has smart materials (a folder of layers dropped onto a texture set) and shelves of presets for this.

## Options

- **Presets as project files** (an `.avtl` holding only the layers): no new format, but the file is big, binary and needs the app's version to match.
- **Presets in the app's settings only:** quick, but they can't be shared or moved.
- **A small preset file (chosen):** readable JSON, one kind per file, versioned.

## Decision

- **Preset files** (`fileio::preset`, extension `.paintpreset`, readable JSON with a `format` number; newer formats refused with a message; at most 4 MB). One file holds one of:
  - **brushes:** your own brushes from the brush bar;
  - **a style:** one paint layer or group, as a recipe;
  - **a material:** a material's colour and all its layers, as a recipe.
- **Recipes, not pixels.** Layers keep their names, blend modes, opacity, clipping, fill colours, gradients, light and shade, and masks (smart masks with their settings); painted pixels and the strokes that made them are left out, because they belong to one model's UVs. A preset is made at the project's texture size when it is loaded, and its gradients, light and shade and smart masks are made on the new shapes.
- **Where:**
  - the Paint layers panel's **Presets** row: **Save style…** (the chosen layer or group), **Save material…** (the material's colour and layers) and **Load preset…**;
  - the brush bar: **Save brushes…** and **Load brushes…**.
- **Loading** does what the file holds, whichever button opened it:
  - a style is added above the chosen layer, one undo step;
  - a material replaces the shown material's colour (on every object that has it) and its layers, one undo step;
  - brushes join your own (doubles and brushes out of range left out, up to 200).
- **Starting layouts:** while a material has no layers, the panel offers **Start with a layout: Hand-painted | Realistic**, one undo step, choosing the top layer to paint on:
  - **Hand-painted:** Base colour (the material's colour), Light and shade, Edge highlight, Paint;
  - **Realistic:** Base colour, Cavity dirt (Multiply, a cavity smart mask), Edge wear (the base lightened, a thin, broken edge mask), Dust (a facing-up mask), Paint. Realism channels (roughness, metal) join it with milestone 28.
- **No keys or mouse actions:** all buttons. File windows as elsewhere, with a typed path when the system's window can't open.
- The extension doesn't name the app, so it survives a rename.

## Consequences

- Looks and brushes move between projects and people as small text files that can be read and checked.
- Painted detail doesn't travel in presets; projection or importing an image is the way to bring pictures.
- Built-in style presets beyond the two layouts can be added later as files shipped with the app.
