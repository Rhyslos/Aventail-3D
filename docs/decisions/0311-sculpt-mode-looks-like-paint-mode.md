# 0311. Sculpt mode looks and works like Paint mode

- **Status:** Accepted; the four points are the project lead's (2026-11-18), the further details were chosen by Claude while the project lead was away and wait for review in [docs/plans/phase-17.md](../plans/phase-17.md#choices-to-review) (48 to 55)
- **Date:** 2026-11-18
- **Follows:** [0304](0304-sculpt-mode-and-layers.md) (Sculpt mode), [0181](0181-paint-layers.md), [0185](0185-brush-bar-and-tips.md), [0213](0213-paint-interface-overhaul.md), [0223](0223-paint-page-like-the-mockup.md), [0224](0224-paint-tool-row.md), [0227](0227-paint-layers-on-top.md), [0228](0228-paint-tab-sections.md) (Paint mode's page)

## Problem

After symmetry and masks (17a.4), Sculpt mode still looked like the modelling page with brushes added: the modelling toolbar, the mode column, the modifiers palette, the view's menu line, labels, grid and dice, the brush bar under the menu line, the Sculpt tab over Objects and Edit layers, and the tab as one long list. Paint mode had been made calm and plain over seven pull requests. The project lead asked on 2026-11-18 for Sculpt mode to look and work like Paint mode, and chose four points:

1. **The right side as Paint's:** the Sculpt layers (Edit layers) on top with Objects as a tab beside them; the Sculpt tab underneath with Attributes beside it, the Sculpt tab in front. Saved Sculpt layouts from before still open.
2. **The plain page as Paint's:** the modelling toolbar and mode column step aside, a quiet slate accent whatever the chosen one, no selection outline, no names or grid (Grid in the tool row's Show list, remembered), one view (a middle click doesn't split it into four).
3. **Folding sections in the Sculpt tab** as the Paint tab's: each section's name with what's set inside on the right ("pressure → size", "Mirror X", "off"), open or closed remembered.
4. **The viewport's look, with its tools:** the view's tool row and floating brush bar placed and styled as Paint's (the same tool row, Show list, brush bar position and look, the left bar only for the sculpt tools when Settings puts them there), and the view drawn as Paint draws it apart from the matcap (the backdrop, the page's light, no edges).

## Options

The four points were chosen; the ways to build them:

1. **Share Paint mode's code** where it is clean: the layout's arrangement, the plain-page switch, the tool row's drawing and placing, the floating bar's rows, the folding sections; or copy it into the sculpting files (quicker, but two copies drift).
2. **The tool row's buttons:** select, move, rotate and scale, then the brushes with a number key; or every brush (fourteen buttons after the transform tools); or no select arrow.
3. **The Look button while sculpting:** the matcaps (Clay, Red wax, Pale stone, Lit) in its list, as the paint previews are in Paint mode's; or matcaps only in the Sculpt tab.
4. **Old saved Sculpt layouts:** replaced once by the new arrangement, as old Paint layouts were (decision 0227); or kept as they were.

## Decision

- **Shared, not copied** (option 1, the first way). `layout::brush_page_layout` arranges both the Paint and the Sculpt layouts (Paint's with the UV Editor beside the view); `Workbench::plain_page` is true in Paint and Sculpt mode and drives the toolbar, mode column, modifiers palette, accent and the viewport's `set_plain_page` (renamed from `set_paint_page`), so one view, no menu line, labels, dice or grid, and no selection outline; `paint_tools::RowButton`, `show_row` and `column_of` draw any tool row and place it after the view's controls; `paint_bar::floating` lays any brush bar's groups in rows; `attributes::folds::fold` draws the Sculpt tab's sections.
- **The Sculpt layout:** Objects and Edit layers on top (Edit layers in front), Attributes and the Sculpt tab under them (the Sculpt tab in front), the shelf under the view with Coordinates behind it. A saved Sculpt layout with Edit layers still under the Sculpt tab and Attributes (from before this decision) opens in the new arrangement once; one in the new arrangement, however changed, is kept (option 4, the first way).
- **The tool row** (`sculpt_tools`): **Select (Q)**, **Move (W)**, **Rotate (R)**, **Scale (S)**, a line, then **Draw (1)**, **Clay strips (2)**, **Inflate (3)**, **Crease (4)**, **Flatten (5)**, **Grab (6)**, **Snake hook (7)** and **Mask (9)** (option 2, the first way); the rest stay on the shelf, in the bar's brush list and in the tool menu (E). With Settings' paint tools in the left bar, the same buttons go down the left bar instead. **Q turns the select arrow on**, as in Paint mode: clicks select (so another shape can be picked to sculpt) and nothing is sculpted; a brush's key or button puts it down.
- **The brush bar** sits at the view's top (no menu line above it) in Paint's look: the brush as a button with its picture and a list of every brush, Size, Strength and Spacing as bars, switches for **Symmetry** (with its axes, L), the **pen** (pressure on size and strength together) and **smoothing** (D), and a fourth group with Falloff, Stroke and Alpha, which goes to a second row when the view is too narrow. The Sculpt tab leaves out the number bars while the bar shows, as the Paint tab does.
- **The view's controls** are Paint's ☰, 3D, Lit and Show. While sculpting the Look button names the matcap and lists the matcaps (option 3, the first way); the Show list has Grid (the same remembered switch as the Paint page's), the sculpt's own **Cavity**, and the other overlays, but no Edges (the forms are drawn without edges, as before).
- **The Sculpt tab** has the sculpt's status, the brush's name, then folding sections: **Brush** (open; the falloff on the right), **Stroke and alpha**, **Symmetry** ("Mirror X", "radial 8" or "off"), **Mask** (the mask tool), **Pen and smoothing** ("pressure → size, pressure → strength", "smoothing" or "off"), **Look** ("clay, cavity") and **Keys for sculpting**, closed to start with. Open or closed is kept while the app is open, as the Paint tab's.
- No new file format. One key changes meaning in Sculpt mode: **Q** (the select arrow, as in Paint mode, where before it brought the brush back after W, R or S). No mouse actions are added.

## Consequences

- The two brush pages read as one design: what's learnt painting carries over to sculpting, and the model and its forms are what shows.
- Paint and Sculpt mode now share the code that makes the plain page, the tool row and the bar, so a later change to Paint's page reaches Sculpt's too; a change meant for one page must check the mode.
- The modelling toolbar's tools are a menu away (the Mesh and Select menus, and command search, still work); the mode column's buttons too, as in Paint mode (the layout buttons and Esc leave).
- The bar's fourth group usually wraps to a second row on a 1440-pixel-wide screen; if that proves busy, Falloff, Stroke and Alpha can leave the bar for the Sculpt tab (choice 52).
