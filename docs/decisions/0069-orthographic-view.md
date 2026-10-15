# 0069. An orthographic view that orbits, on P; flat views orbit into it

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** [0028](0028-ask-about-controls.md) (ask about controls), [0066](0066-clickable-axes.md) (clickable axes)

## Problem

The project lead needed an orthographic view "asap". There were flat Top, Front and Right views, but they are fixed: they could pan and zoom but not orbit. What was missing is a view with no perspective that can be turned any way, as Blender's Numpad 5 gives, for judging lines and heights from an angle and for isometric-looking assets.

## Options

The project lead was asked (decision 0028):

- **The key:** they first asked for `Ctrl + O`, but that is Open project in nearly every program. The alternatives offered were `O`, `Alt + O`, `P`, or moving Open. They chose **P** ("P for perspective").
- **Orbiting a flat view:** leave Top, Front and Right fixed, or **let orbiting turn them into a free orthographic view**, as Blender does (chosen).

## Decision

1. **`ViewKind::Orthographic`:** a flat view free to orbit, pan and zoom from any direction, named *Orthographic* in its corner. The ground grid and a plain backdrop with no horizon, as the other flat views have. A flat view looking exactly as Top, Front or Right look is that view; any other flat view is Orthographic.
2. **P** switches the active view between perspective and orthographic (`Navigator::toggled_projection`). It keeps the direction, the pivot and the pivot's place on screen, and things at the pivot's depth keep their size, so nothing jumps. In the Top view, P gives a perspective view looking (almost) straight down. The **Cameras** menu lists Perspective, Orthographic, Top, Front and Right; picking Perspective or Orthographic switches the same way, keeping the direction.
3. **Orbiting a flat view** turns it into an Orthographic one at the same zoom. The corner axis letters work in flat views too, so from Top, X lands exactly on the Right view. F2, F3 and F4 bring the exact views back.

## Consequences

- Orthographic is one more kind of view; the four-view layout still opens as Perspective, Top, Front and Right.
- A flat view can no longer be orbited away from by accident without its name changing; the manual says how to get back.
- The renderer needed no change: it already drew any flat camera, and an orthographic view uses the ground grid.
