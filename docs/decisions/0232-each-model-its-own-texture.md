# 0232. Each model its own texture

- **Status:** Accepted
- **Date:** 2026-10-30

## Problem

A texture belongs to a material (decision 0177), and every new shape starts with the standard material, Default, with UVs filling the whole square. Two new shapes painted one after the other therefore share one texture with their UVs on top of each other: paint on one shows on the other. Decision 0214 warned about it and offered two fixes, but the project lead asked for each model to be its own texture: "it doesn't make sense to me that models share UV space".

## Options

1. **Give every new shape its own material when it is made.** Simple to explain, but every shape then has its own material in the Materials list and its own texture in an export, painted or not, and every existing project and test assumes Default.
2. **Give a shape its own copy of Default when it is painted while another shape has Default too.** Only painted shapes get their own; a lone shape keeps Default; deliberate sharing (a material made by hand) is untouched.
3. Keep the warning only (decision 0214).

## Decision

Option 2.

- **When:** the first change to the paint of a shape's Default material while another shape (not an instance of it) also has Default: a stroke or fill in the 3D view or the UV Editor, any change in the Paint layers panel (adding or changing layers, starting layouts, the shelf's cards), loading a style or material preset, putting a bake into a layer, importing a picture as a layer. For the panel and presets, "the shape" is the one selected shape; with several selected, nothing is copied.
- **What:** the shape's slots with Default get a copy of the material named after the shape (numbered if the name is taken), with a copy of Default's texture set if it has one (`paint_layers::own_texture`). One undo step, "Give *shape* its own texture", before the change itself; the status bar says what happened. The panel then shows the new material.
- **Not:** instances (they show their original's materials), shapes alone on Default, and any other material: one made by hand and given to several shapes is shared on purpose (a trim sheet, a kit).
- **The warning (decision 0214)** is no longer shown for Default, which painting never shares now; it stays for other materials, with its two fixes. Its own-texture fix and this share one function.

## Consequences

- Each painted model has its own texture without the project lead doing anything; Ctrl + Z separates the copy from the painting.
- An export has one texture set per painted shape, which costs more texture memory than packing shapes onto one; packing stays possible by giving them one material of their own and choosing Pack.
- Painting the first of two new shapes leaves the second on Default; it keeps that name, the first is renamed after itself.

## Amended 2026-10-30, new shapes start clean

The project lead painted a lone cube (on Default, as it was alone), deleted it and made a sphere: the sphere showed the cube's paint, as Default still held it. Now `Command::Create` gives a new shape (a primitive or mesh) a material of its own, named after it (`Scene::unused_material_name`: not used by any object or texture set, numbered if taken), whenever Default holds a texture set. With Default unpainted, new shapes keep Default as before. It is part of the same undo step as making the shape. Copies and instances are unchanged: a copy of a painted shape looks painted.
