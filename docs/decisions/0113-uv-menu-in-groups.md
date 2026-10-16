# 0113. The UV menu in groups

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0108](0108-uv-powerhouse-and-baking.md) (the Phase 8 plan), [0112](0112-locked-pieces-and-piece-settings.md) (locked pieces)

## Problem

After Phase 8's first four milestones, the main menu bar's **UV** dropdown held about thirty entries and only just fit a 900-pixel-high window. Twenty-four milestones still add UV tools, so it had to be reorganised before any more were built.

## Options

The project lead was shown three mock-ups:

1. **Grouped submenus (recommended):** Seams ▸, Turn and flip ▸, Lay out ▸, Pieces ▸, then Texture ▸, Trim sheet ▸ and Check for baking…, then Settings ▸. One extra hover for every command, but the menu stays short however many tools arrive, and each group is easy to guess.
2. **A flat menu with headings:** every command still visible at once, but it keeps growing and would soon not fit.
3. **Separate top-level menus** (Seams, Pieces, Layout on the menu bar): one fewer click, but the menu bar is already wide and would overflow on smaller screens.

## Decision

Option 1, the project lead's choice:

- **Seams:** Mark seams, Clear seams, Seams from hard edges, Suggest seams, Select seams; then Prefer seams here, Never cut here, Clear seam preference.
- **Turn and flip:** the four turns and flips.
- **Lay out:** Unwrap, the two Unfold commands, Relax, Straighten, Pack; then Set and Match texel density, and Put on trim row.
- **Pieces:** Pin, Unpin; then Lock pieces, Unlock pieces, Piece settings and Stretch budget.
- **Texture**, **Trim sheet** and **Check for baking…** stay as before.
- **Settings:** the five settings submenus of the automatic tools (decision 0110), unchanged inside.

The group names are constants in `ui::workbench` (`UV_SEAMS_MENU` and the rest), so tests and later milestones refer to them rather than retyping them. Nothing else changes: no commands were renamed, added or removed, and no keys changed.

## Consequences

- Every UV command is one hover deeper. The E ring still gives one-key access to the common ones.
- New UV tools from later milestones go into the group they fit, or a new group, rather than the top of the menu.
- Manual pages name the full path ("UV → Lay out → Pack").
