# 0289. The Trim sheet box: strips of materials on the project's trim rows

- **Status:** Accepted
- **Date:** 2026-11-13
- **Follows:** [0106](0106-trim-sheets.md) (the project's trim rows and Put on trim row), [0269](0269-material-extras.md) (the material extras), [0281](0281-palette-map.md) (project data reaching the graphs through the registry)

## Problem

Phase 12's eleventh extra (milestone 26, [decision 0269](0269-material-extras.md)): **build a trim sheet**, strips of different materials stacked on one texture, for walls, edges and pipes. The UV side has existed since decision 0106: the project keeps trim rows (the UV Editor's Trim menu) and **Put on trim row** lays strips of a model straight onto a row. What was missing is making the texture those rows describe: until now an artist had to paint it by hand, or make each strip's material separately and line them up somewhere else.

## Options

- **Where the strips lie.**
  - (a) Only the box's own heights: simple, but the texture and the project's rows could disagree, and strips of a model laid on a row would show the wrong material.
  - (b) **The project's trim rows, read by the box** (chosen, the standard): Strip 1 on the top row, Strip 2 on the next, so a strip of a model put on a row shows that row's material, and changing a row in the UV Editor remakes the sheet. The rows reach the graphs the way the palettes do (decision 0281): the scene hands a copy to the graphs' registry (`Registry::trim_rows`, `GraphStore::set_trim_rows`, on opening and on every `SetTrim`), boxes that read them say so (`BoxType::uses_trim_rows`) and hash their key into their result's key.
  - (c) Rows only: a project with none would have nothing to show. So a project without rows (or the **Strips on → Heights of their own** choice) shares the height by each strip's **Height** (1 to 16 parts), and Attributes offers **Make these the project's trim rows**, one undo step, so the UV side can follow.
- **How many strips.** Plugs are fixed per kind, so a box has a set number. **Eight Strip plugs** (chosen): more than most trim sheets use; a strip with nothing plugged in is left out (with own heights) or leaves its row to the gap (on rows). A ninth plug, **Between strips**, fills the gaps with a material; with nothing there they take **Gap colour**.
- **How a material fills its strip.** (a) **Cut to the strip** (the standard): a band of the material at its real-world size, starting from the material's top row, so every strip keeps one scale (the point of a trim sheet: equal texel density) and a pattern made to fit (four rows of planks for a quarter-tall strip) shows whole pieces. (b) **Shrink to fit**: the whole material made small enough to fill the strip's height and repeated a whole number of times along it, so a round profile becomes a pipe and one dome a row of bolts. Both, chosen per strip (**Strip N fit**). Shrinking first averages the material down by the shrink (blocks of pixels), so it never flickers.
- **Edges.** Strips that meet with a hard line read as one painted picture, not as pieces. **Gap** (cm), **Edges** (Square, Chamfered, Rounded), **Edge width** (cm) and **Depth** sink each strip's top and bottom edges into the gap; the normals bend by the sink as the weathering boxes raise theirs, occlusion darkens the gaps and curvature brightens the bevels' tops, so Dirt and Edge wear after the sheet follow the grooves. Rows of pixels part strip and part gap are mixed by how much of the row each covers, so strip edges between whole pixels stay smooth.
- **Outputs.** **Sheet** (the material) and **Edges** (the gaps and bevels as a shape, to weather only there). A third, a grey per strip, was tried and left out: it jumps from the last strip to the first at the texture's edge, so it doesn't tile, and Edges plus weathering each strip before the sheet covers what it was for.
- **On a model.** Graph materials are laid by their real-world size and Tiling (decisions 0260, 0270), which would scatter a trim sheet's strips anywhere. A graph whose Material out is fed by a Trim sheet (straight or through other boxes, `matgraph::is_trim_sheet`) is laid **once across the texture square**, whatever its real-world size or Tiling, in the painter's graph fills, given materials and the Material mode's preview, so its strips stay on the rows the model's UVs were put on. The painter's graph fill rows say Tiling doesn't change it.
- **Tiling.** Every strip repeats left to right because every material does. Top to bottom, the last strip meets the first in the middle of a gap, so the sheet still tiles as the crate's rules ask; a narrow gap leaves a sharp groove at that join, which the seam check may notice (a wider gap gives it a flat floor).

## Decision

- **Trim sheet** (`trim_sheet`, Finish, after Mix materials; 61 standard boxes): inputs `strip1`…`strip8` and `between` (materials; `needs_an_input`); outputs `material_out` ("Sheet") and `edges`; settings `lay` (0 The project's trim rows, 1 Heights of their own), `height1`…`height8` (1 to 16), `fit1`…`fit8` (0 Cut to the strip, 1 Shrink to fit), `gap` (0 to 20 cm, 0.5), `edges` (Square, Chamfered, Rounded: Rounded), `edge_width` (0 to 20 cm, 1.5), `depth` (0 to 1, 0.3), `gap_colour`. Plain-words refusals when no strip has a material, or every strip plugged in is past the project's last row.
- `matgraph::trim`: `TrimBand` (name, from and to as shares from the top), `TrimRows` (sorted, bad rows left out, a key), `uses_trim_rows`, `is_trim_sheet`; `Registry`, `Job`, `Given` and own boxes carry the rows like the palettes. `boxes::trim_sheet::layout` / `layout_of` / `describe` for the interface.
- **Attributes** (`ui::material::trim_rows`): where each strip lies in plain words ("Strip 2: 12.5% to 50.0% down, on the row Bolts, cut to the strip"), strips with no row, and **Make these the project's trim rows** (`Command::SetTrim`, one undo step).
- A **Trim sheet** starter graph: bolts and a pipe shrunk to fit, a painted panel and planks cut.
- **No new keys or mouse actions; no file format change.**

## Consequences

A trim sheet is one box, and the texture and the UV Editor's rows stay in step both ways. Graph fill layers whose graph reads the rows are made again when a row changes. A strip can't be laid at an angle or along a column (rows run across, as decision 0106's do); a strip cut from a material shows its top band, so moving which part shows is a Transform box before it. A later milestone could add a UV tool that puts a selected strip on the row matching a chosen strip, and milestone 27's health check could warn when the rows and a trim sheet's strips disagree.
