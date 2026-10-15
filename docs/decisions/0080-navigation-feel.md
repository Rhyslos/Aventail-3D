# 0080. Navigation feel: a pivot you can see, steady close zoom, no lost movement

- **Status:** Accepted
- **Date:** 2026-10-15
- **Follows:** [0015](0015-navigation-schemes.md) (navigation schemes) and [0017](0017-picking-the-point-under-the-mouse.md) (the point under the mouse)

## Problem

Testing, the project lead found navigating close up hard:

- orbiting sometimes swung around nothing;
- panning was sometimes far too fast;
- zooming in on something 30 cm big "fought back", and orbiting that close was painful.

All four came from the pivot, the point the view turns around, zooms towards and pans at the depth of:

- **Over empty space, orbiting kept the last pivot**, wherever it was. That could be a point left in mid-air or on distant ground by zooming over nothing, out of view.
- **Zooming over empty ground took the ground up to four times farther away than the pivot.** Every zoom step, pan and orbit after that was scaled to that far point.
- **Panning moved at the pivot's depth**, so a far pivot made it fast.
- **Zooming moved a share of the distance each step**, creeping towards the surface and stopping 2 cm short.
- Besides that, egui only takes a press for a drag once the mouse has moved 6 pixels, and those first pixels were lost: a slow start felt sticky.

## Options

- **A pivot over empty space:**
  - keep the last one (as before);
  - always the middle of the view;
  - **the first of these that is in front of the camera and on screen** (chosen):
    1. the surface point last pointed at;
    2. the middle of the selection;
    3. the middle of everything shown;
    4. otherwise the middle of the view.
  - So it is always something you can see, and usually what you're working on.
- **Panning:** at the pivot's depth (as before), or **at the depth of what you grabbed** (chosen): the grabbed spot stays under the mouse, the same rule over empty space as orbiting.
- **Close zoom:**
  - a smaller minimum distance;
  - **zoom through** (chosen): within 10 cm the steps stop shrinking and the pivot is pushed ahead, as 3ds Max and Blender's dolly do.
- **The turning-point ring** (asked, the project lead chose): it existed, but only around the mouse. Now it shows whenever you orbit, larger and with a shadow, and **can be switched off** (Options menu and Settings → Viewport, on as standard).

## Decision

1. `View::pivot_over_empty_space` chooses the pivot for an orbit or pan started over empty space, by the rule above. `View::pivot_on_surface` remembers whether the pivot is a real surface point.
2. A pan started on a surface makes that point the pivot, so it stays under the mouse.
3. `zoom_target` uses the ground only when it is no farther than the pivot, and says whether its answer is a surface.
4. `Navigator::move_towards`: zooming in within `ZOOM_THROUGH_DISTANCE` (10 cm) moves in steady steps and keeps the pivot that far ahead. Zooming out is unchanged.
5. A drag's first frame uses the whole way from where the button went down, so no movement is lost.
6. `NavigationSettings::show_pivot` (on as standard) shows the ring while orbiting, and while drag-zooming around the mouse.

## Consequences

- Turning, panning and zooming always relate to something on screen, at a sensible distance.
- Zooming can now go inside a model; zooming out or Z (frame selected) brings you back.
- The turning speed is unchanged (0.3° per pixel); a speed setting can come if wanted.
