# 0074. Material slots

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** [0071](0071-obj-export-and-import.md) and [0072](0072-gltf-export.md) (OBJ and glTF export)

## Problem

A game asset is usually split into parts that get different materials: painted metal, rubber, glass. In Substance Painter each material becomes its own **texture set**, and in Godot each its own surface with its own material. The modeller has to say which polygons belong to which material, and the export has to carry it.

## Options

- **Where the slot lives:**
  - a separate list of polygons per material on the object, which every edit would have to keep up to date;
  - **a number on each polygon** (a polygon attribute, `material`), which the mesh's attribute system already carries through every edit (chosen). An extrude's walls take the number of the polygon they grew from.
- **Where the materials live:**
  - one list for the whole project;
  - **a list on each object**, the numbers pointing into it (chosen). Materials with the same name are one material in an export, across objects, so a "Metal" on two objects is one texture set.
- **How a polygon gets one (asked, the project lead chose):**
  - **a Materials list in the Attributes panel with an Assign button**, and **Select → Select by material** (chosen); no new keys;
  - a key that cycles materials.
- **Colours:** the colour you pick is stored as seen (sRGB), and converted where a format wants linear light (glTF).

## Decision

1. **Mesh:** each polygon has a slot number in the `material` polygon layer, 0 unless set. At most 64 slots.
2. **Objects:** each has a list of materials (name and colour). Empty means the one standard material, "Default", in the viewport's grey. An instance uses its original's list; made editable, it keeps a copy.
3. **Edits:**
   - `Command::SetMaterials` changes an object's list (names trimmed, never empty, colours 0 to 1), one undo step.
   - `Operation::SetMaterial` gives polygons a slot. It is a recorded edit like any other, so it follows its polygons by their anchors, is undone like any other, and is made on both sides with Symmetry on.
   - **Join** merges the lists by name and renumbers the joined parts' polygons.
4. **The panel:** the Attributes panel's **Materials** section lists the materials with a colour swatch, a name, how many polygons use each, and **Assign** (in the Polygons mode, with polygons selected). **Add material** and **Remove last** (refused while a polygon uses it) change the list.
5. **The viewport** draws each polygon in its material's colour: every drawn object carries a palette of 64 packed colours, and each vertex its slot. Where two materials meet on a soft edge, the vertex is doubled, one for each material, with the same shading.
6. **Exports:**
   - **OBJ** writes `usemtl` lines and a `.mtl` file beside it (`mtllib`), with each material's colour (`Kd`). Names become one word (spaces to underscores), as most programs read only the first.
   - **glTF** writes one material per name, used by any object, with its colour (`baseColorFactor`, converted to linear), and one primitive per material in each mesh.
   - Only materials some polygon uses go out.
7. **Import:** each `usemtl` name an OBJ object uses becomes one of its materials, in the order it first uses them, coloured from the `.mtl` file when it is found beside the OBJ (by file name only, so a file can't point elsewhere).
8. **File format 18:** the objects' lists, and the new edit on its own or mirrored. Older files open with no materials.

## Consequences

- Substance Painter makes one texture set per material, and Godot one surface per material, from the moment a model has them.
- Real materials (textures, roughness, metal) arrive with texturing; until then a material is a name and a colour, which is what the engines and Substance Painter need to tell the parts apart.
- The viewport's grey is now the standard material's colour, so a project that never uses materials looks exactly as before.
