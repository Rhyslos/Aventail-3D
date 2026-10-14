# 0067. Clearer names, and a README for game developers

- **Status:** Accepted
- **Date:** 2026-10-14
- **Follows:** [0046](0046-our-own-names.md) (our own names)

## Problem

A few names said less than they could. "Fix facing" sat beside "Flip normals" for the same idea. "Show only the selection" was the palette's name for what Ctrl + H calls isolating. "Instance" didn't say it makes a copy. "Select similar facing" was vaguer than what it does.

The README's introduction leaned on "calm" and "Cinema 4D-style" more than on what the app is for, and spoke to people who miss older programs rather than to game developers.

## Decision

1. **Renames** (shown to the user; internal names that files rely on are unchanged):
   - Fix facing → **Fix normals**
   - Show only the selection (the left palette's button) → **Isolate**
   - Instance (the command) → **Duplicate as instance**
   - Select similar facing → **Select same facing**
   - In code, the Slit command and its icon are now `CommandId::Slit` and `Icon::Slit` (they were `Knife`).
2. **README:**
   - The tagline is "A calm, easy-to-learn 3D modeller for game assets."
   - The *Why* section speaks to game developers: one job (game assets), easy to learn, made for game models, into the pipeline, never losing work. It doesn't use "indie" or quotes from users.
   - Cinema 4D and 3ds Max are named once, as inspiration and not a copy.
   - The status paragraph is short and current.
3. **DESIGN.md and the manual:** wording of the same kind was reported to the project lead first, then changed with their approval. The introduction now says DESIGN.md takes inspiration from Cinema 4D and 3ds Max, principle 4 reads *Familiar first, our own way after*, and "like C4D" and "C4D-style" are gone from principle 1, the UI specification's heading, the Phase 0 gate and the modelling menu. Comparisons that say where an idea came from stay. The manual needed nothing.

## Consequences

- Menus, the tool menu, the Keys window and the manual use the new names. Saved projects are untouched, because no stored name changed.
- Past decision records and history keep the names used at the time.
