# 0168. Modifiers in the Objects and Edit layers lists

- **Status:** Accepted (the project lead's choices)
- **Date:** 2026-10-25
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack), [0091](0091-edit-layers.md) (edit layers), [0165](0165-levels-panel.md) (stable modifier ids)
- **Replaces part of:** [0142](0142-modifier-stack-and-remesh.md), where every modifier's settings, with Up, Down and Remove, sat one after another in the Attributes panel

## Problem

With several modifiers, the Attributes panel listed every one's settings one after another, each with its switch, Up, Down and Remove. The project lead found it "really messy and hard to see": the rows of one modifier ran into the next, and it was hard to tell where one ended.

## Options

The project lead was asked (decision 0028) where modifiers should be listed, what the Attributes panel should show, and which controls the lists get:

- **Where:** under their object in the Objects panel (recommended; Blender's outliner and Cinema 4D's object list do this), after the layers in the Edit layers panel, or **both**.
- **Attributes:** **only what's clicked** (recommended), or every modifier with all but the clicked one folded.
- **Controls:** drag to reorder, a tick to switch on and off, Delete to remove (all recommended), and a right-click menu.

## Decision

The project lead chose **both lists**, **only what's clicked**, and **all four controls**.

1. **Rows.** An object's modifiers are rows under it in the Objects panel, a step in, before any objects inside it, first to last (the order they're worked out in). The same rows come after the layers in the Edit layers panel, under a **Modifiers** heading. Each row has the kind's icon (as on the modifiers palette), its name, a dot when it can't do its work (amber) or has nothing to do (grey), and a tick at the end. An object with modifiers gets the fold arrow, which folds them away with its contents (`crate::modifier_list`).
2. **Click.** A click on a modifier selects its object alone and *picks* the modifier: `AttributesPanel::picked_modifier`, by object and stable modifier id (decision 0165), so it survives moves around it. While its object is the only one selected, the Attributes panel shows only that modifier: a **← *object*** button, its switch and name, its settings, Show in viewport and its note. A click on the object's own row, the ← button, a click in the viewport, or any other selection puts the pick away. The object's row is a softer shade of the accent colour meanwhile, so the picked modifier stands out.
3. **The object's Attributes** have a short **Modifiers** section instead of every modifier's settings: their names as links (struck through when off, amber when they can't work).
4. **A new modifier added to one shape is picked at once**, so its settings show straight away, as they did before.
5. **The tick** switches a modifier on or off (`Command::ChangeModifier`).
6. **Drag** a row within its object's modifiers to reorder (`Command::MoveModifier`); a line shows the gap. A drag ending where it started changes nothing.
7. **Delete or Backspace in the Model mode** removes the picked modifier, not its object (`Command::RemoveModifier`). With nothing picked, they delete objects as before.
8. **Right-click:** Switch off/on, **Duplicate** (`Command::DuplicateModifier`: a copy just after it under a new id), Move up, Move down, **Bake into shape up to here** (`Command::BakeModifiersUpTo`) and Remove.
9. **Bake into shape up to here** bakes the modifiers up to and including the one clicked, as Bake into shape does with all of them: the list is cut short for the bake without forgetting the levels of detail's changes to the later ones, the shape is baked, and the later modifiers come back, ids and all, worked out on the baked shape. Those switched off or that couldn't work stay. Up to the last, it's the same as Ctrl + E.
10. The Attributes panel loses its Up, Down and Remove buttons: ordering, copying and removing are in the lists.

## Consequences

- One modifier's settings at a time: easy to read, however many a shape has.
- The order is visible at a glance in the Objects panel, and changed by dragging.
- One new key use (Delete and Backspace on a picked modifier), in the Model mode only, chosen by the project lead.
- Reference picture `modifier_list` shows a cube with three modifiers, the last picked.
