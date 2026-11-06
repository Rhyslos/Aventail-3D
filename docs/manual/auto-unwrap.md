# Auto-unwrap and recipes

> Every UV step in one click (seams, unwrap, straighten, texel density, pack and lightmap UVs), each kept as an edit of its own that you can see, switch off and change; and recipes to unwrap a whole kit the same way.

## When to use it

Unwrapping a game asset well nearly always takes the same tools in the same order: cut it where it needs it ([seams](seams.md)), lay the pieces flat ([unwrap](unwrapping.md)), line up pieces made of quads, give every piece the same [texel density](concepts.md#uvs-and-texturing), [pack](packing.md) them into the texture, and make [lightmap UVs](uv-sets.md) for Godot's baked light. **Auto-unwrap** does all of that at once, so a prop goes from no UVs to ready in one click.

It doesn't hide what it did. Each step is an ordinary [edit](edit-list.md), kept together in an [edit layer](edit-layers.md) called **Auto-unwrap**, and a window says what each step did. If a step made things worse, switch it off and look again; if it needs different settings, change it like any other edit.

For a hero asset where every seam matters, do the steps by hand instead, or run Auto-unwrap first and adjust its result.

## How to use it

1. Select the objects (in the Model or Object mode, or any mode: Auto-unwrap always works on whole objects).
2. Open **UV → Auto-unwrap**. Untick any step you don't want (for example **Lightmap UVs** for an object that won't be in baked light).
3. Click **Auto-unwrap the selection**.
4. The **Auto-unwrap** window opens top right, listing the steps with what each did, such as "24 seams (24 edges changed)" or "6 pieces packed, 61% of the texture used".
5. Untick a step in the window to switch its edits off: the model shows the result without it. Tick it to switch it back on. Each switch is one undo step.
6. Close the window. **UV → Auto-unwrap → Show the last auto-unwrap** opens it again.

Ctrl + Z straight after Auto-unwrap takes the whole thing back in one go, layer and all.

## Recipes: the same unwrap for a whole kit

A **recipe** keeps the steps you ticked, every UV setting (seam rules, padding, lightmap size…) and the project's [texel density](texture-and-density.md) at the time (in pixels per metre, from UV Editor → Texture) under a name, so a whole kit can be unwrapped the same way, and the next kit too.

1. Choose the steps in **UV → Auto-unwrap** and the settings in **UV → Settings** until the result is right on one piece.
2. In **UV → Auto-unwrap**, type a name under **Recipes** ("Kit walls") and click:
   - **Keep in the project**: saved with the project, so anyone who opens it has the recipe too. Ctrl + Z takes it back.
   - **Keep in the app**: kept by the app for every project you open.
3. Select every piece of the kit and click the recipe's name in the list: each selected object is auto-unwrapped with the recipe's settings, in one undo step. Its Set texel density step gives the recipe's density, with this project's texture size. Your own UV settings, and the project's density, stay as they were. With nothing selected, the status bar says so and nothing happens.

Recipes kept by an earlier version of the app hold no density: their Set texel density step uses the project's density, as it did then. Keep the recipe again to keep a density with it.

Keeping a recipe under a name that's already used replaces it. The × beside a recipe deletes it.

## Settings

| Setting | What it does |
|---|---|
| **Suggest seams** | Cuts the shapes where they need it, by the seam settings in UV → Settings (the same as UV → Seams → Suggest seams). |
| **Unwrap** | Lays every piece flat, by the unwrap settings. |
| **Straighten** | Lines up pieces made only of quads in rows and columns. Pieces with triangles or other polygons are left as they are; if there are none, the window says so. Like the menu tool, it tells the pieces to stay straight after later edits ([UV intents](uv-intents.md)). |
| **Set texel density** | Gives every piece the project's texel density (UV Editor → Texture menu), or a recipe's, and tells them to keep it ([UV intents](uv-intents.md)). With Pack on after it, it evens out the pieces' sizes against each other, and Pack then scales them all together to fill the texture: the pieces keep the density they have once packed. |
| **Pack** | Fits every piece into the texture square, by the packing settings. |
| **Lightmap UVs** | Makes a second UV set called lightmap, exported as UV2, at the size and room chosen in UV → Lightmap UVs. |

All on as standard. Your choice is remembered.

## Shortcuts

| Action | Shortcut |
|---|---|
| Auto-unwrap | UV menu → **Auto-unwrap** |

No key; ask if you'd like one.

## Tips and common problems

- **Tip:** keep a recipe per kind of asset (walls, props, characters): the padding and density that suit one rarely suit another.
- **Problem:** a project recipe says it "can't be read". **Why:** it was kept by a newer version of the app with settings this one doesn't know. **Fix:** update the app, or keep the recipe again here.
- **Tip:** the steps run in order, each on the result of the one before. Switching off an early step (Suggest seams) leaves the later ones on: they're made again on the shape without it.
- **Tip:** to change one step's settings afterwards (say, the padding Pack used), open the [Edit layers](edit-layers.md) panel, find the step in the Auto-unwrap layer and change it there.
- **Problem:** the window says "No piece is made only of quads: nothing to straighten". **Why:** the shape is made of triangles (an icosphere, an imported scan). **Fix:** nothing to fix; Straighten only helps quad pieces.
- **Problem:** pieces look too small in the texture. **Why:** not the density: Pack scales every piece together to fill the texture, whatever density Set texel density gave them. What's left empty is the room between pieces (the padding) and the gaps round awkward shapes, such as long thin strips. **Fix:** lower the **Padding**, or turn on **Turn pieces to fit**, in the [packing](packing.md) settings, or cut long pieces with more seams.
- **Tip:** to keep the project's texel density exactly (so every prop in a kit gets the same pixels per metre), untick **Pack**: the pieces stay at that density, though they may then not fit in the square; pack them by hand afterwards if needed.

## Related

- [Seams](seams.md), [Unwrap, Relax and Straighten](unwrapping.md), [Packing](packing.md), [UV sets and lightmap UVs](uv-sets.md)
- [Edit layers](edit-layers.md) and the [edit list](edit-list.md)
- [Concepts: UV map, seam, texel density](concepts.md#uvs-and-texturing)
