# 0014. A darker studio look, with a choice of accent colour

- **Status:** Accepted (replaces the colours in [0009](0009-theme.md); its font, readability tests and always-dark rule stay)
- **Date:** 2026-10-10

## Problem

After the first viewport and navigation, the project lead found the app "still feels a little like Blender": the bright grey gradient behind the 3D view, a grid running to the horizon, boxy panel tabs and light panels. They sent screenshots of the look they want, approved a mockup, then picked an accent colour from twelve versions of it.

## Options

1. **Tune the old theme:** keep the layout and bright viewport, adjust a few greys. Quick, but it doesn't change the feel the project lead described.
2. **A darker studio look, as in the approved mockup:** darker, flatter panels; a viewport with a horizon and a small grid; each panel with its own menu line; one accent colour, chosen by the user.

## Decision

Option 2:

- **Panels:** darker neutral greys (panels `#262626`, tab strips and bars `#222222`, palettes `#2a2a2a`, fields `#1c1c1c`, the gaps `#1b1b1b`). Tabs are plain text on a dark strip; the front tab joins its panel.
- **Menu lines inside panels:** the viewport has its own (View, Cameras, Display, Options, Panel, plus buttons you drag to pan, zoom and orbit) in place of its tab, which is hidden. Attributes and Objects have small menu lines too.
- **Viewport backdrop:** a studio sky (`#4a4a4a` overhead to `#3e3e3e` at the horizon) over a slightly darker ground (`#383838`, darkening towards your feet), drawn from each pixel's view direction so the horizon follows the camera. The grid is finite: it fades out around the point the camera looks at, and reaches farther as you zoom out. A green Z line rises from the origin. The test cube is a mid grey.
- **Labels over the view:** the view's name, the camera's name, an axis marker, and the grid spacing.
- **Accent colour:** marks the active mode or tool (a filled square behind a white icon), the chosen row in a list, the layout in use and selected text. **Red is the standard**, with Blue, Marine green, Plum, Olive and Petrol blue as choices (Window → Accent colour), remembered between sessions. Every accent is tested for readable text and visible marks.
- **Layout tabs** on the menu bar (Standard, Modelling, UV Edit). Only Standard exists; the others are greyed out until their layouts arrive.
- **No other program's name** appears in the app, its code or the manual. Navigation schemes are named by what they do (decision [0015](0015-navigation-schemes.md)).

## Consequences

- The colours and the accent still live in one file (`ui/src/theme.rs`); the accent is read from egui's memory wherever it is drawn.
- The red accent sits close to the X axis's red; the axis letters always show their names, so they stay easy to tell apart.
- The finite grid needs the point the camera looks at, so the renderer now receives it each frame.
