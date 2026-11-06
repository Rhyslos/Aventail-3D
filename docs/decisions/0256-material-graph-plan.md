# 0256. Materials with a node graph: plan

- **Status:** Accepted (the project lead's choices)
- **Date:** 2026-11-06

## Problem

Phase 12, materials with a node editor, is next on the roadmap ([0255](0255-materials-before-source-import.md)) and needed its planning talk. The project lead wants Substance Designer's power without its confusing way of working, and to keep engine shaders out of the app.

## Options

Research (Substance Designer tutorials and forums, Substance 3D Sampler, Quixel Mixer, InstaMAT, Material Maker) gave four ways to offer it, shown to the project lead:

1. **Material layers** like the painter's, each a maker plus effects.
2. **A guided builder in stages** (shape, surface, fill, colour, finish).
3. **A node graph, redesigned to be friendly.** Chosen.
4. **Layers on top with nodes underneath.**

## Decision

- **A node graph**, in a Material mode. It is made friendlier than Substance Designer in these ways:
  - a live picture on every box;
  - wires coloured by what they carry, and one wire that can carry a whole material;
  - the app keeps the layout flowing left to right;
  - plain names, "What does this do?" and "what usually comes next";
  - height first;
  - a real-world size and seamless tiling.
- **Starter graphs** and **your own boxes** (a group of boxes saved as one box with sliders).
- **A shape canvas** (the project lead's idea): vector shapes drawn or cut out inside the editor. It includes assemblies with plain and linked copies, an edge randomiser with per-copy variation, and foliage output (opacity, translucency, atlases).
- **No engine shaders inside the app.** Graphs run on the app's own engine. Export bakes them to textures, and a **translator** writes a Godot shader for only the parts that move or change in the game. Unity and Unreal translators come later.
- 16 milestones in four groups. The working plan is [docs/plans/phase-12.md](../plans/phase-12.md).
- **Controls** (the project lead's choices, listed in the plan):
  - On the node canvas:
    - **Space** or **right-click** adds a box;
    - drag between dots to connect, and drag a wire's end away to disconnect;
    - middle drag pans, the wheel zooms, **Z** frames;
    - **Ctrl + C / V** copies, **Alt + C / V** makes a linked copy;
    - **Delete** removes a box, **Backspace** removes it but keeps the wire through;
    - **Alt + G** and **Shift + G** make and open your own boxes.
  - On the shape canvas:
    - **Q / W / R / S / T** select, move, rotate and scale, as everywhere;
    - **P** pen, **B** basic shapes, **Alt** while drawing cuts, **M** mirror line.

## Consequences

- Two new core crates, `matgraph` and `shapes`, with no UI or GPU, so both can be tested without a window. Evaluation starts on the CPU, and GPU evaluation is an option later.
- The painter gains graph fill layers fed by the baked maps.
- Material Maker (MIT) is a reference for pattern maths. Nothing comes from Substance.
