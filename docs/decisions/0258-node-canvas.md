# 0258. The node canvas and the Material mode

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 12's second milestone ([the plan](../plans/phase-12.md), [decision 0256](0256-material-graph-plan.md)) puts the material graphs of milestone 1 ([decision 0257](0257-material-graphs-core.md)) on screen: a Material mode, a canvas to build graphs on, a preview, and box settings in Attributes. The project lead had already chosen the controls and the look (the mockups: live pictures, coloured wires, a tidy left-to-right layout, comment frames, plain-words notes when something won't work). What was left to choose was how it fits into the app.

## Options

- **Where the canvas goes:** a panel of its own in a Material layout (chosen), or a big window over the app. A panel keeps the app's way of working: it can be moved and resized like any other, and the layout is remembered.
- **Where the graphs are worked out:** on the interface thread (simple, but a slow graph would stall the window) or on a thread of their own (chosen). The interface never waits: it sends the graph and draws the last pictures that came back.
- **How the layout is kept tidy:** moving boxes only when asked ("Tidy up" button), or after every change (chosen, as the plan says). Boxes snap into columns, a box is pushed right of everything feeding it, boxes sitting on each other are pushed apart, and a frame grows to keep its boxes. The tidying is part of the same undo step as the change.
- **Box groups' names:** the engine's group for whole-material boxes is now listed as **Finish**, the last step of the working order, as the plan names it.

## Decision

- **A Material mode** after Paint on the left palette, and a **Material** layout tab. Entering the mode switches to the layout and leaving puts the one before back, as Paint does. The layout:
  - the **Material graph** panel (the canvas) in the main area, with the viewport as a tab behind it;
  - the **Preview** beside it, over **Attributes**, with Objects and Coordinates as tabs beside Attributes.
- **The canvas:**
  - boxes with live pictures, plugs and wires coloured by what they carry (shape grey, height amber, colour blue, whole material green);
  - a red "!" on a box that can't run, with why;
  - comment frames that stay readable zoomed out;
  - a bar above it to choose, make, rename and delete graphs.
- **Controls**, exactly as the project lead chose (decision 0256): drag between dots to connect, drag a wire's end away to disconnect, middle drag pans, the wheel zooms, Z frames, click / Ctrl / Shift and a drawn box select, Delete removes, Backspace removes and keeps the wire through, C makes a comment frame, Space or right-click opens the box search, clicking a box's picture looks through it.
  - The canvas takes these keys only while the pointer is over it, before the rest of the app, so Space doesn't open the command search there and Delete doesn't delete objects.
  - **Alt + C / V** paste plain copies for now and say so: linked boxes come with your own boxes (milestone 7).
  - **Alt + G / Shift + G** say that making your own box comes later in the phase.
- **"Why it won't work":** while a wire is dragged over a plug that won't take it, the plug turns red, a note follows the pointer with the engine's reason and fixes, and the status bar repeats it. Letting go cancels the wire and leaves the note, so a fix can still be clicked; nothing changes until one is. Other refusals (a second Material out, a frame that can't be kept, a box that can't run) show the same note.
- **The box search** lists boxes in the working order (Make, Shape, Detail, Colour, Finish), "Usually comes next" first. Dropping a new wire on empty canvas opens it with only the boxes that fit, and connects the box picked.
- **Pictures** are worked out on a thread: at 256 pixels while anything is held down, at 512 after. A box's picture is made into a texture again only when it changed.
- **The preview** shows the material (what Material out is given) on a lit ball or flat and tiled twice each way, or what a box makes when its picture is clicked. It is drawn from the pictures the canvas already has.
- **Attributes** shows the chosen box's settings, "What does this do?", its reason and fixes when it can't run, and buttons for the boxes that usually come next. A chosen frame shows its title, colour and note; with nothing chosen, the graph's name and seed.
- **Every change is one undo step** (`scene::Command::EditGraph`), the tidy layout's moves included. Drags merge into one step; Esc during a drag takes it back.
- **Found in the Keys window** (a Material mode colour) and in **command search** (New material graph, Add a box, Comment frame).

## Consequences

- The preview shows a ball and a flat square. The model and a cube, with the app's looks, come with milestone 4 (Preview and checks), where the seam check and the texture budget go too.
- Nothing new is saved: graphs (with their frames and box places) were already in the project file (version 95). The canvas's pan and zoom aren't saved.
- Later milestones add boxes (milestone 3) without touching the canvas: the box search, Attributes and the plugs come from each box kind's information.
- Pictures for very big graphs may take a moment; the bar above the canvas says "Working out the pictures…" meanwhile, and the window never waits.
