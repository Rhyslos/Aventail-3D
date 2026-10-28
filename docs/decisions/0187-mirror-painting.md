# 0187. Mirror painting

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's ninth milestone ([0177](0177-texture-painter-plan.md)): paint one side of a symmetrical model and have the other side painted too, as Substance Painter's symmetry (`L`) does. Faces, bodies, vehicles and most props are symmetrical, and painting both sides by hand doubles the work and never quite matches.

## Options

- **Reflect the dab's place in the world** and paint a dab there through the same camera. Simple, but the reflected place is often hidden or facing away from the camera (the back of a head), so the hidden-parts test would leave it unpainted, and the brush's tip and projection's picture would land the wrong way round.
- **Reflect the camera** (chosen): paint the other side as if seen by the stroke's camera reflected across the mirror's plane. Seen that way the reflected dab lands on the same screen point, faces the reflected camera exactly as the dab faces the real one, and everything that works from the screen (tips, projection, hidden parts) works the same on both sides.
- **Mirror in UV space only.** Works only when the UVs are laid out symmetrically, which the app's unwrapping doesn't promise.

## Decision

- **L** turns mirror painting on and off in Paint mode (Substance Painter's key); **Attributes → Mirror** shows it, with the axis: **X** (left and right, the default), **Y** (front and back) or **Z** (up and down). Z is up in the app ([0012](0012-z-up-axes.md)).
- The mirror is the object's own axis plane **through its pivot**, as for the live mirror ([Symmetry](../manual/symmetry.md)): move or turn the object and the mirror goes with it.
- **On the model**, each stroke makes a second view, the stroke's camera reflected across that plane (`ModelMirror`: the view-projection times the reflection, the eye and forward reflected), with its own depth map. Each dab's ray is reflected into the object's space and cast again; where it hits the surface, a dab is painted through the reflected view at the same screen point. A model that isn't quite symmetrical still gets the paint on its surface.
- **The polygon fill tool** fills what the reflected ray hits too.
- **In the UV Editor**, each dab is painted again flipped left and right across the texture's middle (U to 1 − U), the tip turned the mirrored way. The axis doesn't apply there.
- **Smudge and clone don't mirror:** both carry colour from one place, which would lay the first side's colour on the other. Every other brush, the eraser and the pen's eraser end mirror.
- **One undo step:** the mirrored dabs belong to the same stroke and are kept with it (`StrokeRecord::dabs`), so no format change.
- **The views show where the other dab goes:** a dashed circle at the reflected place, in the view the mouse is over.

## Consequences

- One side painted is both sides painted, with the same brush feel, and painting continues on the other side even when it's out of sight.
- An object with **Symmetry** whose halves share their UVs already shows the paint on both halves; mirror painting is for shapes modelled whole.
- The reflected stroke costs a second depth map at the start of each stroke and twice the dabs; milestone 10 (speed) measures it with the rest.
