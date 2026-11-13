# 0280. Variations and the dice

- **Status:** Accepted
- **Date:** 2026-11-13

## Problem

Phase 12's third extra (milestone 18, [decision 0269](0269-material-extras.md)): save a graph's "shown in the painter" sliders as named looks (Clean, Mossy, Ruined) to flip between, let the painter and a shape given the material pick one, so one graph gives several looks in one project; and a dice button that rolls every seed for a fresh take. The plan names no keys or mouse actions.

## Options

- **What a variation holds:** the sliders shown in the painter only; or those plus every seed (each box's and the graph's own) and the moment of the animation baked (milestone 15); or also which boxes are bypassed (milestone 16), or the Tiling (decision 0270).
- **Where variations live:** in the graph (saved, copied and undone with it), or beside the graphs in the project.
- **Using one:** a new kind of edit, or the plain setting edits it stands for, made as one undo step.
- **The dice's numbers:** from the clock (a different take every time, but tests and replays can't repeat it), or from a roll number worked out from the graph and a count of rolls (the same roll always gives the same seeds).
- **Controls:** a key for the dice (none is free and common for this), or buttons only.

## Decision

- **A variation holds the shown sliders, every seed and the moment, not bypass.** Seeds are part of a look (the dice make a look you then keep), and the moment is part of what the painter bakes. **Bypass is left out:** it is for comparing while you work, and a variation that quietly switched boxes off would be easy to forget and confusing in the painter, where boxes can't be seen. **Tiling is left out too:** it belongs to each layer and each shape (decision 0270), not to the graph. A seed setting is held even at its standard, so using a variation always puts it back. Own boxes' seeds are held and rolled through their sliders when a seed is one of them (`matgraph::seed_keys`).
- **In the graph:** `Graph::variations` (`matgraph::Variation`: name, values by box and setting key, seed, moment; ids from the graph's one counter, at most `MAX_VARIATIONS` 100, each at most `MAX_VARIATION_VALUES` 8192 values). Three edits: `AddVariation`, `ChangeVariation` (rename or update) and `RemoveVariation`, each one undo step. `Graph::variation_now(registry, name)` makes one from the graph as it is.
- **Using one** is the setting, seed and moment edits it stands for (`Graph::variation_edits`), run together as one undo step ("Use the variation Mossy"); boxes and settings no longer there are skipped, values out of range brought into range. `Graph::variation_in_use` says which variation the graph looks as now, shown on the bar ("Variations: Mossy") and in Attributes ("In use").
- **The dice** (`Graph::rolled(registry, roll, only)`) give every seed setting of every box and the graph's own seed a new number from 0 to 999 999 (the Seed rows' range), each from the roll, the box's id and the setting's key, so the same roll on the same graph always gives the same seeds. The roll comes from the graph's seeds and a count of rolls in this run (`Graph::roll_from`), so rolling again after an undo gives another take. One undo step. A box's right-click menu has **Roll its seed** (that box only; greyed when it has no seed).
- **The painter:** a graph fill (`paint::GraphFill::variation`) picks a variation or none ("As the graph is"); it runs the graph as the variation makes it look, then its own values on top. A variation that is deleted leaves the layer showing the graph as it is (the picker says so). Saving or changing other variations doesn't make layers again: a layer is made from the graph without its variations, and its made key leaves out the graph's id counter.
- **Controls: buttons only, no keys** (decision 0028). The canvas bar gains a **Variations** menu (each variation, a click uses it; **Save as a new variation**) and a **dice** button (a small die, its name and hover say "Roll the dice"); Attributes, with nothing selected, lists the variations (name field to rename, **Use** or "In use", **Update**, **Delete**), with **Save as a new variation** and **Roll the dice**; the Paint layers panel's graph fill settings and Attributes → Materials (a material made by a graph) have a **Variation** drop-down. Inside your own box there is no Variations menu (variations are the outer graph's looks); the dice still roll the seeds inside.
- **Saved** in file format 105 (`fileio::project::v105`): `GraphV95::variations` and `GraphFillV97::variation`, stored only when there are any; graph files carry them too. Copying objects into another tab copies their graphs with their variations.

## Consequences

One graph gives several looks in one project (clean walls here, mossy there), and the dice make trying a different take a single click, safely undone. Variations need nothing to show in the painter: seeds and the moment alone make them useful. The canvas bar is fuller again, so the find field narrows when the bar is crowded. A later milestone could add bypass or Tiling to variations if the project lead wants them, and the library browser (milestone 29) could list a graph's variations.
