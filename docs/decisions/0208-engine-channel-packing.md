# 0208. Engine channel packing

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's twenty-ninth milestone ([0177](0177-texture-painter-plan.md)): "Godot's ORM (AO, roughness, metal in one image), Unity's mask maps and Unreal's ORM; normal maps the way each engine wants them." Realism channels ([0207](0207-realism-channels.md)) are painted, but exports carried only colour. Each engine wants roughness and metal packed its own way.

## Options

- **One layout for every engine:** simplest, but Unity's HDRP reads a different packing.
- **glTF's own layout inside, chosen extras beside (chosen):** glTF already says where roughness and metal go (an image's green and blue, occlusion in red: the "ORM"), and Godot, Unity (glTFast), Unreal, Blender and Solis read it. Files beside the model cover engines and tools that want the maps on their own.

## Decision

- **Inside a glTF**, whenever a texture set uses the realism channels:
  - an ORM image as the material's `metallicRoughnessTexture` (factors 1);
  - its glow, if any layer glows, as `emissiveTexture` (factor 1).
- **Beside the file**, Export → **Realism maps** (shown while a material going out has realism channels), remembered:
  - **In the file** (standard): nothing more for glTF; an ORM beside an OBJ;
  - **ORM beside it** (Godot, Unreal Engine);
  - **Unity mask map beside it** (red metal, green occlusion, blue detail mask 0, alpha smoothness = 1 − roughness), chosen by the Unity preset;
  - **Separate maps beside it** (roughness and metal as greys).
- **OBJ's `.mtl`** names them by the format's PBR additions: `map_Pr` and `map_Pm` (from the ORM's green and blue with `-imfchan`, or the separate files), `map_Ke` and `Ke 1 1 1` for glow.
- **Occlusion** is white in the ORM and the mask map for now (it darkens nothing). Engines add their own ambient occlusion, and a baked one can be painted in with Bakes → Occlusion → Add as a layer. Baking it into the ORM on export is left for later; it needs the baker on the export thread.
- **Normal maps** (green up for Godot, Unity and Blender; green down for Unreal) come with height and normal painting (milestone 32), when there is a normal map to export.
- Every map is padded like the colour and listed in the texture budget by kind (colour, ORM, glow, mask map, roughness, metal).
- **No keys:** one row in the Export window.

## Consequences

- A realistic material reaches Godot, Unity, Unreal, Blender and Solis looking as painted, without setting up its maps by hand.
- More files beside an export when asked; the texture budget counts them.

## Amended 2026-10-30, room and smaller fixes

Separate maps now sends every map beside the model: the colour (also beside a glTF, which holds its own), roughness, metal, the height as grey (`TextureMap::Height`, named as `bump` in an OBJ's .mtl) and the glow when painted.
