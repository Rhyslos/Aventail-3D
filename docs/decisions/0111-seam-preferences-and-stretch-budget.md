# 0111. Seam preferences and stretch budgets

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0108](0108-uv-powerhouse-and-baking.md) (the Phase 8 plan), [0098](0098-seams.md) (suggested seams), [0110](0110-controllable-automation.md) (controllable automation), [0028](0028-ask-about-controls.md) (ask about controls)

## Problem

Phase 8's third milestone. The project lead asked for:

- seam preferences: "prefer a seam here" and "never a seam here";
- a stretch budget per piece.

Every automatic tool must obey both.

## Options

- **Controls:** these add menu commands only: no keys and no mouse actions, so nothing needed asking. The commands are:
  - **Prefer seams here**, **Never seams here** and **Clear seam preference** in the UV menu, beside the seam commands;
  - a **Stretch budget** submenu, holding a value with **Give the selection this budget** and **Clear the selection's budget**.

  The E ring is full; the project lead can ask for places or keys later.
- **What "prefer" means:**
  - **where a needed cut goes first** (chosen): the edge counts as the least visible of all (−10 against about 0 to 4), so hidden paths and spanning trees cut it before any other. It doesn't make a seam on its own: "mark it a seam" already does that.
  - always cut: the same as a seam mark.
- **What "never" means:**
  - **no tool cuts it** (chosen). Suggest seams leaves it out of the hard edges and sharp corners it cuts, and Seams from hard edges skips it. For pieces that must be cut to lie flat, it counts as the most visible edge there is (1,000): spanning trees keep it joined first, and hidden paths avoid it. So it's cut only when a piece couldn't lie flat otherwise (every way out is "never"). Budget slits never cross it. Seams marked by hand still go anywhere.
- **Where preferences are kept:** an edge layer on the shape, `seam_preference` (1 prefer, −1 never). The modelling tools carry it along like seams and hard edges, and the layer goes when nothing is marked.
- **What the budget measures:**
  - **each triangle's off-ness:** the greater of how many times more it stretches one way than the other, and how many times bigger or smaller (in length) it is than the piece's average. The piece is measured as laid flat by Keep shapes (LSCM), the fastest.
  - **nearly all of the piece** (chosen): the off-ness that only the worst 5% of the piece's area goes beyond. The plain worst triangle was tried first. A slit's tip always squeezes a little, so the worst never fell and slitting stopped at once. A percentile reads as "nearly all of the piece is within 1.5×", and a single sliver can't hold a whole piece over its budget.
- **How a piece is brought within budget:**
  - **slits** (chosen), one at a time, from the point inside the piece that is most squeezed or stretched one way. A curved piece laid flat squeezes in its middle and spreads at its border, and a slit from the squeeze lets it open out. Each slit runs to the border along the most hidden way, so it obeys the "hide cuts" setting and preferences.
  - A slit is kept only if it lowers the measure. The search stops when the piece is within budget, nothing more helps, or after 24 slits.
  - Each piece stays one piece (a slit opens a disk without splitting it).
  - Ties go by element ids, so the same shape and budget always give the same slits.
  - The other way, cutting the piece into separate pieces, was not chosen: it loses more of the layout.
- **Where budgets live:**
  - **a budget for every piece** in UV → Seam suggestions (off as standard, 2× when on). It's recorded in each unwrap's options, as decision 0110 requires.
  - **a piece's own budget:** a polygon layer, `uv_budget` (0 for none). A piece takes the least of its polygons' budgets. **Give the selection this budget** sets it on whole pieces, so a piece has one.
- **Which tools obey:**
  - Suggest seams and Unwrap (its cuts are marked as seams, like its cut-open of closed pieces) obey budgets.
  - Preferences are obeyed by those two, by Seams from hard edges, and by mending's lay-out of new pieces, which uses Unwrap's cut-open.
  - Mending doesn't hold pieces to budgets: new seams during modelling would surprise.
- **Seeing them:** blue for "never" and pale orange for "prefer". They're dashed in the UV Editor and thinner in the viewport, wherever seams show (the UV mode, Display → Seams).

## Decision

1. `mesh`:
   - `attributes::SEAM_PREFERENCE` and `UV_BUDGET`;
   - `uvs::SeamPreference`, `seam_preference`, `seam_preferences` and `set_seam_preference`;
   - `uvs::budgets`, `budget_of`, `set_budget` and `BUDGET_RANGE` (1.05 to 10).
2. `uv`:
   - `seams::visibility` reads preferences (`PREFERRED`, `NEVER`);
   - `SuggestOptions` and `UnwrapOptions` gain `budget`;
   - `seams::budget_cuts` (at most `MAX_BUDGET_CUTS`, 24);
   - `methods::stretch_parts` and `nearly_all` (`BUDGET_SPARE`, 5%);
   - `unwrap::cuts_of` is public.
   - The distortion measure now treats a layout mirrored as a whole as even (laying flat may give either; placing turns it back). Only triangles turned over against the rest are priced.
3. `scene`:
   - new operations: `SetSeamPreference { edges, preference, anchors }` (mirrored with symmetry like seams) and `SetStretchBudget { faces, budget, anchors }` (not mirrored);
   - `numbers_are_real` checks budgets.
4. File format 36: `OperationV36` stores preferences, budgets and unwraps with a budget.
5. `ui`:
   - `workbench::seam_preferences`: the commands and the submenu;
   - `UvSettings` gains `budget_on`, `budget` and `piece_budget`;
   - drawing: `theme::UV_SEAM_PREFER` / `UV_SEAM_NEVER` and `OverlayStyle::SeamPrefer` / `SeamNever` (coloured in `app::gpu`).

## Consequences

- No new keys.
- A strict budget on a big piece costs an unwrap per slit (24 at most).
- Milestone 4's per-piece settings will sit beside the piece budget.
