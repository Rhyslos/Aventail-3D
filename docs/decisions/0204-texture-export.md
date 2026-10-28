# 0204. Texture export

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's twenty-fifth milestone ([0177](0177-texture-painter-plan.md)), the first of export and sharing: "PNG, TGA, padding, linked into glTF and OBJ, presets, export on save, budget report". Painted textures lived only in the project; an exported model went out with plain material colours.

## Decision

- **Each painted material's texture goes out with the model** (`fileio::export::textures`): its layers mixed (as the viewport shows them) and laid over the material's colour, so it is opaque and unpainted parts keep the material's colour.
  - **glTF:** inside the file as PNG (the format glTF wants), each material's base colour texture, its colour factor white; a sampler that's smooth, with smaller copies, and repeating (for trim sheets).
  - **OBJ:** beside it as `name_Material.png` or `.tga`, named by `map_Kd` in the `.mtl`, the material's colour white.
- **Padding:** each texture's colours are spread past the edges of its UV pieces, ring by ring, 8 texels as standard (0 to 64), so a game's smaller copies of the texture don't pull in the colour around a piece. Coverage counts UVs past the square where the texture repeats ([0202](0202-paint-trim-sheets-live.md)).
- **The Export window** gets a **Textures** row (PNG, standard; TGA; Leave out) with **Padding**, remembered with the other export settings, kept when Made for changes. Every preset takes them; Godot, Unity, Unreal Engine and Solis get them inside the glTF.
- **Export on save** and kit pieces carry them too (the same export).
- **Texture budget:** the Export window lists each texture going out (material, size, channels) and what it takes on the graphics card, uncompressed with its smaller copies (4 bytes a pixel, a third more), and the total. The export's files record each texture's file size too (`ExportFiles::textures`).

## Consequences

- A painted model reaches Godot looking as it does in the viewport, with no extra steps.
- Only base colour goes out for now; the realism channels (roughness, metal, normal) and engine channel packing come with milestones 28 and 29.
- Warnings about texture memory while working, and a compressed-format estimate, are left for later.
