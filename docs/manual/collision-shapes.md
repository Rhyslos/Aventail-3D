# Collision shapes

> Choose what a game engine makes your object collide as, from the exact shape down to a box, see it outlined in the viewport, and split a shape with dents into a few wrapped parts.

A game doesn't usually collide things with the model you see. Testing every polygon of a detailed model against everything else, every frame, is slow, so the [physics engine](concepts.md#the-game-asset-pipeline) uses a simpler, invisible [collision shape](concepts.md#the-game-asset-pipeline) instead. The quickest kind to collide with is [convex](concepts.md#the-game-asset-pipeline): a shape with no dents, like a box, a ball or a wrapped parcel. Every object's **Collision** setting says which shape Godot should make, and the app now draws it, so you can see what the player will bump into before you export.

## When to use it

Pick by how the object is used and what shape it is:

| Collision | What it is | Use it for |
|---|---|---|
| **None** (standard) | Nothing: things pass through. | Decoration, grass, anything out of reach. |
| **The shape itself** | Exactly the model. Accurate but the slowest, and it can't move. | Walls, floors, terrain, stairs: big parts that never move. |
| **Wrapped (convex)** | One [convex hull](concepts.md#the-game-asset-pipeline): the shape wrapped tightly, as if in cling film, its dents filled in. Cheap, and it can move. | Crates, rocks, barrels, props that roll or get pushed. |
| **Wrapped in parts (convex)** | The shape split into a few convex parts, each wrapped: the dents stay open. Nearly as cheap as one. | Arches, doorways, tables and chairs, U-shaped walls, rings: anything you should fit under, through or into. |
| **A box** | A box round the shape. The cheapest. | Anything roughly box-shaped, or far from the player. |

Wrapped fills in every dent: wrap an arch and the opening is closed, so the player can't walk through it; wrap a chair and nothing fits under the seat. **Wrapped in parts** fixes that: the arch becomes its two legs and the top, the chair its legs, seat and back.

## How to use it

1. Select an object. In the [Attributes panel](attributes.md), under **Basic**, open **Collision** and pick one (rest the mouse on a choice to see what it's for).
2. The viewport outlines the collision shape in **yellow** while the object is selected: a box, one hull, or each part. *The shape itself* has no outline (it is the shape).
3. For **Wrapped in parts**, two more rows appear under Collision: **Max parts** and **Detail**. Change them and watch the outline follow. Splitting takes a moment on a detailed shape; it's worked out in the background, and the old outline stays until the new one is ready.
4. Export for **Godot** ([Export](export-and-import.md) with *Made for: Godot*, a Godot [game project folder](export-and-import.md#export-into-a-game-project), or [Export kit](kit-export.md)). Godot makes the collision shapes when it imports the file.

To see every object's collision shapes at once, not only the selected ones', switch on **Display → Show collision** in the viewport's menu line (also in **Edit → Settings… → Viewport**, under *Viewport display*, or press **Space** and type *show collision*).

### How the parts are made

The app looks for the dents and cuts there. It fills the shape with little cubes (voxels) to know what's inside, wraps it, and measures the **waste**: the empty space the wrapping fills in. While there are fewer parts than **Max parts** and a part wastes more than a little, it cuts the most wasteful part in two with a flat, straight cut (across, along or up), trying many places and keeping the one that wastes least, preferring a cut near the middle through a thin place. A box standing on another is cut where they meet. Then each part is wrapped again, exactly, so neighbouring parts meet with no gap. Parts that turn out convex together are joined back.

A convex shape (a box, a ball, a cylinder) stays one part, however high Max parts is. A ring uses every part it's allowed: the more, the closer they follow its hole. The same shape and settings always give the same parts.

### In the exported file

With Godot's names on, the app tells Godot through the names: *The shape itself* adds `-col` to the object's name, *Wrapped* adds `-convcol`, *A box* adds a box called `name-convcolonly` beside it, and *Wrapped in parts* adds each part beside it as `name-part1-convcolonly`, `name-part2-convcolonly` and so on. Godot recognises these endings, makes a collision shape of each, and doesn't draw the parts. Exports for other engines carry none of these.

## Settings

| Setting | What it does |
|---|---|
| **Collision** (Attributes → Basic) | None, The shape itself, Wrapped (convex), Wrapped in parts (convex), or A box. Saved with the project. Only exports made for Godot carry it. |
| **Max parts** (Wrapped in parts) | The most parts the shape is split into, 1 to 32. **8** as standard. More follow its dents more closely; fewer are cheaper for the game. A few (4 to 8) suit most props. |
| **Detail** (Wrapped in parts) | How small a dent is still followed, 0 to 100%. **50%** as standard. Higher finds smaller dents (and may use more parts, up to Max parts), but takes longer; lower leaves small dents filled in. |
| **Show collision** (Display menu) | Off (standard): only the selected objects' collision shapes are outlined. On: every shown object's. Remembered. |

Each part has at most 64 corners, as physics engines prefer. The settings are kept if you pick another Collision, so choosing *Wrapped in parts* again brings them back.

## Shortcuts

| Action | Shortcut |
|---|---|
| Show every object's collision shapes | **Display → Show collision**, or `Space`, then type *show collision* |

There's no key for it, so no key is taken from future tools. Ask if you'd like one.

## Tips and common problems

- **Problem:** the outline has one part, and an amber note says *Not split into parts: the shape isn't closed*. **Why:** splitting needs a closed shape, with an inside; this one has a hole in its surface (a missing polygon, an open end), or edges shared by more than two polygons. It collides as one wrapped shape instead. **Fix:** close it (select the border and [Fill hole](clean-up.md)), or use **A box** or **The shape itself**.
- **Problem:** *No collision shape: the shape is flat*. **Why:** a flat shape (a plane, a sign) has no inside to wrap. **Fix:** give it some thickness, or use **The shape itself**.
- **Problem:** the parts miss a small notch or groove. **Why:** it's smaller than Detail follows, or Max parts is used up. **Fix:** raise **Detail**, or **Max parts**. Small details rarely matter for collision: the player can't feel a 1 cm groove.
- **Problem:** a part looks a little smaller than a round shape. **Why:** each part keeps at most 64 corners, so a very round part is cut a little short of its curve. **Fix:** nothing needed for most games; for a closer fit, use more parts.
- **Tip:** an object with [modifiers](modifiers.md) collides as its exported shape (every modifier that is on), so a smoothed or bevelled shape's collision matches what Godot draws.
- **Tip:** collision doesn't need to be exact. Players notice getting stuck on an invisible corner far more than a box a little bigger than a barrel. Start cheap (A box, Wrapped) and use parts where the dents matter.
- **Tip:** Unity and Unreal Engine get the model but not the collision: set it up there.

## Related

- [Kit export and collision](kit-export.md): each piece of a kit in its own file, collision and all.
- [Export and import](export-and-import.md): Made for: Godot, and game project folders.
- [How shapes are drawn](viewport-display.md): the Display menu's other switches.
- [Concepts](concepts.md#the-game-asset-pipeline): collision mesh, convex, convex hull.
