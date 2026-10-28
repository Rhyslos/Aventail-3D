# 0183. Colour: the colour picker, picking and palettes

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's sixth milestone ([0177](0177-texture-painter-plan.md)): choosing colours well, which hand-painted work lives on. The project lead asked for a picker in the Attributes panel built on Nodelit's (and found the mockup's too big), Substance Painter's P to pick colours from the model, and palettes kept with the project and as files.

## Decision

- **The colour picker** (`attributes::colour_picker`) sits in Attributes under the brush (and the polygon fill) in Paint mode, replacing the plain colour button:
  - **Wheel:** a hue ring around a saturation and value square, 128 points across (smaller than the mockup's). **Sliders:** hue, saturation and value. The hue is kept while the colour is grey or black, so turning saturation back up returns to it.
  - **New** over **Before** (the colour last painted with; click it to go back), the colour's **lightness** as a percentage (a value check), and **Pick (P)**.
  - The colour as **hex** (typed, `#RRGGBB` or `#RGB`) and **R G B** numbers, and **Copy** (hex to the clipboard).
  - The **palette** row: a chooser of the project's palettes (with New palette, Import palette…, Export palette… and Delete palette), its swatches (click to use, right click to remove) and **+** to add the colour (a palette is made if there is none).
- **Picking (P, Substance Painter's key):** P takes the colour under the mouse at once, on the model or in the UV Editor; **Pick (P)** waits for a click there instead. It takes what the surface shows: the material's painted layers mixed over its own colour, averaged over 3 × 3 texels so a single speck doesn't decide.
- **Palettes** (`paint::palette::Palette`: a name and up to 1024 colours) are kept with the project's paint (`PaintStore::palettes`, up to 64) and changed with `Command::SetPalettes`, one undo step each. They are read from and written to **GIMP palette files** (`.gpl`), a plain text format Krita, Inkscape, Aseprite and GIMP share.
- **Saving:** project format 68 adds the palettes; a version 67 file opens with none.

## Options considered

- Nodelit's triangle centre and its Detailed mode's honeycomb were left for later (the square and the HSV sliders, which the plan chose for Detailed, are here). A perceptually even wheel (OKLCH, the mockup's "Even") was left for later too: the plain HSV wheel is what Nodelit and most programs show, and the lightness readout covers the value check.
- Harmony swatches and the light-and-shade row from the mockup come with the light-and-shadow helper (milestone 18).

## Consequences

- In Paint mode P picks colours, as the painter's keys win there (the project lead's choice for 1 to 6); elsewhere P still switches the view between perspective and orthographic, which in Paint mode stays in the view's Cameras menu.
- Palettes travel with the project and can be shared with other paint programs.
