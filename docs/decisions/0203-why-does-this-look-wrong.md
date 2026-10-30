# 0203. Why does this look wrong?

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's twenty-fourth milestone ([0177](0177-texture-painter-plan.md)): a check that "points at visible seams, stretched pixels, blurry areas and flat lighting, explains each in plain words, and offers a fix, like the app's other explainers". Someone relearning painting often can't tell why a model looks off.

## Decision

- **A window, "Why does this look wrong?"**, opened by a button at the bottom of the Paint layers panel or from the command search. Like Check for baking ([0077](0077-bake-check.md)), it lists findings per object, each with a line, an explanation on pointing, **Show** (picks the polygons in the Polygons mode) and, where there is one, **Fix** (one undo step). It looks again whenever the scene or texture settings change while it's open.
- **What it finds** (`paint::advice::look_for`, on each object's polygons of a material and the material's mixed texture):
  - **Seams that show:** a UV seam (an edge whose two polygons have different UVs) whose two sides are painted more than 16% of the colour range apart, at most of three places along it, a texel and a half in from the edge on each side. Show picks the polygons beside them; the help says to paint across the seam on the model or blur along it.
  - **Stretched pixels:** polygons whose UVs squash them more than twice as much one way as the other. Show also switches the view to Stretch.
  - **Too few pixels:** polygons getting less than half the project's aimed-for pixels per metre (UV → Texture), in the world (the object's scale counts). **Fix** raises the texture size to the smallest that reaches it (at most 4096); the paint is repainted at the new size ([0191](0191-texture-size-without-blur.md)).
  - **Flat lighting:** the painting's lightest and darkest tenths less than 12% of the lightness range apart (with at least 16 painted samples). **Fix** adds a Light and shade layer ([0196](0196-light-and-shade.md)).
- **No keys.**

## Consequences

- Each common problem is named, explained and, where it can be, mended in one click.
- Seams are found where the paint differs, not where it merely could: a seam painted the same on both sides isn't flagged.
- Unpainted objects, and parts with no UVs, aren't judged. More checks (texture too big for its use, colours too saturated for a style) can join the list later.

## Amended 2026-10-30, room and smaller fixes

Flat lighting is not reported for a set with the realism channels on: a realistic material is lit by the game's lights, and baking light into it is the wrong advice.
