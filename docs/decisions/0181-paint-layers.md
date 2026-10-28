# 0181. The Paint layers panel and the Paint layout

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's fourth milestone ([0177](0177-texture-painter-plan.md)): managing a texture set's layers as Substance Painter does: paint and fill layers, opacity, show and hide, order, blend modes, merge down, clipping and groups. It decides how layers mix, how they are found and changed, where the panel lives, and which layer the brush paints on.

## Decision

- **Layers** (`paint::Layer`): a layer holds paint (pixels), a **fill** (one colour everywhere) or a **group** (layers of its own, bottom first, mixed together then laid on as one; nested up to 8 deep). Every layer has a name, shown or hidden, opacity, blend mode and **clipped**: shown only where the nearest unclipped layer under it is painted, scaled by that layer's coverage and opacity (as Photoshop's clipping masks and Substance Painter's anchor-style use). A hidden base hides what is clipped to it. A group remembers whether its layers are listed.
- **Blend modes** (`paint::blend`): Normal, Multiply, Screen, Overlay, Add, Subtract, Colour and Hue, by the W3C compositing formulas paint programs follow: where nothing is under a layer, its own colour shows.
- **Layers are found by path** (their place in the set, then in each group) and changed by `TextureSet` functions (`insert_layer`, `remove_layer`, `duplicate_layer`, `move_layer`, `group_layer`, `ungroup`, `merge_down`), each giving a new set the panel hands to the document with `Command::SetTextureSet`: one undo step per change, a slider's drag or a name's typing merged into one.
- **Merge down** mixes the layer into the one under it as shown, into one paint layer with the lower one's name, blend and clipping. The strokes kept with either are dropped: merged pixels can't be painted again stroke by stroke, so later milestones (paint after modelling, other texture sizes) carry them as pixels.
- **The Paint layers panel**: the material chooser (the selected objects' materials, then any with paint), buttons to add paint and fill layers and groups above the chosen one, and to move it up and down, duplicate, merge down, group or ungroup and delete it; the chosen layer's name, opacity, blend mode, clipping and (for a fill) colour; then the list, top first, each row with its eye, its kind (a brush mark, the fill's colour, a group's arrow), indented in groups, with ↳ when clipped and its blend mode and opacity when not standard. Moving is with the Up and Down buttons, within a group (Group and Ungroup move layers in and out); dragging rows comes later if wanted.
- **The brush paints on the chosen layer.** With none chosen, the topmost paint layer; with a fill or group chosen, a new paint layer is made above it and chosen. Painting on a material shows it in the panel.
- **The Paint layout** (mockup A, chosen in the plan): entering Paint mode switches to it, and leaving puts back the layout before: the viewport and the UV Editor side by side, Coordinates under them (the brush bar takes its place in milestone 8), Attributes on the right over the Paint layers with Objects as a tab beside them. It is a third tab on the menu bar (**Paint**), keeps its own arrangement and is remembered between sessions; choosing it enters Paint mode, choosing another layout leaves it.
- **Saving:** project format 66 adds each layer's clipping, a group's open state, a fill's colour and a group's layers; a version 65 file reads as before. Blend modes are saved by number (0 to 7); an unknown one is refused as from a newer version.

## Consequences

- Shading, tints and patterns can be laid over a part without painting over its edges (clip a layer to it).
- A fill layer covers the whole texture until masks (milestone 5) shape it; clipping shapes it now.
- The layout switch moves panels when entering Paint mode: wanted by the plan, but noticeable; the Standard layout comes back as it was.
