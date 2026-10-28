# 0174. Modifiers in the Objects panel only

- **Status:** Accepted (the project lead's choice)
- **Date:** 2026-10-28
- **Replaces part of:** [0168](0168-modifiers-in-the-lists.md) (modifiers listed in both the Objects and the Edit layers panels)

## Problem

With modifiers listed as children of their shape in the Objects panel and again after the layers in the Edit layers panel, the project lead found Edit layers "a little confusing": two places to reach the same thing.

## Options

1. **The Objects panel only** (recommended): one home for modifiers; Edit layers back to layers and edits.
2. The Edit layers panel only.
3. Edit layers merged into the Objects panel, layers as children too, and the Edit layers tab gone.

## Decision

Option 1, the project lead's choice. The Edit layers panel lists the selected shape's layers and their edits only; modifiers are under their shape in the Objects panel, with everything decision 0168 gave them (click, tick, drag, Delete, right-click).

## Consequences

- One place for each thing: edits in Edit layers, modifiers in Objects.
- The Edit layers panel no longer shows that modifiers come after the layers; its manual page says so.
