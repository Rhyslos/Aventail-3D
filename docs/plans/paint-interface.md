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
| 4 | Colours tab in the shelf, used colours kept | | |
| 5 | Showing one channel, with Height and Glow views | | |
| 6 | Setting: paint tools in the left bar | | |
