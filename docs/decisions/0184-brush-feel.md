# 0184. Brush feel: pen pressure, smoothing and lines

- **Status:** Accepted
- **Date:** 2026-10-28

## Problem

The texture painter's seventh milestone ([0177](0177-texture-painter-plan.md)): painting with a pen should feel like painting. The project lead uses a Wacom tablet and chose Wacom first. It decides where pressure comes from, what it changes, and how strokes are steadied.

## Decision

- **Wintab first** (`app::tablet`, Windows): the app loads `Wintab32.dll` at run time (through `wintab_lite`, MIT, and `libloading`, ISC), opens a system context for the window and reads its packet queue every frame (winit doesn't pass Wintab's window messages on). The latest packet gives the pressure (scaled by the tablet's top reading), the tilt (from azimuth and altitude) and whether the eraser end is down. No driver, or another system: no tablet. Wintab is opened once; the context closes with the app.
- **Other pens through the system:** a pen Windows passes on as touch with force (egui's `Event::Touch { force }`) gives the pressure when there's no Wintab pen.
- **A mouse** has no pressure: it paints at full size and flow (the mouse defaults).
- **What pressure changes** (`ui::pen::PressureCurve`): the **size** (on by default, as in Substance Painter) and the **flow** (off by default), each with a least share at the lightest touch and a curve: **Soft** (light touches paint well), **Even**, **Firm** (powers 0.6, 1 and 1.7). Pressure is blended along a stroke between frames, dab by dab. Flow pressure is each dab's **strength** (`paint::Dab::strength`): within a stroke the texel still keeps its largest coverage.
- **The eraser end** erases for that stroke without changing the brush.
- **Smoothing** (**D**, Substance Painter's lazy mouse): the brush follows the pointer on a string up to 60 points long at 100%; strokes come out steadier. Off by default, 30% when turned on.
- **Shift + click** (Substance Painter's) paints a straight line from where the last stroke ended (in the same view, or in the UV Editor), then carries on as a stroke while the button is held.
- **Kept with the strokes:** each dab's strength (project format 69; left out when all are full); its radius already includes the pressure.
- Attributes shows **Pen and smoothing** under the brush, with the pen's pressure now (or that a mouse has none).

## Consequences

- Wintab code is Windows-only; it's built and linted by the Windows CI job. On the project lead's machine, the Wacom driver's own Windows Ink setting doesn't matter: Wintab is read first.
- Tilt is read but not used yet: tip brushes (milestone 8) can turn with it.
- Ctrl + left drag's up and down (brush rotation, in the plan) comes with the tip brushes too.
