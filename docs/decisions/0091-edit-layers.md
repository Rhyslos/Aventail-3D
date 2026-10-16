# 0091. Edit layers and the modifiers palette

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0006](0006-base-plus-operation-stack.md) (base plus operation stack), [0044](0044-edits-that-hold.md) (edits that hold), [0090](0090-stable-element-names.md) (stable names); DESIGN.md §5.1 (the modifiers palette) and Phase 6 (W5)

## Problem

Every object has kept its edits in layers from the start (0006), but there was only ever one, "Edits", and no way to see or manage it. 3ds Max users stack edit layers: a set of edits kept apart, switched off to compare, moved, collapsed, deleted. Phase 6 brings that, and the modifiers palette the project lead chose (2026-10-12) arrives with it, as its first button. The edit list inside each layer (opening and changing any edit) is the next milestone; this one is the layers themselves.

## Options

- **Where the layers are shown:**
  - **an Edit layers panel, a tab beside Objects** (chosen): the right column stays two panels high, the tab can be dragged anywhere, and the Edit layer button brings it to the front;
  - a sub-list under each object in the Objects panel (DESIGN's first sketch), which would crowd the tree once objects have several layers and, in the next milestone, their edits;
  - a third panel in the right column, which leaves Attributes and Objects too short on a laptop screen.
- **What the Edit layer button does** (the project lead was asleep, so the recommended choice was taken, [0028](0028-ask-about-controls.md)):
  - **adds an empty layer on top of each selected object** (chosen): what 3ds Max's "add Edit Poly" does, and the only kind of modifier so far;
  - opens a menu of modifiers, which waits until there is more than one.
- **Which layer new edits go into:**
  - **the top layer that is switched on** (chosen); if none is on, a new layer is added on top, so an edit is never made into a layer that hides it;
  - always the top layer, which would record edits that do nothing visible while it is off;
  - a "current layer" picked by the user, which is one more thing to keep track of; it can come later if wanted.
- **Merging (Max's "collapse"):**
  - **Merge down: a layer's edits go to the end of the layer below** (chosen), only when both are on, so nothing switched off is switched on by surprise;
  - baking a layer into the shape, which is the edit list's "freeze" (next milestone).
- **Saved panel layouts** from before: **Edit layers is added beside Objects** rather than resetting the user's layout (`Panel::ADDED_LATER`).

## Decision

1. **Scene:**
   - `Command::AddLayer(ids)` adds an empty layer on top of each object with a shape of its own (groups and instances are left alone), named "Edits", "Edits 2" and so on (`Stack::next_layer_name`).
   - `Command::ChangeLayer { id, index, change }` with `LayerChange`: `SetEnabled`, `Rename` (trimmed, never empty), `Move { up }`, `MergeDown` (both layers on), `Delete` (with its edits). Each is one undo step; a change that changes nothing records nothing. They are made of the layer edits undo already had (`InsertLayer`, `RemoveLayer`).
   - `Command::Record` puts a new edit into `Stack::recording_layer()`, the top layer that is on, or a new layer on top when none is.
   - Nothing new is saved: layers' names, order and switches were always in the file.
2. **The interface:**
   - **The modifiers palette**: a thin column on the right edge of the viewport panel, beside the right column, with the **Add edit layer** button (`Icon::EditLayer`, a stack of sheets). It is not part of the command list, which is the tool menu's (0037); the Mesh menu has **Add edit layer** too.
   - **The Edit layers panel** (`Panel::EditLayers`, `layers::LayersPanel`): the selected object's layers, top first, each with an eye (on or off), its name, its edit count and a **new edits** mark on the layer new edits go into, and under it its edits, newest first, by their Edit menu names. Click a layer to pick it for the panel's **Layer** menu; right-click for the same entries (Rename, Switch off, Move up, Move down, Merge down, Delete layer); double-click to rename. **Add layer** at the top.
   - The Attributes panel's Edits line says how many layers there are when there's more than one.
   - **No new keys.**

## Consequences

- Sets of edits can be kept apart, compared by switching them off and taken back in one step.
- Moving a layer changes the order edits are made in. Its edits find their elements again by their anchors and names (0032, 0090), so most still land where they were made; one that can't says so in the Attributes panel, as before.
- The edit list (opening any edit, pin, freeze, warning badges) fills in the panel's edit rows next. Generators, booleans and the rest join the modifiers palette in later phases.
