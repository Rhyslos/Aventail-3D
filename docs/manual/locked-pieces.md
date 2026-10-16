# Locking pieces and piece settings

> Lock a piece so no automatic tool changes it, or give a piece its own unwrap settings that every later unwrap uses.

## When to use it

**Locking.** Once a piece is laid out just right, you don't want a later Unwrap, Pack or modelling edit to undo your work. A locked piece is left exactly as it is by every automatic tool:

- Unwrap (and unwrapping as seams change), Relax and Straighten;
- Pack, which leaves it where it is and packs the others;
- Set and Match texel density, and Put on trim row;
- Suggest seams, which keeps the seams on its edges as they are;
- mending after modelling edits ([UVs while you keep modelling](uvs-and-modelling.md)).

You can still move, turn, scale, flip and pin it by hand. Locking is about the *automatic* tools.

**Piece settings.** Some pieces need different treatment from the rest:

- a face that should get more texture (a higher texel density);
- a curved part best laid flat with **Keep sizes** while the rest keeps shapes;
- a strip you'd rather not have straightened.

Give the piece its own settings once, and every unwrap of it uses them, whatever the app's settings say.

## How to use it

1. Select the pieces: in the UV mode click them, or select polygons in them (**1**, **2**, **3**). With nothing selected inside an object, the whole object is used. Settings always go on **whole pieces**.
2. **Locking:** choose **UV → Pieces → Lock pieces**. Locked pieces show faint grey stripes in the UV Editor. **UV → Pieces → Unlock pieces** frees them.
3. **Piece settings:** open **UV → Pieces → Piece settings** and choose:
   - **Method**: **The app's**, or one of the four ways (see [Unwrap](unwrapping.md#settings));
   - **Straighten as a strip**: **The app's**, **Straighten** or **Don't straighten**;
   - **Its own texel density**: tick it and set pixels per metre.

   Then choose **Give the selection these settings**. **Clear the selection's settings** puts them back to the app's.
4. Each is one undo step, saved with the project and listed on the object's [edit list](edit-list.md).

## Settings

The Piece settings submenu remembers what you last chose, so giving the same settings to another piece is quick. A piece's texel density is stored in the texture's own units: if you change the texture size later, the piece keeps its size on the texture.

## Shortcuts

| Action | Shortcut |
|---|---|
| Lock pieces, Unlock pieces, Piece settings | **UV → Pieces** |

No keys of their own; ask if you'd like some.

## Tips and common problems

- **Problem:** Unwrap did nothing. **Why:** the piece is locked. **Fix:** UV → Pieces → Unlock pieces, or select other pieces.
- **Problem:** after an extrude, new polygons next to a locked piece have squashed UVs. **Why:** new polygons made from a locked one are locked too, so mending leaves them. **Fix:** unlock, Unwrap the new part, and lock again.
- **Problem:** a piece ignores UV → Settings → Unwrapping → Method. **Why:** it has a method of its own. **Fix:** UV → Pieces → Piece settings → Clear the selection's settings.
- **Tip:** lock a hero piece (a face, a logo) before you Pack the rest: it stays where you put it.

## Related

- [Unwrap, Relax and Straighten](unwrapping.md)
- [Pack](packing.md)
- [Texture size and texel density](texture-and-density.md)
- [Pinning UVs](pins.md): hold some UVs, not a whole piece
- [Settings of the automatic UV tools](uv-settings.md)
