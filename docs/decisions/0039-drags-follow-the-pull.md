# 0039. Drags follow the pull

- **Status:** Accepted
- **Date:** 2026-10-13
- **Follows:** [0030](0030-transform-gizmos.md), [0038](0038-extrude-and-inset.md)

## Problem

Extrude and Inset first used a fixed rule: drag right for more, left for less. When the project lead tried it, it felt rigid. Dragging a polygon on the left side of a shape to the left pushed it *in*. He asked for one rule across the whole app: where a drag changes something, the direction of the pull should decide what happens.

## Decision

**A guiding principle (DESIGN.md §1, principle 8) and a ground rule (CONTRIBUTING.md, rule 16):** a drag in the viewport is measured from the thing it changes (its facing, its axis, its middle), so what you pull follows the mouse. A fixed screen direction ("right for more") is only the fallback, used when there is no way to follow.

How each drag follows it today:

| Drag | Follows | Fallback |
|---|---|---|
| Move gizmo (arrows, squares, centre) | The axis or plane under the mouse | — |
| Rotate gizmo (rings) | The turn around the gizmo's middle on screen | — |
| Scale gizmo (boxes, squares) | The axis; the plane by the drag's direction | — |
| Scale gizmo's uniform centre | — | Right or up grows: a drag that starts on the middle has no way to follow |
| Extrude (polygons, border edges) | The way they face on screen, scaled so they follow the mouse | Right means out when they face the viewer straight on |
| Inset | Towards the polygons' middle on screen, from where the drag began | Right means more when the drag starts on the middle |
| Bevel (edges, points) | Away from the selected edges or points on screen, from where the drag began | Right means more when no edge can be seen |
| Tool menu flick (E) | The direction of the flick | — |

Typed and dragged numbers in panels (the Attributes and Coordinates panels) aren't viewport pulls and keep the usual "right or up for more".

Future tools say in their pull request how they follow the pull. Bevel, for example, will widen as you pull away from the edge.

## Consequences

- One habit works everywhere: pull the thing where you want it to go.
- Each tool has to decide what "towards" means for it (a facing, an axis, a middle) and what its fallback is. That is a little more work per tool, and tests check both.
