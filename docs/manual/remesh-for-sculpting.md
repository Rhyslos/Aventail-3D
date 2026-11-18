# Remesh for sculpting

> Bake a sculpt, and any other shapes selected, into one even surface of polygons all about the same size: overlapping pieces merge into one, and every part has room for detail.

Sculpting moves the points a shape has; it doesn't add any. Stretch a part far with Grab or Snake hook and its polygons stretch too, until there aren't enough left to sculpt detail into. And a creature blocked out from a ball, a few cylinders and a box is still separate pieces poking through each other. **Remesh for sculpting** answers both: it wraps one brand-new surface round everything selected, at the **[voxel size](concepts.md#voxel-remesh)** you choose, then you carry on sculpting it.

It uses the same [voxel](concepts.md#objects-and-scenes) method as the [Remesh modifier](remesh.md), but it is not a modifier: it **bakes** the result into the shape, as one undo step. The Remesh modifier stays live and would work everything out again each time the shape changed, throwing your sculpting away; this doesn't.

## When to use it

- **Blocking out:** make the big shapes of a creature, a tree stump or a rock from [primitives](primitives.md), move them so they overlap, select them all, go to Sculpt mode and press **Ctrl + R**: one closed surface, joined where they met, ready to sculpt.
- **After a big pull:** when Grab, Elastic grab or Snake hook have stretched polygons thin (a horn, a tail, a long branch), remesh to even them out again.
- **Before detail:** a smaller voxel size gives more polygons everywhere, for finer detail.
- **To merge pieces:** a separate ear or horn sculpted as its own shape becomes part of the head.

Remeshing makes new polygons, so anything tied to the old ones starts again: UVs, the [mask](concepts.md#sculpt-mask), and the edit layers (see below). For a game model, the remeshed sculpt is the high-poly; the low-poly comes later in the phase.

## How to use it

1. In [Sculpt mode](sculpting.md), with the shape you're sculpting selected. To merge other shapes into it, select them too: **Ctrl + click** them in the Objects panel. The shape being sculpted (the Sculpt tab names it) keeps its place, and the others join it.
2. Set the **Voxel size** on the brush bar (or in the Sculpt tab's **Remesh** section). While you change it, a small grid of that size shows on the shape where the view looks, so you can see how big the new polygons will be. The bar's hint and the Remesh section say about how many polygons it will make.
3. Press **Ctrl + R** (or click **Remesh** on the brush bar, or choose **Sculpt → Remesh for sculpting**, or the button in the Sculpt tab).
4. A small window shows how far it has got, with **Cancel** (or **Esc**). The work runs in the background, so the window never freezes, even on millions of polygons; strokes wait until it's done.
5. Carry on sculpting. **Ctrl + Z** takes the whole remesh back, the other shapes too.

What happens to the shape: its edit layers (the [Sculpt layer](concepts.md#sculpt-layer) and the edits under it) are baked into its new base, a plain mesh, and a fresh, empty Sculpt layer starts on top for your next strokes. A primitive's settings (radius, segments) are gone, since it is no longer a primitive. Its [booleans](booleans.md), [live mirror](symmetry.md) and [modifiers](modifiers.md) stay on the object and still come after the sculpt. The other shapes merged in are deleted, as [Join](split-and-join.md) deletes them (anything inside them takes their place); their materials join by name.

## Settings

| Setting | What it does |
|---|---|
| **Voxel size** (brush bar and Sculpt tab) | How far apart the new surface's points are, in the world's centimetres: about the size of the new polygons. Half the size makes about four times the polygons. **2 cm** as standard. For a stylised character about 2 m tall, 1 to 2 cm is a good start (a few hundred thousand polygons); go smaller for finer detail later. From 0.05 to 50 cm. |
| **Keep detail** (Sculpt tab, on as standard) | After the new surface is evened out, it is laid back on the old one, so the sculpting the voxels can hold stays sharp. Off: the result is a little softer. |
| **Fix holes** (Sculpt tab, on as standard) | Closes an open shape's holes first (a plane's edge, a cylinder without caps), so it has an inside to remesh. Off: an open shape is refused, as the Remesh modifier refuses it. |

### Heavy results

- **More than 3 million polygons:** sculpting slows down and baking takes longer, so the app asks before it runs (**Remesh anyway** or **Don't remesh**). The bar's **Remesh** button turns amber while the size is that small.
- **More than 8 million polygons:** refused before it starts, with the smallest voxel size that fits.

## Shortcuts

| Action | Shortcut |
|---|---|
| Remesh for sculpting | `Ctrl + R` |
| Stop a remesh under way | `Esc` (or **Cancel**) |
| Take it back | `Ctrl + Z` |

Ctrl + R works only in Sculpt mode. The key is the plan's recommended choice, still to be reviewed.

## Tips and common problems

- **Problem:** "the voxels are too small: it would make about … million polygons". **Why:** more than 8 million polygons would be too heavy to sculpt or keep. **Fix:** use the voxel size the message suggests or bigger.
- **Problem:** "the pieces are too far apart for voxels this small". **Why:** the shapes selected are spread far apart, and the grid round them all would be too big. **Fix:** a bigger voxel size, or remesh the pieces one at a time.
- **Problem:** "nothing is left". **Why:** the shape is thinner than a voxel everywhere (a flat plane closed by Fix holes is as thin as it was). **Fix:** a smaller voxel size, or give the shape some thickness first.
- **Problem:** thin parts (a tail's tip, a finger) merged or vanished, or small gaps closed. **Why:** anything thinner than about a voxel can't be told apart. **Fix:** a smaller voxel size; or remesh, then sculpt the thin part back.
- **Problem:** "The shapes changed while they were remeshed". **Why:** something changed a shape (an undo, say) before the remesh finished, so its result no longer fitted. **Fix:** press Ctrl + R again.
- **Problem:** the mask is gone after remeshing. **Why:** the mask belonged to the old points. **Fix:** paint it again on the new surface.
- **Tip:** remesh early and often while blocking out, with a big voxel size; go smaller only when the big forms are right. A light shape is quicker to sculpt.
- **Tip:** symmetry keeps working after a remesh as long as the shapes were placed symmetrically: the new surface is as symmetric as what went in.

## Related

- [Sculpting](sculpting.md), [Sculpt brushes](sculpt-brushes.md), [Symmetry and masks while sculpting](sculpt-symmetry-and-masks.md)
- [Remesh](remesh.md), the live modifier, and why the two differ
- [Split and join](split-and-join.md), which joins shapes without remeshing them
- [Heavy shapes](heavy-shapes.md)
- [Concepts: voxel remesh](concepts.md#voxel-remesh), [Sculpt layer](concepts.md#sculpt-layer)
- Why it works this way: [decision 0312](../decisions/0312-remesh-for-sculpting.md)
