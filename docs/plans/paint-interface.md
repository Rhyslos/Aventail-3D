# Paint interface overhaul plan

The working plan for reworking the Paint mode's interface after Phase 11 ([decision 0213](../decisions/0213-paint-interface-overhaul.md)). Agreed with the project lead on 2026-10-29 after seven mockups and research (the Design artifact "Paint workspace", boards "Final · floating tools bar" and "Final · tools in the left bar").

The standing OK from Phase 11 carries on: each pull request is merged once its checks pass, one piece per pull request, and each adds its test steps to the testing document ("Aventail 3D: everything to test", section 17).

## The layout it ends with

- **Top of the viewport, floating:** colour square (click: picker pop-up), brush (click: the shelf's brushes), Size, Flow, Hardness, Spacing, then Mirror, Pen, Smoothing.
- **Bottom of the viewport, floating:** the shading and navigation buttons already there, with the paint tools between them (paint, eraser, projection, polygon fill, smudge, clone).
- **Under the views:** the brush shelf, with tabs Brushes and Colours (and Coordinates behind).
- **Right:** a Paint tab in front of Attributes (the rest of the brush, and which channel the views show), the Paint layers under them.
- **Setting:** the paint tools in the left bar instead of floating at the bottom.

## Milestones

| # | Milestone | PR | State |
|---|---|---|---|
| 1 | The Paint tab beside Attributes | see the PR list | built |
| 2 | Floating brush bar, colour pop-up | see the PR list | built |
| 3 | Paint tools floating at the viewport's bottom | see the PR list | built |
| 4 | Colours tab in the shelf, used colours kept | see the PR list | built |
| 5 | Showing one channel, with Height and Glow views | see the PR list | built |
| 6 | Setting: paint tools in the left bar | see the PR list | built |
| fix | One painting area: the bar and the tools over the viewport and the UV Editor together ([0215](../decisions/0215-one-painting-area.md)) | see the PR list | built |

### Part 2: the Paint page like the mockup ([0223](../decisions/0223-paint-page-like-the-mockup.md))

The project lead, 2026-10-29: the interface isn't finished until the Paint page looks like the mockup's board "Final · shelf at the bottom". Choices: the layout tabs stay where they are on every page; the status bar stays; the grid off while painting with a switch; one view while painting; move, rotate and scale kept in the tool row.

| # | Milestone | State |
|---|---|---|
| 7 | Page clean-up: no modelling toolbar or mode column, no view labels, the grid off (Grid switch), one view | built |
| 8 | The tool row along the viewport's bottom: ☰ (the view's menus) · 3D (the view) · Lit (shading and preview) · Show (edges, grid, cavity…), then select, move, rotate, scale, then the paint tools (brush, eraser, projection, polygon fill, smudge, blur, clone, fill, picker); the view's menu line, buttons and dice go ([0224](../decisions/0224-paint-tool-row.md)) | built |
| 9 | The brush bar's look: two colour squares, the tip drop-down with its picture, fill bars for Size, Flow, Hardness and Spacing, icon switches for mirror, pen and smoothing | next |
| 10 | A plain UV Editor while painting: its name and the object's (UV · Clay pot), no menu line, no 0 and 1, no notes ([0225](../decisions/0225-plain-uv-editor-while-painting.md)) | built |
| 11 | The right side: Objects and Paint layers on top, Attributes and Paint under them; layer rows with colour and mask pictures, blend mode and opacity; + Paint, + Fill, + Group, ⋯ (the rest) | planned |
| 12 | The Paint tab: channel buttons with coloured dots; folding sections Tip (picture, shape, angle), Size is, Paint on, Lays down, Pen and smoothing, Symmetry and trim, Paint through and effects | planned |
| 13 | The shelf: Brushes, Colours, Materials, Smart masks, Stencils; filters and search; round brush pictures with their names | planned |
