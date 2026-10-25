# Modifiers

> Live changes on top of a shape: add one, change its settings any time, switch it off, reorder it or take it away, and the shape you modelled underneath never changes.

A [modifier](concepts.md#objects-and-scenes) is a change the app works out from your shape every time it's shown or exported, instead of a change made to its points once and for all. Rebuild a shape as even quads with [Remesh](remesh.md), and keep modelling the original: the remeshed version follows each edit. Don't like it? Switch it off, or remove it: nothing is lost. Happy? **Bake into shape** makes it permanent.

Each object has its own list of modifiers, its **modifier stack**, worked out first to last: each modifier gets the result of the one before. So far there are eight kinds, **[Subdivide](subdivide.md)**, **[Scatter](scatter.md)**, **[Lattice](lattice.md)**, **[Bend along curve](bend-along-curve.md)**, **[Shrinkwrap](shrinkwrap.md)**, **[Remesh](remesh.md)**, **[Decimate](decimate.md)** and **[Weighted normals](weighted-normals.md)**; more arrive through this phase (bend and twist, array, shell and others), and they all work the way this page describes.

## When to use it

- When you want a result that would be slow or impossible to make by hand (an even surface of quads), but still want to keep editing the shape underneath.
- When you want to try something and compare with and without it: one click switches a modifier off.
- When you're done with a modifier's settings, bake it ([Bake into shape](objects.md#bake-into-shape)) to edit the result point by point.

For a change you want to make once, at one spot, an ordinary tool (extrude, bevel, loop cut) is the better choice: it becomes an [edit](edit-list.md) you can still change later.

## How to use it

1. **Select one or more shapes** (in the viewport or the Objects panel). Groups are left out; an [instance](copies-and-instances.md) shows its original's modifiers, so give them to the original.
2. **Add a modifier**, from any of these:
   - its button on the **modifiers palette**, the thin column of buttons on the right of the viewport, below the [booleans](booleans.md);
   - **Mesh → Modifiers**, where they're grouped by what they do (Generate: Subdivide, Scatter onto surface; Deform: Add lattice, Bend along curve, Shrinkwrap onto; Clean up: Remesh, Decimate; Game ready: Weighted normals);
   - **Space** ([command search](command-search.md)), then type its name (*remesh*) and press **Enter**.
3. The modifier goes at the end of each selected shape's list, on, and the viewport shows the result. (**Scatter onto surface** is an exception: it goes on the shape selected *first*, and scatters the others over it; see [Scatter](scatter.md). **Bend along curve** and **Shrinkwrap onto** too: select the shape first, then Ctrl + click the curve or the target; see [Bend along curve](bend-along-curve.md) and [Shrinkwrap](shrinkwrap.md).) The [Attributes panel](attributes.md) lists it under **Modifiers**, with its settings.

If the button or menu entry is greyed out, hover over it: it says what's missing (nothing selected, or an instance selected).

### Changing modifiers later

In the shape's **Attributes**, under **Modifiers**, each modifier has:

- **its switch**, the box before its name: untick it to see the shape without it. It stays, with its settings, until you tick it again.
- **Up** and **Down**: the order it's worked out in. Order matters: a remesh followed by a coarser one gives a different shape from the other way round, and later modifiers (smoothing, weighted normals) will usually want to come last.
- **Remove**: takes it away. **Ctrl + Z** brings it back.
- **its settings**: drag or type, and the shape follows. One drag is one undo step.
- **Show in viewport**: untick it and the viewport skips the modifier, while exports and Bake into shape still use it. Handy for a slow modifier while you model; tick it again to check the result.

If a modifier can't do its work, it says why in amber ("Not worked out: …"), and the shape goes on without it, as if it were switched off. Fix the cause and it works again.

### Where modifiers come in

The app works out a shape in this order:

1. its starting shape (a primitive's settings, or a mesh) and your edits, layer by layer;
2. its [booleans](booleans.md);
3. its [mirror](symmetry.md);
4. its modifiers, first to last.

So a modifier sees the whole shape: cut by its booleans, with both halves of a mirrored shape. The Points, Edges and Polygons modes still work on the shape **before** the modifiers: what you select and edit are your own points, and the modifiers are worked out again after every change, so in those modes you see your points over the modified result.

### Making them permanent

**Bake into shape** (**Ctrl + E**, or **Mesh → Shape and object**) turns the shape as an export would get it (for [Subdivide](subdivide.md), at its export levels) into a plain mesh: edits, booleans, mirror and the modifiers that are on, all in one. Those modifiers go; any switched off, or that couldn't do their work, stay. A mirror goes too, as its other half is now part of the mesh. One **Ctrl + Z** undoes it all.

**Apply mirror** on a shape with modifiers bakes only what comes before them (edits, booleans and mirror); the modifiers stay live on the result.

### Saving and exporting

Modifiers are saved with the project, live, and come back as they were. [Export](export-and-import.md) sends out each shape with its modifiers that are on, including those hidden in the viewport: what goes to the game engine or Substance Painter is the finished shape.

## Settings

| Setting | What it does |
|---|---|
| **The switch** (before the name) | On: the modifier does its work. Off: the shape is as if it weren't there; it keeps its settings. |
| **Up**, **Down** | Where it is in the list. Modifiers are worked out first to last, each on the one before's result. |
| **Remove** | Takes it away. |
| **Show in viewport** | Off: the viewport skips it (the shape draws faster while you model), but exports and Bake into shape still use it. On as standard. |
| Its own settings | See its page: [Subdivide](subdivide.md), [Scatter](scatter.md), [Lattice](lattice.md), [Bend along curve](bend-along-curve.md), [Shrinkwrap](shrinkwrap.md), [Remesh](remesh.md), [Decimate](decimate.md), [Weighted normals](weighted-normals.md). |

## Shortcuts

| Action | Shortcut |
|---|---|
| Find a modifier by name | `Space`, then type the name |
| Make the modifiers permanent (Bake into shape) | `Ctrl + E` |
| Undo adding, changing, moving or removing one | `Ctrl + Z` |

Modifiers have no keys of their own.

## Tips and common problems

- **Problem:** the result appears a moment after a change. **Why:** heavy modifiers (a fine remesh of a big shape) are worked out on another thread, so the window stays smooth; the last result is shown meanwhile. **Fix:** nothing to fix. While modelling, a coarser setting, or **Show in viewport** off, keeps things quick.
- **Problem:** "Not worked out: the shape isn't closed". **Why:** that modifier needs a [closed solid](concepts.md#mesh-basics). **Fix:** see [Remesh](remesh.md#tips-and-common-problems).
- **Problem:** the Points, Edges or Polygons modes show different points from the shape drawn. **Why:** they edit your own shape, before the modifiers. **Fix:** that's how it should be; bake the modifiers to edit the result itself.
- **Problem:** hiding polygons doesn't hide them from a modifier's result. **Why:** modifiers work on the whole shape, hidden polygons included (as booleans do); hiding is for editing. **Fix:** switch the modifier off while you work on the hidden part.
- **Tip:** a modifier on an object is copied with it (Ctrl + C, Ctrl + V, Duplicate); an instance follows its original's modifiers.

## Related

- [Subdivide](subdivide.md), [Lattice](lattice.md), [Remesh](remesh.md), [Decimate](decimate.md) and [Weighted normals](weighted-normals.md) (best last in the list)
- [Concepts: modifiers](concepts.md#objects-and-scenes)
- [The Attributes panel](attributes.md), [Bake into shape](objects.md#bake-into-shape)
- [Booleans](booleans.md) and [Symmetry](symmetry.md), which come before the modifiers
- Why it works this way: [decision 0142](../decisions/0142-modifier-stack-and-remesh.md)
