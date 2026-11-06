# 0247. Player-view UVs

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 21. The automatic tools hide seams by a fixed rule (underneath and at the back, as seen from the Front view) and give every piece the same texel density. A game asset is seen from somewhere particular: a crate in a third-person game is seen from above and round the sides at a few metres, a wall in a strategy game from high above. Seams belong where that player doesn't look, and texture where the player looks closest.

## Decision

- **The player's cameras** (`bake::player`): eight cameras round the asset's middle, as a third-person game (eye height 1.7 m, 3 m standard), a first-person one (1.6 m, 1.5 m) or a top-down one (50° from straight down, 12 m) places them, at a distance of your own. For each polygon, `player_view` gives the share of cameras that see it (its middle faces the camera and nothing is in the way, with the baker's ray box tree, now also made from plain triangles: `Rays::of_triangles`) and the screen pixels a metre of it covers from the nearest one (1080 pixels high, 60° field of view). **Worked out on the CPU with rays**, not with GPU picture passes as the plan said: the baker's rays were already there, and the result is the same on every computer and testable without a GPU.
- **UV → Settings → Player view**: the kind (Third person standard), the distance, and **Pack gives more texture where the player looks closest**. Remembered with the UV settings, so recipes (0246) keep them.
- **Seams:** Seam suggestions' **Where the player doesn't look** makes Suggest seams weigh each edge by how much the player sees the polygons beside it (`uv::seams::with_player_view` puts the view on a copy of the shape as the `player seen` layer, never kept in a model). Unwrap's own cuts, made again on every change of an edit without the cameras, keep the standard rule (underneath and at the back), so nothing saved changes and no new file format is needed.
- **Pack:** with the switch on, each piece gets the square root of its polygons' average screen pixels a metre over the object's, from half to twice as much texture across, on top of its own texture scale (0240).
- **Texels per screen pixel:** the UV Editor's **View → Texels per screen pixel** colours each polygon by texture pixels a screen pixel from the nearest camera: blue under half (blurry), green about one, red over two (texture wasted), grey never seen. **In the UV Editor**, not as a surface in the 3D view as the plan said: the surfaces have no per-polygon value to colour by, and the UV Editor is where the fix (scaling pieces) is made. A surface view can follow.
- What is seen is worked out again only when the shape or the view changes.
- No keys.

## Consequences

- An asset's own placement (its turn and scale in the scene) isn't used: the cameras look at its shape as built, Z up.
- A polygon is seen or not by its middle; a big polygon half hidden counts by its middle alone.
