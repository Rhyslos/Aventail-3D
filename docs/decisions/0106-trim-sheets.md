# 0106. Trim sheets

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0095](0095-uvs-before-advanced-modelling.md) (the UV phase plan), [0099](0099-unwrap-relax-straighten.md) (straight strips), [0100](0100-texture-size-and-texel-density.md) (texture settings), [0105](0105-bake-readiness-for-uvs.md) (the UV bake check)

## Problem

Game artists texture much of a model with a **trim sheet**: one shared texture made of horizontal rows (a strip of bolts, a panel edge, a rubber seal) that repeat left to right. Strips of the model (a bevel, a cylinder's side, a panel's border) are laid straight along a row instead of getting texture of their own, so many props share one texture and stay sharp. The roadmap (wish W16): define the rows, then lay a strip straight onto a row, straightened, scaled to fit its height, moved along it.

## Options

- **What a trim sheet is:**
  - **one per project, its rows full-width bands, each a name and where it starts and ends as shares of the texture's height** (chosen). Saved with the project (file format 33), changed as one undo step (a drag or a run of typing merged). Shares rather than pixels, so a new texture size keeps the rows. The Trim menu shows pixels (from the top, as an image editor does).
  - rows of any width, or a grid of tiles: rarer, and harder to explain. Can come later.
- **Laying a strip:** **Put on trim row** (UV menu), for the pieces holding the selection (whole objects with nothing selected inside them). Each piece is:
  - laid out as a straight grid if it's a strip of square quads (else as it lay, or unwrapped if its UVs were squashed);
  - the right way round, its long side along the row;
  - scaled so its height fills the row exactly;
  - set from the row's left end, the pieces one after another.

  Pieces may run past the square's right side, as the row repeats there. Sliding along the row is the UV Editor's move along U (W, the red arrow).
- **How the edit keeps the row:**
  - **as its band** (bottom, top) (chosen): the edit replays the same however the sheet changes later, as other recorded edits do;
  - by row number: changing the sheet would silently move pieces, or point at a row that's gone.
- **Which pieces count as on the trim sheet:**
  - **any piece whose top and bottom lie on a row's edges** (to within 0.1% of the square), worked out when needed (chosen);
  - a mark stored on polygons: one more thing to keep in step with edits.
- **What leaves trim pieces alone:**
  - Pack: they stay on their rows;
  - the bake check: overlaps, padding, outside the square and uneven density don't apply to them; they repeat and overlap on purpose. Instead it lists trim pieces that share a texture (material) with pieces that aren't on the trim sheet, since the trim texture would land on those too. Select only: choosing the material is the user's.
- **Seeing the rows:** faint bands across the UV Editor's square, every other one stronger, with their names, and a fainter continuation to the right, where they repeat.

## Decision

1. `scene::trim`: `TrimSheet { rows }`, `TrimRow { name, bottom, top }`, `even(n)`, `is_valid`, `row_at`; `MAX_TRIM_ROWS` (64), `MAX_ROW_NAME` (64). `Scene::trim`, `with_trim`; `Command::SetTrim` / `Edit::SetTrim`.
2. `scene::Operation::PlaceOnTrim { faces, bottom, top, anchors }` runs `uv::trim::place`; never mirrored, doesn't mend.
3. `uv::trim`: `place`, `on_row`, `ON_ROW`. `uv::check::UvReport::new` takes the rows to skip.
4. File format 33: `ProjectV33` (version 31's model with the rows) and `OperationV33::PlaceOnTrim`.
5. `ui`:
   - `uv_editor::trim_menu` (the UV Editor's Trim menu and UV → Trim sheet);
   - `UvEditor::paint_trim`;
   - `Workbench::put_on_trim_menu` and `run_put_on_trim` (UV → Put on trim row);
   - Pack and the bake check skip trim pieces (`baking::on_trim`);
   - `Kind::TrimShared`.

## Consequences

- No new keys.
- The UV phase's milestones are all built. Its gate (a prop unwrapped and packed with clean bakes in Substance Painter, its UVs kept through later modelling) waits for the project lead.
- Trim sheets in texturing (painting or importing the sheet itself) come with the texturing phase.
