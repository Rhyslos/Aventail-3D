# 0270. Tiling, and Transform and Repeat on colours

- **Status:** Accepted
- **Date:** 2026-11-12

## Problem

Milestone 8b of Phase 12 ([the plan](../plans/phase-12.md), [decision 0269](0269-material-extras.md)). The project lead asked: "make a scale/tiling for the texture, like making the bricks 4 times smaller on this starter". Until now the only ways were lowering Material out's **Real-world size** inside the graph (which changes the graph for every shape using it, and keeps lengths in centimetres, such as the mortar's width, the same) or the Bricks box's rows and across. Transform and Repeat, the boxes that move and copy a pattern, worked on heights only, so a coloured pattern or a finished material couldn't be turned or copied smaller.

## Options

**Tiling, where it lives:**

- On the graph (a setting of Material out): one size for every shape using it. Rejected: the point is to use the same graph at different sizes on different shapes without opening it.
- **On the graph fill layer** (chosen): `paint::GraphFill::tiling`. A material made by a graph is a texture set holding one graph fill (decision 0262), so the same value serves both places the project lead named: **Attributes → Materials** (under "Made by the graph …") and the **Paint layers** panel's graph fill settings.

**What Tiling changes:**

- As lowering the real-world size: the pattern gets smaller but lengths in centimetres stay (mortar as wide as before, so thicker against smaller bricks). Rejected: that isn't what "4 times smaller" means, and every other 3D app's tiling scales the texture evenly.
- **The repeat laid more often** (chosen): the picture of one repeat is exactly what the Material mode shows; it is laid `repeats × tiling` times across the model's texture square. Everything in it gets smaller together.
- **From the model boxes** (Curvature, Occlusion…) follow the model, not the repeat, so they **stay where they are**: moss in the crevices stays in the crevices; only the repeating parts shrink. Lengths in boxes working on the model's square stay real centimetres on the model (`OnModel::square_cm` ignores tiling).

**The control:** a slider named **Tiling** from **×⅛ to ×32** on a doubling scale, shown as ×½, ×1, ×2, ×4…, snapping to the doublings when within a small step of them so the common sizes are easy to hit, with a ↺ back to ×1; a value can be typed (4, ×4, 1/2). No keys or mouse actions added.

**Transform and Repeat on colours and materials:**

- A plug whose kind follows what is plugged in. Rejected for now: every place that reads a plug's kind (wire colours, connection checks, the box search, own boxes) would have to work kinds out from the graph.
- New boxes ("Transform colour"). Rejected: the project lead asked for Transform and Repeat to do it.
- **More plugs on the same boxes** (chosen): each takes a **Height** (the released `in` and `out` keys, unchanged), a **Colour** and a **Material**, and moves each one plugged in the same way. None is needed on its own; the box needs one of them (`BoxType::needs_an_input`) and says so with ways to fix it when nothing is plugged in. An output whose input is empty gives nothing (`BoxType::run_outputs`, `BoxResult::outputs` now `Option`s), and a box wired to it says why, instead of the evaluator making and storing flat pictures nobody uses.

**A material moved:** every map moves the same way. The normal map turns with it: a quarter turn turns its normals a quarter, a flip mirrors them, and Repeat's smaller copies have slopes steeper by as much as they are smaller (and gentler where Vary makes a copy lower). Occlusion and curvature move as they are. Repeat's Vary darkens a copy's colour and lowers its height.

## Decision

- **matgraph:** `boxes::moving` (moving any kind; the shared plugs); Transform and Repeat take and give a height, a colour and a material; `BoxType::run_outputs` and `needs_an_input`; `BoxResult::outputs: Vec<Option<Value>>`; the evaluator's refusals for nothing plugged in and an empty output; such boxes are never offered as makers. `OnModel::tiling` (`new`, `with_tiling`): `repeats()` includes it, `square_cm` doesn't.
- **paint:** `GraphFill::tiling` (`MIN_TILING` ⅛, `MAX_TILING` 32, `usable_tiling`), checked in `is_valid`.
- **bake:** `graph_fill(…, repeats, tiling)`.
- **ui:** `material::tiling` (the slider: `control`, `snapped`, `shown`, `parsed`); the Tiling row in Attributes → Materials and in a graph fill's settings; the graph fill's made key includes it; the Material mode's Model preview lays the material as often as the selected model's material is tiled (`giving::tiling_on`, `laying::tiled`), so the preview, the viewport, the painter and the texture export agree.
- **File format 99** (`project/v99.rs`): a graph fill's Tiling, 1 in older files; out of range brought within the limits, not a number refused. Sample `tests/data/sample-v99.avtl`.

## Consequences

- One graph serves a big wall and a small crate at the right size each, without copies of the graph.
- Old graphs open and work out exactly as before (tested against the old maths); the Bark starter's Transform box is taller, so its file's layout moved, but its pictures are unchanged.
- Exporting a graph on its own (Export textures…) writes one repeat, as before: Tiling is how often a shape's material lays it, so it is baked into a model's textures, not into a graph's texture set.
