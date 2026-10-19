# Why is this here?

> Tick **View → Explain choices** in the UV Editor, then point at a seam or a piece: it says why it's there and how to change it.

![The UV Editor with Explain choices on, pointing at a seam: the seam is lit green and a box by the mouse reads "Seam. It's a hard edge. A seam where the shading turns hard keeps the normal map clean there. To change it: select the edge (Edges mode, 2) and choose UV → Seams → Clear seams, or Never seams here so suggestions avoid it. Prefer seams here on other edges draws cuts there instead."](../screenshots/2026-10-17-why-is-this-here.png)

## When to use it

The app makes many UV choices for you: [Suggest seams](seams.md) picks where to cut, [Unwrap](unwrapping.md) sizes pieces, [Pack](packing.md) shrinks and places them. When a result surprises you ("why is there a seam across the front?", "why is this piece so small?"), you want the reason, and the way to get what you'd rather have. Explain choices gives you both, without changing anything.

It's off as standard, so the UV Editor stays quiet while you work (the project lead's choice). Turn it on when you're puzzled, turn it off when you know.

## How to use it

1. In the UV Editor, open **View** and tick **Explain choices**.
2. Point at a **seam** (the orange lines). It lights up green, and a box by the mouse says why it's there:
   - **a hard edge**: seams where the shading turns hard keep normal maps clean;
   - **a sharp corner**: the polygons meet at a sharper angle than **UV → Settings → Seam suggestions → Cut corners sharper than**;
   - **to lie flat**: the piece is closed (a box, a ball) or loops round (a tube), so it needs a cut; this is the most hidden one, as **Hide cuts** says;
   - **preferred**: the piece needed a cut, and you marked this edge **Prefer seams here**;
   - **a stretch budget**: the piece stretched more than its budget once laid flat, so it was slit here;
   - **not one Suggest seams would make**: you marked it by hand, it came with a ready-made shape, or another tool added it.

   Then it says how to change it: select the edge and use **UV → Seams → Clear seams** or **Never seams here**, or prefer seams elsewhere.
3. Point inside a **piece**. The box lists what decides it, each with what to do:
   - locked, on a [trim row](trim-sheets.md), pinned, or with [settings of its own](locked-pieces.md);
   - its size: its [texel density](texture-and-density.md) against the one aimed for, and why it differs (Pack shrinks every piece by the same amount to fit them all);
   - its place: where it was laid out, moved, or packed;
   - whether it overlaps another piece or lies mirrored (see [Seeing and checking UVs](seeing-and-checking.md)), and how stretched it is.
4. Untick **View → Explain choices** to stop.

Seam reasons are worked out from the current settings, the same way Suggest seams would. If you've changed a setting since suggesting seams, a seam may be explained as "not one Suggest seams would make": run Suggest seams again to see the new choices.

## Settings

| Setting | What it does |
|---|---|
| **View → Explain choices** | Explains what the mouse points at in the UV Editor. Off when the app starts. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Explain choices | **View → Explain choices** in the UV Editor, then point |

No keys or clicks of its own: pointing is enough.

## Tips and common problems

- **Problem:** pointing at a seam on a big shape says it's too big to explain. **Why:** working out Suggest seams' reasons takes too long on shapes over 20 000 polygons while you point. **Fix:** the piece explanations still work; for seams, check the settings in **UV → Settings → Seam suggestions**.
- **Problem:** a seam is explained as a sharp corner, but I want the piece in one. **Why:** the corner is sharper than the corner setting. **Fix:** **UV → Seams → Never seams here** on that edge, or raise **Cut corners sharper than**, then Suggest seams again.
- **Problem:** every piece says it's under the aim. **Why:** Pack scales all pieces down together until they fit the square with their padding. **Fix:** a bigger texture, less padding, or a texture per material (**UV → Settings → Packing**).
- **Tip:** with **View → Numbers on pieces** on as well, you see every piece's size at once and the reason for the one you point at.

## Related

- [Seams](seams.md)
- [Seam preferences and stretch budgets](seam-preferences.md)
- [Pack](packing.md)
- [Seeing and checking UVs](seeing-and-checking.md)
- [Settings of the automatic UV tools](uv-settings.md)
