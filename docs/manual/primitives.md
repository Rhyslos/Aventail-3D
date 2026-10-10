# Primitives

> Ready-made shapes to start a model from: cube, plane, cylinder, sphere, disc, tube and cone.

![The seven primitives side by side: cube (selected), plane, cylinder, sphere, disc, tube and cone](../screenshots/2026-10-10-primitive-gallery.png)

## When to use it

Almost every hard-surface model starts as a simple shape: a crate starts as a cube, a barrel as a cylinder, a lamp shade as a cone or a tube. These starting shapes are called [primitives](concepts.md#objects-and-scenes). Each one is made from a few settings, such as its size and how many pieces it's cut into. Those settings stay editable, so you can change a cube's size or segments long after you've started modelling it. (Changing them in the Attributes panel arrives with the next update.)

## How to use it

1. Open the **Create** menu and pick a shape, or click one of the blue shape buttons on the command palette (cube, cylinder, plane, sphere).
2. The new shape appears at the centre of the scene, selected (its edges and its name in the Objects panel turn orange). It's added at the top of the Objects list, named after its kind: *Cube*, then *Cube 2* for the next one, and so on.
3. Rename, group, hide or delete it in the Objects panel like any other object (see [Objects and the Object Manager](objects.md)). **Ctrl + Z** takes the new shape away again.

Every shape **stands on the ground** with its [pivot](concepts.md#objects-and-scenes) at the middle of its base, and the plane and disc lie flat on it. That's how game engines like to place things: put the asset on the floor and it sits there, instead of being half sunk in.

New shapes all appear at the centre of the scene, so they overlap. Moving them arrives with the Move tool in Phase 3; until then, hide the ones in the way with the eye in the Objects panel.

## The shapes

| Shape | What you get | Hard and soft edges |
|---|---|---|
| **Cube** | A box 2 m on each side, one piece per side. | All twelve edges are hard, so it shades with crisp corners. |
| **Plane** | A flat 4 m square on the ground, facing up, one piece. | Flat, so it has no shading breaks. |
| **Cylinder** | An upright cylinder, 1 m across and 2 m tall, with 16 sides and closed ends. | The side is soft (it shades as a smooth curve); the rims where it meets the ends are hard. |
| **Sphere** | A ball 2 m across resting on the ground, 24 pieces around and 12 from top to bottom. | All soft: one smooth surface. |
| **Disc** | A flat circle 2 m across on the ground, made of 24 straight pieces. | Flat. |
| **Tube** | A pipe 2 m across and 1 m tall, with a 1.4 m hole and 24 sides. | The inside and outside are soft; the four rims are hard. |
| **Cone** | A cone 2 m across and 2 m tall, with 16 sides and a closed base. | The side is soft; the rim of the base is hard. |

The ends of cylinders, tubes and cones, and the disc, are each **one face with many corners** (an [n-gon](concepts.md#mesh-basics)) rather than a fan of triangles. That keeps them clean to model on, and they stay flat, so they shade correctly.

Faces are drawn from both sides, so you can see the underside of a plane or look inside an open shape; from behind, a face is a little darker.

## Settings

Each shape has its own settings, set to the standard values above when it's created:

| Shape | Settings |
|---|---|
| Cube | Width, depth and height; segments along each |
| Plane | Width and depth; segments along each |
| Cylinder | Radius, height, sides, height segments, closed ends |
| Sphere | Radius, pieces around, pieces from top to bottom |
| Disc | Radius, hole radius, sides |
| Tube | Outer and inner radius, height, sides, height segments |
| Cone | Bottom and top radius (a top above zero cuts the point off), height, sides, height segments, closed ends |

Editing them in the Attributes panel arrives with the next update. Sizes are kept between 1 mm and 10 km, and segments and sides between sensible limits, so a typing slip can never make a shape vanish or freeze the app.

## Shortcuts

None yet: creating shapes is in the **Create** menu and on the command palette. (**B**, the shape key, arrives with the Create tool in Phase 3.)

## Tips and common problems

- **Tip:** start with the fewest segments you need. Low-poly game assets stay light, and you can always add segments later.
- **Problem:** a new shape seems to be missing. **Why:** it's inside another shape at the centre of the scene (a cube hides a cylinder, for example). **Fix:** hide the other shape with its eye in the Objects panel, or press **F** with the new shape selected to frame it.
- **Problem:** a plane or disc vanishes when you look at it edge on. **Why:** it's perfectly flat, with no thickness. **Fix:** orbit a little higher or lower.

## Related

- [Objects and the Object Manager](objects.md): select, rename, group, hide and delete shapes.
- [Concepts](concepts.md): [primitives and pivots](concepts.md#objects-and-scenes), [n-gons](concepts.md#mesh-basics), and [hard and soft edges](concepts.md#shading-and-normals).
