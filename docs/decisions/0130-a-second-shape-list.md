# 0130. A second shape list on Alt + B

- **Status:** Accepted
- **Date:** 2026-10-23
- **Builds on:** [0129](0129-more-shapes.md) (eleven more shapes)

## Problem

With eighteen shapes, the B list got long, and the shapes used most were mixed in with the ones used now and then. The project lead asked for the B list to keep the shapes used most, with **Alt + B** for the others, and no shape in two lists.

## Decision

- **B** opens the seven basic shapes (cube, plane, cylinder, sphere, disc, tube, cone) as before; **Shift + B** opens it at the bottom.
- **Alt + B** opens the eleven others (icosphere to arch); **Shift + Alt + B** opens it at the bottom. Once a list is open, **B** and **Shift + B** move through that list (the project lead's choice, 2026-10-23: Alt + B, then B, B, B goes down the More list); Alt + B while the basic list is open switches to the More list.
- The Create menu keeps every shape, with a line between the two groups.
- Alt + B was free: Alt with the mouse turns the view, but Alt + B on its own wasn't bound.

## Consequences

- No shape appears in both lists.
- The tool menu's shape entry still opens the basic list; Alt + B is a key away.
