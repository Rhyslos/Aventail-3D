# Player view

> Tell the app how the player sees your assets (third person, first person or top-down), so seams go where the player doesn't look, texture goes where they look closest, and you can see where the texture will look blurry.

## When to use it

The standard rule for hiding seams is "underneath and at the back", which suits a prop seen from the front. But a game asset is seen from somewhere particular. In a third-person game the player sees a crate from above and all round at a few metres, never from below. In a strategy game they see the tops of buildings from high up. In first person they see everything at eye height up close.

**Player view** places a few cameras round the asset the way your game would and works out, for every polygon, how much the player sees it and from how close. Three tools use it:

- **Seams:** cuts go where the player never or rarely looks.
- **Packing:** pieces the player sees up close get more texture, pieces seen only from far off (or never) get less.
- **Texels per screen pixel** in the UV Editor: see where the texture will look blurry (blue), about right (green) or wasted (red). See [Concepts: texels per screen pixel](concepts.md#uvs-and-texturing).

## How to use it

1. Choose **UV → Settings → Player view**, and pick **Third person**, **First person** or **Top-down**. Set the **Distance** the player's camera usually is from the asset, in metres.
2. For seams: in **UV → Settings → Seam suggestions**, choose **Where the player doesn't look**. Then select the objects and use **UV → Seams → Suggest seams** (or [Auto-unwrap](auto-unwrap.md)).
3. For texture: tick **Pack gives more texture where the player looks closest** in the same Player view menu, then **Pack**.
4. To check: open the UV Editor and tick **View → Texels per screen pixel**. Each polygon is coloured:
   - **green**: about one texture pixel per screen pixel, about right;
   - **blue**: the texture will look blurry up close; give the piece more room (or the project a bigger texture);
   - **red**: more texture than the player can see; the piece can be made smaller to make room for others;
   - **grey**: the player never sees it.

## Settings

| Setting | What it does |
|---|---|
| **Third person** (standard) | The camera follows a character at eye height, 3 m off as standard: the tops and sides are seen, the bottom never. |
| **First person** | Through the character's eyes, 1.5 m off as standard: everything at eye height is seen sharpest. |
| **Top-down** | From high above at a slant, 12 m off as standard: the tops are seen most. |
| **Distance** | How far the player's camera is from the asset's middle, in metres. Choosing a kind sets its standard distance. |
| **Pack gives more texture where the player looks closest** | Off (standard). On: pieces seen up close get up to twice as much texture across, pieces seen from far off or never down to half. |
| **Where the player doesn't look** (Seam suggestions) | Suggest seams puts its cuts where the cameras see least. Unwrap's own cuts still go underneath and at the back. |

All remembered, and kept in [recipes](auto-unwrap.md#recipes-the-same-unwrap-for-a-whole-kit).

## Shortcuts

| Action | Shortcut |
|---|---|
| Player view settings | UV menu → Settings → **Player view** |
| Texels per screen pixel | UV Editor → View → **Texels per screen pixel** |

No keys; ask if you'd like one.

## Tips and common problems

- **Tip:** the screen is taken as 1080 pixels high with a 60° camera. On a bigger screen everything looks a little bluer; aim for green to light red.
- **Problem:** everything is grey. **Why:** the cameras can't see the shape, or nothing is selected in the UV Editor. **Fix:** select the objects; check the Distance isn't inside a bigger shape round it.
- **Problem:** a seam still lands where the player looks. **Why:** some cuts are needed to make a piece lie flat, and corners and hard edges are always cut (Seam suggestions settings). **Fix:** mark a [seam preference](seam-preferences.md) "never" on that edge, or move the seam by hand.
- **Tip:** the asset is looked at as built, standing on Z up. Its turn and size in the scene don't count.

## Related

- [Seams](seams.md), [Packing](packing.md), [Auto-unwrap and recipes](auto-unwrap.md)
- [Texture size and texel density](texture-and-density.md)
- [Concepts: texels per screen pixel, texel density](concepts.md#uvs-and-texturing)
