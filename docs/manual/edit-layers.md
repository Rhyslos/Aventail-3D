# Edit layers

> Edit layers keep sets of edits apart on an object, so you can switch a whole set off to compare, move it, merge it or delete it in one step.

Every change you make to a shape's points, edges and polygons (an extrude, a bevel, a move) is recorded as an *edit* on top of the shape's [base](concepts.md#objects-and-scenes), and the edits are kept in *layers*. Until now every object had just one layer, called Edits. Adding a layer starts a fresh set: the edits you make from then on go into it. Because it's a separate set, you can switch it off and see the shape without it, then switch it back on, a bit like layers in a paint program. 3ds Max users know this as stacking Edit Poly modifiers. See [Concepts: edit layer](concepts.md#objects-and-scenes).

![The Edit layers panel beside Objects: the cube's three layers, top first (Edits 2, marked "new edits"; Details, picked, with its right-click menu open; Edits), each with an eye and its edits listed under it. The Add edit layer button sits in the thin column on the right of the viewport.](../screenshots/2026-10-16-edit-layers.png)

## When to use it

- **Before trying something out:** add a layer, model the idea, then switch the layer off and on to compare. Delete it if you don't like it.
- **To keep stages of a model apart:** the blocked-out shape in one layer, details in the next, so you can hide the details while you rework the big shapes.
- **To take back many edits at once** without undoing everything made since.

## How to use it

1. Select an object with a shape (in any mode).
2. Click **Add edit layer** in the thin column on the right of the viewport (the stack of sheets), or choose **Mesh → Add edit layer**. Each selected object gets an empty layer on top, and the **Edit layers** panel (a tab beside Objects) comes to the front.
3. Model as usual. Your edits go into the layer marked **new edits**.
4. In the Edit layers panel:
   - click a layer's **eye** to switch it off or on;
   - **double-click** its name to rename it;
   - **right-click** it (or click it, then open the panel's **Layer** menu) to **Rename**, **Switch off**, **Move up**, **Move down**, **Merge down** or **Delete layer**.

The panel lists the selected object's layers with the top one first, as they stack: the bottom layer's edits are made first, and each layer works on the shape the layers below it made. Under each layer are its edits, newest first. Every change to a layer is one undo step (**Ctrl + Z**).

## Settings

| Action | What it does |
|---|---|
| **Eye (switch off / on)** | A layer that is off is skipped: the shape looks as if its edits were never made. Nothing is lost; switch it on to bring them back. Exports use what you see. |
| **Rename** | Give the layer a name that says what's in it ("Panel lines"). Names can't be empty. |
| **Move up / Move down** | Change when the layer's edits are made, after or before the layer next to it. Edits find the points they were made on by place, so most still land where they were; an edit that can't find all of them says so in the Attributes panel. |
| **Merge down** | Put this layer's edits at the end of the layer below, making the two one layer (3ds Max calls this *collapse*). Both layers must be on. The shape doesn't change. |
| **Delete layer** | Remove the layer and every edit in it. |
| **Add layer** | At the top of the panel: an empty layer on top of the selected object. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Add an edit layer | **Add edit layer** beside the viewport, **Mesh → Add edit layer**, or **Add layer** in the panel |
| Rename a layer | Double-click its name |
| Every layer action | Right-click a layer, or the panel's **Layer** menu |

There are no keys for layers yet.

## Tips and common problems

- **Problem:** my edit didn't go into the top layer. **Why:** the top layer is switched off, and new edits always go into the top layer that is *on*, so you can see them. **Fix:** switch the top layer back on first. If every layer is off, the next edit makes a new layer on top.
- **Problem:** Merge down is greyed out. **Why:** it's the bottom layer, or one of the two layers is off. Merging would otherwise switch edits on or off by surprise. **Fix:** switch both on.
- **Problem:** the panel says to select one object. **Why:** it shows one object's layers at a time. **Fix:** select just one. (**Add edit layer** still works on several at once.)
- **Tip:** groups have no shape, and an instance shows its original's shape and edits, so neither gets layers.
- **Tip:** layers and their names are saved with the project.

## Related

- [Concepts: base and edits, edit layer](concepts.md#objects-and-scenes)
- [The Attributes panel](attributes.md): the Last edit section changes the newest edit
- [The Objects panel](objects.md)
- [Interface tour](interface-tour.md)
