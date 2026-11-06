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

The Bake window's settings (the maps ticked, what to bake from, **Matched by name**, the cage, the Godot material and the rest) are saved with the project, so they're there again when you or anyone else opens it. Changing them isn't a step to undo (Ctrl + Z undoes changes to the models, not the window), but it does count as a change to save. **Preview on the model** always starts off. Projects saved before this version open with the standard settings.

## A whole kit: _low and _high

Name each low piece `something_low` and its high-detail model `something_high`. Small details modelled separately and resting on the surface (bolts, panels, rivets) can be called `something_high_bolts` and so on: they're **floating details**, and the bake projects them onto the surface beneath.

1. Select every `_low` piece.
2. In **File → Bake…**, tick **Matched by name (_low and _high)**.
3. Click **Bake into a folder…**. Each low piece is baked from its own high ones, on its own, so a neighbouring piece's detail never lands on it (other programs make you move the pairs apart, *explode* them, for that). Each gets its own files. Two pieces whose names would make the same file name (`crate low` and `crate_low`, or two objects both called `crate_low`) don't write over each other: the second gets `_2` added (`crate_low_2_normal.png`).

## A whole kit, vertex colours and Godot

- **Several pieces at once:** select every piece of the kit and bake: each is baked on its own, into its own files (with `_2` added when two names would make the same file). The message afterwards counts the maps baked; a Godot material is mentioned on its own.
- **Leave out stacked and mirrored copies** (on as standard): pieces laid on top of each other in the texture (copies sharing one piece of texture, mirrored halves) would bake over each other; the copies are left out, so the piece they share bakes once, cleanly. Only pieces whose polygons really lie over another's count: a small piece packed into the empty corner of an L-shaped piece, or inside a ring, is baked as usual. The copies left out still cast their shadows: a copy sitting on the piece baked still darkens it in the ambient occlusion, as it will in the game.
- **Bake into vertex colours:** the first ticked map that comes from the model itself (occlusion, curvature, thickness, colour ID or position) goes into the selected objects' [vertex colours](painting-textures.md#vertex-colours) instead of a picture, one undo step: for games that colour models by their corners, with no texture. Colour ID gives each polygon its own material's colour at every corner, so the colours meet in a clean line where two materials do, rather than one bleeding into the other.
- **A Godot material (normal map and ORM):** also writes `name_orm.png` (ambient occlusion, roughness and metal packed into one picture, as Godot's ORM materials read it; set **Roughness** and **Metal** for the whole object) and `name.tres`, a Godot material using it and the normal map, with its ambient occlusion turned on (Godot ignores an ORM's occlusion otherwise). Bake into a folder inside your Godot project, then drag the `.tres` onto the model's material in Godot. Leave **Normal maps green down** off for Godot: the window warns you if it's on, because Godot reads normal maps green up and the bumps would come out the wrong way.

## The cage

The **cage** is the low model pushed out a little: the bake looks from it back in for the high model ([Concepts: cage](concepts.md#the-game-asset-pipeline)). While the Bake window is open, it's drawn over the selected object in see-through cyan (**Show the cage**).

Where the high model sticks out further than the cage, the bake misses it. Raise **Cage** for the whole object, or paint it out just there:

1. Click **Paint the cage** in the Bake window.
2. Drag over the points where the cage should reach further: each stroke pushes them out a step (half the cage's push). **Ctrl + drag** pulls them back in. **[** and **]** change the brush's size.
3. Press **Esc** (or the button again) to stop. Each stroke is one undo step; **Clear** clears what you painted. The painted cage is saved with the project.

The bake reaches as far as the cage drawn, even on an object you've scaled: the painted push grows and shrinks with the object.

## Seeing it before you bake

Tick **Preview on the model** in the Bake window and choose what to see: any map, or **Problems**. A small, quick bake is shown on the model (and behind the UVs in the UV Editor), and it's made again whenever you change the cage, paint it, move or edit the models. Unticking it, or closing the window, puts the model's look back. So does selecting nothing: the preview goes until you select an object again, so it isn't mistaken for the model's own look.

**Problems** (with a high-detail object: with **The object itself** there's nothing to miss, and the window says so) colours the model where the bake goes wrong, and the window says how much and what to do:

- **red**, *found nothing*: the high model lies further out or in than the cage reaches. Raise **Cage**, or paint the cage out there.
- **purple**, *probably another part*: the cage reaches so far it finds a part of the high model behind the one meant. Lower **Cage**, or paint the cage in there.
- **yellow**, *smeared*: the high surface is at a steep slant to the low one, so its detail smears. Add an edge loop or a bevel to the low model there so it follows the high one, or make that edge hard and a seam.
- **grey**: all well.

## Settings

| Setting | What it does |
|---|---|
| **Matched by name (_low and _high)** | Off (standard). On: each selected `_low` object is baked from the objects named like it with `_high`, instead of **Bake from**. |
| **Preview on the model** | Off (standard). On: a quick bake (at most 256 pixels) of the map chosen beside it, or the **Problems**, shown on the model and made again on every change. |
| **Leave out stacked and mirrored copies** | On (standard): pieces lying over others in the texture are left out of the bake. |
| **A Godot material (normal map and ORM)** | Off (standard). On: an ORM texture and a Godot material file too. **Roughness** (0.8 standard) and **Metal** fill the ORM's green and blue. |
| **Bake into vertex colours** | Bakes the first ticked map that comes from the model itself into vertex colours instead. |
| **Show the cage** | On (standard): the cage drawn on the model while the window is open. |
| **Paint the cage** / **Clear** | Paint the cage out (or in with Ctrl) where the bake should reach further; clear what you painted. |
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
| Paint the cage out / in | Drag / **Ctrl** + drag (with **Paint the cage** on) |
| Brush size | **[** and **]** |
| Stop painting the cage | **Esc** |

No key to open it; ask if you'd like one.

## Tips and common problems

- **Problem:** the normal map has thin lines along some seams. **Why:** a seam runs along a smoothly shaded edge. **Fix:** turn on **Display → Bake seams** and put seams on hard edges, or make those edges hard.
- **Problem:** parts of the normal map are flat where the high model has detail. **Why:** the cage doesn't reach it. **Fix:** raise **Cage**, or **Paint the cage** out just there.
- **Problem:** with Matched by name, a piece bakes flat. **Why:** no object is named like it with `_high`. **Fix:** check the names: `door_low` needs `door_high` (any capitals).
- **Problem:** detail from a neighbouring part shows up in the wrong place. **Why:** the cage reaches too far. **Fix:** lower **Cage**.
- **Tip:** the preview is small, so it can miss tiny problems; the files the full bake writes are the final word.
- **Problem:** in Godot the material shows no textures. **Why:** the folder wasn't inside the Godot project, so the material names the pictures by their file names alone. **Fix:** bake into a folder inside the project (the one with `project.godot`).
- **Tip:** the bake doesn't change the scene (except Bake into vertex colours, which Ctrl + Z takes back), so there's nothing to undo; bake again as often as you like.

## Related

- [Check for baking](bake-check.md), [How shapes are drawn: Bake seams](viewport-display.md)
- [Painting textures](painting-textures.md)
- [Concepts: baking, normal map, tangent space, cage](concepts.md#the-game-asset-pipeline)
