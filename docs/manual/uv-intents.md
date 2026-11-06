# UV intents

> Pieces remember what they were told to be (a straight strip, on a trim row, at a texel density, this way up), so changing the model doesn't undo your UV work.

## When to use it

You straighten a strip of quads so a tiling texture runs along it, then inset a polygon in the middle. The app mends the UVs the inset broke ([mending](uv-editor.md)), but a plain mend would leave the strip slightly bent. With an **intent**, the piece is straightened again after the inset, automatically.

Intents are set for you by the tools that imply them:

- **Straighten** tells the piece to stay a **straight strip**.
- **Set texel density** and **Match texel density** tell it to keep its **density**.
- **Put on trim row** tells it to stay **on that trim row**.
- [Auto-unwrap](auto-unwrap.md)'s **Straighten** and **Set texel density** steps do the same as the tools. With its **Pack** step on too, the pieces keep the density Pack gave them (Pack scales every piece to fill the texture).

A piece keeps one intent: the last one it was told. Straighten and then Set texel density leaves it keeping its density.

And one you set yourself: **Keep this way up**, for textures with a direction (wood grain, writing, drips, rain streaks): the piece is turned so the world's up runs up the texture, and kept that way.

See [Concepts: UV intent](concepts.md#uvs-and-texturing).

## How to use it

1. Use Straighten, Set texel density or Put on trim row as usual. The intent is set in the same undo step.
2. Model away: inset, extrude, bevel, or Unwrap again. Each piece that changed and has an intent is made so again.
3. To see what a piece keeps, point at one of its polygons in the [UV Editor](uv-editor.md) (in the Polygons or UV mode): the note bottom left says, for example, "Keeps: on the trim row from 25% to 50% up".
4. For a direction: select the pieces and choose **UV → Pieces → Intents → Keep this way up**.
5. To let pieces go: select them and choose **UV → Pieces → Intents → Clear intents**.

## Settings

| Setting | What it does |
|---|---|
| **Keep this way up** | Tells the selected pieces (whole objects with nothing selected inside) to keep the world's up running up the texture. They are turned at once, and kept that way. |
| **Clear intents** | The selected pieces forget what they were told. Edits lay them out as mending does, and nothing more. |

Intents are re-applied only with mending on (UV → Settings → Mending) for modelling edits, and always after Unwrap. Locked pieces are never changed.

## Shortcuts

| Action | Shortcut |
|---|---|
| Intents | UV menu → Pieces → **Intents** |

No keys; ask if you'd like one.

## Tips and common problems

- **Tip:** moving a piece by hand never fights its intent. Intents come back only after modelling edits and Unwrap, so you can still slide a trim piece along its row with W.
- **Problem:** a piece keeps snapping back to its old density after I scaled it by hand and then extruded. **Why:** it still has its density intent. **Fix:** Clear intents on it, or Set texel density again at the new size.
- **Problem:** Keep this way up says a piece lies flat, with no up to keep, and nothing changed. **Why:** a piece lying flat (a floor, a ceiling, a table top) has no up: the world's up points straight out of it. Such pieces are left as they were, and without an intent. **Fix:** nothing to fix; for floors, turn the piece by hand (R in the UV Editor).
- **Problem:** Clear intents says nothing changed. **Why:** the selected pieces keep no intents. **Fix:** nothing to fix.

## Related

- [The UV Editor](uv-editor.md), [Unwrap, Relax and Straighten](unwrapping.md)
- [Texture size and texel density](texture-and-density.md)
- [Concepts: UV intent](concepts.md#uvs-and-texturing)
