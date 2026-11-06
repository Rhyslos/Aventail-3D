# Baking maps

> Bake a high-detail model's bumps and bevels into a normal map for the low-detail one (as Godot reads it), plus ambient occlusion, curvature, thickness, colour ID, position and height maps, as picture files.

## When to use it

A game draws the light, [low-poly](concepts.md#mesh-basics) model. The detail lives in textures: a **normal map** makes flat polygons light as if they had the high-detail model's bevels, screws and dents ([Concepts: normal map, baking](concepts.md#the-game-asset-pipeline)). Baking is how those textures are made: the app looks at the high-detail model from the low one's surface and writes what it finds into the low one's UVs.

The other maps help painting: **ambient occlusion** darkens creases, **curvature** finds edges for wear, **colour ID** gives each material a flat colour to select by, **thickness**, **position** and **height** feed masks and gradients. They're made from the low model itself.

Substance Painter can bake too; baking here is quicker for a kit and keeps everything in one place. Before baking, check the UVs with [Check for baking](bake-check.md) and **Display → Bake seams**.

## How to use it

1. Make the low-detail model and unwrap it (every bake needs UVs that don't overlap).
2. Make or bring in the high-detail model and place it over the low one, where it should be.
3. Select the **low** model and choose **File → Bake…**.
4. In **Bake from**, pick the high-detail object (or leave **The object itself** for the maps that need no high model).
5. Tick the maps you want.
6. Click **Bake into a folder…** and choose a folder. The maps are baked one by one ("Baking map 2 of 4… 35%"); **Cancel** stops.
7. The folder gets one PNG per map, named after the object: `crate_normal.png`, `crate_occlusion.png` and so on, at the project's texture size.

## Settings

| Setting | What it does |
|---|---|
| **Onto** | The object the maps are for: the first selected object. |
| **Bake from** | The high-detail object the normal and height maps take their detail from. **The object itself**: a flat normal map and a middle-grey height map. |
| **Normal** | Tangent-space normal map, worked out with the same MikkTSpace tangents Godot uses, so it lights the same there ([Concepts: tangent space](concepts.md#the-game-asset-pipeline)). |
| **Ambient occlusion**, **Curvature**, **Thickness**, **Colour ID**, **Position** | From the low model, as the painter's bakes are ([Painting textures](painting-textures.md)). |
| **Height** | How far above (lighter) or below (darker) the low model the high one lies, mid grey on it, within the cage. |
| **Cage** | How far out from the low model the bake looks for the high one, and as far in, as a share of the object's size: 2% standard. Raise it when parts of the high model stick out further; lower it when the bake picks up the wrong part ([Concepts: cage](concepts.md#the-game-asset-pipeline)). |
| **Normal maps green down (DirectX)** | Off (standard): green up, as Godot and Blender read normal maps. On: green down, for Unreal. |

The texture size is the project's (UV Editor → **Texture** menu). Each map's pieces are spread outward by Pack's padding, so a game's smaller copies of the texture don't pull in the empty space.

## Shortcuts

| Action | Shortcut |
|---|---|
| The baker | File → **Bake…** |

No key; ask if you'd like one.

## Tips and common problems

- **Problem:** the normal map has thin lines along some seams. **Why:** a seam runs along a smoothly shaded edge. **Fix:** turn on **Display → Bake seams** and put seams on hard edges, or make those edges hard.
- **Problem:** parts of the normal map are flat where the high model has detail. **Why:** the cage doesn't reach it. **Fix:** raise **Cage**.
- **Problem:** detail from a neighbouring part shows up in the wrong place. **Why:** the cage reaches too far. **Fix:** lower **Cage**.
- **Tip:** the bake doesn't change the scene, so there's nothing to undo; bake again as often as you like.

## Related

- [Check for baking](bake-check.md), [How shapes are drawn: Bake seams](viewport-display.md)
- [Painting textures](painting-textures.md)
- [Concepts: baking, normal map, tangent space, cage](concepts.md#the-game-asset-pipeline)
