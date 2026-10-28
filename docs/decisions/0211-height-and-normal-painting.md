# 0211. Height and normal painting

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's thirty-second milestone ([0177](0177-texture-painter-plan.md)): "paint height to get bumps, and mix normal maps correctly." Rivets, welds, cracks and grooves are too small to model on a game asset; games fake them with a normal map. Painters think in height (raised or sunk), not in normal map colours, and mixing normal maps by averaging their colours gives wrong, flattened bumps.

## Options

- **Paint normal map colours directly:** hard to think in, and layers of normals don't mix by blending.
- **Paint height, make the normal map from it (chosen):** height is easy to paint (light raised, dark sunk), heights mix by the usual layer rules, and one normal map is made from the mixed height, so overlapping bumps combine correctly.

## Decision

- **A fourth realism channel, Height** (`Channel::Height`), grey: 50% level, lighter raised, darker sunk. Like roughness, metal and glow ([0207](0207-realism-channels.md)) it is per layer, painted or a fill value, and mixed by layer order, opacity, masks and clipping (Normal blend).
  - The brush and fill tool get a **Height** row (off as standard; 75% when ticked).
  - Fill layers get the same row.
- **Depth:** the channel's full range is 0.5 cm of surface (`paint::channel::HEIGHT_DEPTH`), so details stay small, as normal maps suit.
- **Seeing it:** the views bump the surface from the height (bump mapping from how fast it changes across each pixel, needing no tangents), in every paint preview, so bumps catch the light while you paint.
- **Exporting:**
  - a normal map made from the mixed height (tangent space; red right, green up, blue out; each texel's slope from its neighbours and the texel density);
  - inside a glTF as the material's `normalTexture`, always green up as glTF says;
  - beside an OBJ (named by `norm` in its `.mtl`), and beside a glTF when Realism maps asks for files beside it ([0208](0208-engine-channel-packing.md)).
  - Export → **Normal maps: Green up (OpenGL) | Green down (DirectX)** sets which way green points in the files beside it. Godot, Unity and Blender read green up; Unreal Engine reads green down, and its preset chooses green down (and an ORM beside the file).
- **Saving:** project format 81 stores each layer's height part.
- **Not done:** importing a picture as a normal map to mix in; heights mixing covers painted detail, and an imported normal map can come later.
- **No keys.**

## Consequences

- Small surface detail is painted, seen lit, and reaches every engine as the normal map it expects.
- Material layers ([0210](0210-material-layers.md)) don't make height yet; a stone or wood bump can come with them later.
