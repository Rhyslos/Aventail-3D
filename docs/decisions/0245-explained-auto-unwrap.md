# 0245. Explained auto-unwrap

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Phase 8 milestone 19. Unwrapping a model well takes the same six tools in the same order every time (suggest seams, unwrap, straighten, texel density, pack, lightmap UVs). Doing them one by one is slow, but a single button that does everything hides what happened and can't be partly taken back.

## Decision

- **UV → Auto-unwrap** (first in the UV menu): a checkbox per step (all on as standard, remembered with the UV settings as `UvSettings::auto_steps`) and **Auto-unwrap the selection**. No keys.
- **Each step is an ordinary edit**, recorded with the same operations as the tools themselves (`SetSeams` and a live unwrap from Suggest seams, `Unwrap`, `StraightenUvs`, `SetTexelDensity` at the project's density, `PackUvs` with every piece, `MakeLightmap`), on the whole of each selected object. They go into **a new edit layer called "Auto-unwrap"** (`Command::AddLayer`, then `LayerChange::Rename`) so they stay apart from the model's own edits, and the whole run is **one undo step** (one merge key).
- **The Auto-unwrap window** (top right, out of the model's way) lists the steps with what each did ("24 seams (24 edges changed)", "6 pieces", "512 px/m", "6 pieces packed, 61% of the texture used", "A lightmap UV set…"), each with a checkbox that switches that step's edits off or on (`Command::SetEditEnabled`, one undo step each). A step that recorded nothing (Straighten with no piece of quads) says why and can't be switched. **Show the last auto-unwrap** opens it again. Changing a step's settings is done in the Edit layers panel, as for any edit.
- No new file format: layers and edits were always saved.

## Consequences

- Switching an early step off (Suggest seams, say) leaves the later ones in place; they find their polygons again and are made again on the new shape, as edits in a layer always are.
- Milestone 20 (recipes) saves `AutoSteps` with the UV settings under a name.

## Amended 2026-11-06, intents and the density step

- **Straighten** and **Set texel density** now give the pieces the intents the menu tools give (decision 0248): Strip, then Density, in the same step, so switching the step off takes its intent with it. With **Pack** on, Pack scales every piece afterwards, so the Density intent is recorded with Pack, at the density each object's pieces have once packed; a density intent at the density set before Pack would shrink or grow the packed pieces at the next modelling edit.
- The manual's tip that pieces come out small because Set texel density runs before Pack was wrong: Pack scales every piece together to fill the texture, so the density step only evens out the pieces' sizes against each other. The tip now names padding and long pieces, and says to untick Pack to keep the project's density exactly.
