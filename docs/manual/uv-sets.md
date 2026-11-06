# UV sets

> Keep more than one UV layout on an object: one to paint, another for baked light or a detail texture, and choose which one the UV tools work on.

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

**Exporting:** glTF files get the set worked on as the first UV set (`TEXCOORD_0`) and one more as the second (`TEXCOORD_1`, which Godot calls **UV2** and uses for lightmaps): the set called **lightmap** if there is one, otherwise the first other set by name. OBJ files hold only the set worked on.

## Settings

None.

## Shortcuts

| Action | Shortcut |
|---|---|
| UV sets | **UV → UV sets** |

## Tips and common problems

- **Problem:** my painting looks wrong after switching sets. **Why:** paint is laid on by the set worked on. **Fix:** switch back to the set you painted with before painting more.
- **Problem:** Godot doesn't see my second set. **Why:** only one extra set is exported, and it's the one called lightmap first. **Fix:** name the set you want as UV2 **lightmap**.
- **Tip:** add a set before trying a very different layout, so the old one is a click away.

## Related

- [The UV Editor](uv-editor.md)
- [Layout snapshots](layout-snapshots.md)
- [Export and import](export-and-import.md)
- [Concepts: UV map](concepts.md#uvs-and-texturing)
