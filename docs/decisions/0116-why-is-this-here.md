# 0116. "Why is this here?"

- **Status:** Accepted
- **Date:** 2026-10-17
- **Follows:** [0108](0108-uv-powerhouse-and-baking.md) (the Phase 8 plan), [0111](0111-seam-preferences-and-stretch-budget.md) (seam preferences and budgets), [0115](0115-seeing-and-checking.md) (seeing and checking)

## Problem

Phase 8's milestone 23, one of the four new ideas: every automatic seam, size and packing choice explains itself and can be changed. The project lead chose that explanations show **only while View → Explain choices is ticked** in the UV Editor, so the editor stays quiet otherwise.

## Options

- **Where seam reasons come from:**
  1. Keep the reasons from the last Suggest seams run, per object (the plan's first idea). Lost on closing the project, and silent about seams from projects opened later or made by other tools.
  2. **Work them out when asked, with the current settings (chosen).** Suggest seams never uses chance (decision 0110), so running it again on the same shape and settings gives the same seams and reasons. A seam it wouldn't make is explained as made by hand, by a ready-made shape or by another tool. Cached per shape and settings; skipped above 20 000 polygons, where it would be slow.
- **How to change what's explained:**
  1. A fix button in the box. The box follows the mouse, so it can't be clicked, and a click to pin it would be a new mouse action to ask about.
  2. **Name the menu command that changes it, in the box (chosen).** No new controls.
- **Pieces:** **explained from what they are now (chosen)**: locked, on a trim row, pinned, with settings of their own, their texel density against the aim (and why Pack makes it smaller), where they are, the warnings on them and their stretch. Recording each pack's scale and place would add data to every edit for little more.
- **Where:** the UV Editor only, for now.

## Decision

1. `uv::seams::suggest_explained(mesh, hard, options)` returns each suggested seam with a `SeamReason`: `HardEdge`, `SharpCorner(angle)`, `ToLieFlat` (cut so a closed or looped piece lies flat), `Preferred` (such a cut along a preferred edge), `StretchBudget(budget)`. `suggest_with` is built on it, so the two can't disagree. `seams::edge_angle` measures a corner.
2. `ui::uv_editor::explain`: **View → Explain choices** (`EXPLAIN_CHOICES`, off at start). While ticked, `point_at` finds a seam within reach of the mouse, else the piece under it; `explanation` words it (`seam_text`, `piece_text`); `paint_explanation` lights the seam green (`theme::UV_EXPLAIN`) and draws the box by the mouse, kept inside the editor. `UvContext::explain` carries Suggest seams' settings and the viewport's hard-edge angle.

## Consequences

- No new keys or clicks; nothing saved.
- If a setting changed since seams were suggested, some seams read as "not one Suggest seams would make"; the manual says to suggest again.
- Explaining in the viewport too, and explaining unwraps' own cuts and mending, can build on `suggest_explained` later.
