# Weighted normals

> Shade a bevelled shape so its big flat faces look perfectly flat and its bevels look round, the way game models are usually shaded.

**Weighted normals** is a [modifier](modifiers.md). It doesn't move a single point: it only changes the [normals](concepts.md#shading-and-normals), the invisible arrows that tell the light which way the surface faces at each corner.

Why that matters: on a smooth-shaded shape, each corner's normal is the average of the polygons round that point. On a box with bevelled edges, the thin bevel strips count as much as the big face beside them, so the big face's corners lean towards the bevel. The light then slides across the big face in a gradient, and it looks dented or bloated. The usual cure is to mark the edges [hard](hard-and-soft-edges.md), but then the bevels look faceted, or to bake a [normal map](concepts.md#the-game-asset-pipeline) from a high-poly model, which is a lot of work for a simple prop.

Weighted normals makes the **bigger polygons count for more**. The big face wins at its corners, so its normals point straight out of it and it shades flat; the bevel's normals bend between the two faces it joins, so the bevel catches the light like a rounded edge. Game engines (Godot, Unity, Unreal) and Substance Painter all read these normals from the exported file, so the model looks the same there.

## When to use it

- Low-poly hard-surface props with bevelled or chamfered edges: crates, panels, weapons, machines, furniture.
- After a [bevel](bevel.md), [Subdivide](subdivide.md) at a low level, or a [boolean](booleans.md) cut, when big faces shade unevenly.
- To get clean shading without a high-poly bake.

It isn't for organic, evenly divided shapes (a character, a rock): their polygons are all about the same size, so weighting changes little. On a shape with hard edges everywhere it changes nothing, since each face already shades on its own.

## How to use it

1. Select one or more shapes. Shapes with bevelled edges and soft shading show the difference best: set the object's **Smoothing** (in the [Attributes panel](attributes.md)) to a larger angle, or to shade by its marked edges, so the bevels are soft.
2. Click **Weighted normals** on the **modifiers palette** (a box seen from the side with arrows on top, the last button on the right of the viewport), or choose **Mesh → Modifiers → Weighted normals** (under *Game ready*), or press **Space**, type *weighted*, and press **Enter**.
3. The big faces now shade flat. Change its settings in the Attributes panel under **Modifiers → Weighted normals**.
4. **Keep it last** in the list of modifiers (only a [Triangulate](triangulate.md) may come after it: it keeps the normals). If another modifier comes after it, the panel says so in amber: use **Down** to move Weighted normals to the end.
5. [Export](export-and-import.md) as usual: the normals go into the file. Or **Bake into shape** (**Ctrl + E**) to keep them in a plain mesh.

On a big shape (from about 19 000 square polygons) the normals are worked out in the background while you keep working; 200 000 polygons take about a fifth of a second.

## Settings

| Setting | What it does |
|---|---|
| **Weight by** | What makes a polygon count for more at its corners. **Polygon area** (standard): big faces win, the usual choice for hard-surface models. **Corner angle**: each polygon counts by the angle it fills at the point, as plain shading does. **Area and angle**: both multiplied. |
| **Face influence** | How much the biggest polygon at a point wins over the smaller ones. At **0%** each counts by its weight alone (a big face still leans a little towards its bevel); higher, smaller polygons count ever less; at **100%** only the biggest counts. **75%** as standard keeps a 1 m face perfectly flat beside a 5 cm bevel. |
| **Strength** | How far the shading turns from the usual towards the weighted: **100%** (standard) all the way, 50% half way, 0% not at all. |
| **Keep hard edges** | On (standard): the shading stays crisp along the edges the object's shading shows hard: those marked hard if it shades by its marks, or those sharper than its [Smoothing](hard-and-soft-edges.md#smoothed-by-angle) angle. Off: every point gets one normal and the whole shape shades smooth, hard edges and all. |
| **Show in viewport** | Off: the viewport shows the usual shading; exports and Bake into shape still use the weighted normals. |

### Why it should come last

The normals it makes are worked out for the exact shape it was given. A modifier after it changes the shape (Subdivide adds polygons, a bend moves points), so the normals no longer fit: they're dropped there (except by [Triangulate](triangulate.md), which only cuts polygons into triangles and keeps each corner's normal), and the shading is worked out the usual way again. The same happens to normals baked into a shape when you then edit it: move a point, extrude, mark an edge hard, and the shape goes back to the usual shading (undo brings them back). Changes that don't touch the shading, such as UVs, seams and materials, keep them. The live [mirror](symmetry.md) and [booleans](booleans.md) drop them too, since they join new surface to the shape.

### What the bake check sees

The [bake check](bake-check.md) warns about hard edges whose UVs aren't split. With **Keep hard edges** off, the exported shading has no hard edges at all, so there's nothing for it to warn about on that shape.

## Shortcuts

| Action | Shortcut |
|---|---|
| Find Weighted normals | `Space`, then type *weighted* |
| Make it permanent (Bake into shape) | `Ctrl + E` |

Weighted normals has no key of its own.

## Tips and common problems

- **Problem:** nothing seems to change. **Why:** the shape's edges are all hard (by its Smoothing angle, or marked), so every face already shades on its own. **Fix:** raise the object's Smoothing angle above your bevel's angle (a 45° bevel needs more than 45°), or shade it by its marked edges and mark only the edges you want crisp.
- **Problem:** the big faces still look slightly shaded. **Why:** Face influence is low, or another modifier comes after Weighted normals. **Fix:** raise **Face influence**, and move Weighted normals to the end.
- **Problem:** the shading went back to normal after an edit. **Why:** weighted normals baked into a shape fit only that shape; the edit dropped them. **Fix:** keep Weighted normals as a live modifier until you've finished modelling, and bake it last.
- **Problem:** in the engine, the model looks different from the viewport. **Why:** the engine works its own normals out instead of reading the file's. **Fix:** in Godot, keep the import's normals (they're used as standard); in Unity and Unreal, set the model's import to *Import normals* rather than *Calculate*. Use the glTF (`.glb`) export, which carries tangents made from these normals for normal maps.
- **Tip:** a 1- to 3-segment bevel with weighted normals looks rounded from a game's distance at a fraction of the polygons a smooth high-poly edge needs.

## Related

- [Modifiers: switching off, reordering, Show in viewport, baking](modifiers.md)
- [Hard and soft edges](hard-and-soft-edges.md) and the object's Smoothing
- [Bevel](bevel.md), which makes the edges it shades, and [Live bevel](live-bevel.md), the bevel as a modifier: put it before Weighted normals
- [Game bevel](game-bevel.md): a Live bevel with round shading and Weighted normals last, in one click
- [Export and import](export-and-import.md) and [Check for baking](bake-check.md)
- [Triangulate](triangulate.md): the one modifier that can come after Weighted normals and keep its normals
- [Concepts: normals, weighted normals, custom normals](concepts.md#shading-and-normals)
- Why it works this way: [decision 0148](../decisions/0148-weighted-normals.md)
