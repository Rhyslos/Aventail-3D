# UV sets and lightmap UVs

> Keep more than one UV layout on an object: one to paint, another for baked light or a detail texture, and choose which one the UV tools work on. Make lightmap UVs for Godot in one click.

## When to use it

Usually one [UV map](concepts.md#uvs-and-texturing) per object is enough. A second one helps when:

- the object needs a **lightmap** for Godot's baked lighting: a layout where no two pieces share pixels and every piece has room around it, which the painted texture's layout (with stacked or mirrored pieces) can't be;
- a **big model** wants a second, more detailed texture for one part;
- you want to try **another layout** without losing the one you have (a [layout snapshot](layout-snapshots.md) is another way).

## How to use it

Open **UV → UV sets** with the objects selected.

1. The list shows the selected object's sets, the one the UV tools work on ticked. The first set is called **UVMap** until you rename it.
2. To add one, type a name (letters, digits, spaces, `-` and `_`) and choose **Add a set with this name**. It starts as a copy of the set worked on, so it's laid out from the start.
3. Click a set in the list to work on it: the UV Editor shows it, and every UV tool (seams, Unwrap, Pack, the brushes…) changes it. Painting uses the set worked on too.
4. **Rename the set worked on** gives it the typed name; **Delete the set worked on** takes it away (the next set takes its place; the last set can't go).

Each change is one undo step, applied to every selected object, and saved with the project. An object can have up to eight sets.

A change that can't be made changes nothing and records no undo step; the status bar says why (the last set can't go, the object already has eight sets, there's already a set with that name). With several objects selected, a change is made on those it fits: switching to a set only switches the objects that have it, and the status bar says how many of the selected objects changed.

**Exporting:** glTF files get the set worked on as the first UV set (`TEXCOORD_0`) and one more as the second (`TEXCOORD_1`, which Godot calls **UV2** and uses for lightmaps): the set called **lightmap** if there is one, otherwise the first other set by name. OBJ files hold only the set worked on.

## Lightmap UVs

Godot can bake lighting into a texture (a **lightmap**): soft shadows and bounced light, cheap to show in the game. It needs a UV layout of its own, where every bit of surface has its own pixels and room round it, so the light doesn't bleed or repeat. **UV → Lightmap UVs** makes one in a click:

1. Select the objects.
2. Open **UV → Lightmap UVs**, pick the **Lightmap size** (512 × 512 suits most props; bigger for large walls and floors) and the **Room round pieces** (4 px standard).
3. Choose **Make lightmap UVs**. Each object gets (or has replaced) a UV set called **lightmap** (an object that already has eight sets and none called lightmap is left as it is, and the status bar says so): cut along its seams and its hard edges, every piece at its real size, nothing stacked or mirrored, packed with that room between pieces. The UVs you paint with stay as they are.
4. Export to glTF: the lightmap goes in as the second UV set (UV2). In Godot, set the imported scene's **Light Baking** to *Static Lightmaps* and bake a LightmapGI node.

It's one undo step, saved with the project and recorded on the edit list, so it's made again if you change the shape before it. Make it again after editing the shape later.

## Settings

| Setting | What it does |
|---|---|
| **Lightmap size** | The lightmap's pixels across, which sets how much room is kept between pieces. 512 standard. |
| **Room round pieces** | Pixels between pieces at that size (4 standard). |

## Shortcuts

| Action | Shortcut |
|---|---|
| UV sets | **UV → UV sets** |
| Lightmap UVs | **UV → Lightmap UVs → Make lightmap UVs** |

## Tips and common problems

- **Problem:** my painting looks wrong after switching sets. **Why:** paint is laid on by the set worked on. **Fix:** switch back to the set you painted with before painting more.
- **Problem:** Godot doesn't see my second set. **Why:** only one extra set is exported, and it's the one called lightmap first. **Fix:** name the set you want as UV2 **lightmap**.
- **Problem:** Make lightmap UVs says an object already has 8 UV sets. **Why:** a shape can hold eight sets, and the lightmap would be a ninth. **Fix:** delete a set you don't need (UV → UV sets → Delete the set worked on), or rename one of them **lightmap** and make the lightmap again: it then replaces that set's layout.
- **Problem:** the baked light in Godot shows seams or blotches. **Why:** the lightmap is too small for the object, or pieces are too close. **Fix:** a bigger Lightmap size, or more Room round pieces, then make it again.
- **Tip:** add a set before trying a very different layout, so the old one is a click away.

## Related

- [The UV Editor](uv-editor.md)
- [Layout snapshots](layout-snapshots.md)
- [Export and import](export-and-import.md)
- [Concepts: UV map](concepts.md#uvs-and-texturing)
