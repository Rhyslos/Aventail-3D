# 0110. Fully controllable automation

- **Status:** Accepted
- **Date:** 2026-10-16
- **Follows:** [0108](0108-uv-powerhouse-and-baking.md) (the Phase 8 plan), [0098](0098-seams.md) (suggested seams), [0099](0099-unwrap-relax-straighten.md) (unwrap and relax), [0101](0101-packing.md) (packing), [0104](0104-uvs-that-survive-modelling.md) (mending), [0109](0109-pins-and-unwrap-methods.md) (unwrap methods)

## Problem

Phase 8's second milestone, asked for by the project lead: "fully featured so there is no way to have the automated tasks be random". Every automatic UV tool must show all of its settings, and the same model and settings must always give the same result. The tools are suggest seams, unwrap (with its cut-open of closed pieces), relax, pack and mending.

Before this, several rules were fixed inside the code:

- the corner angle that gets a seam (66°), and always cutting hard edges;
- where cuts hide (down and back);
- how square a strip must be (25°);
- relax's 60 steps;
- the packing grid's fineness;
- mending's limits (3× stretched, 3× off in size).

## Options

- **Where the settings live:**
  - **the UV menu's last section, "Settings of the automatic tools", with one submenu per tool:** Seam suggestions, Unwrapping, Relaxing, Packing and Mending (chosen). The existing Packing submenu moves there. M1's Unwrap method and the two unwrap switches move into Unwrapping. It's one place to look, and each tool's settings sit together.
  - a settings window: one more window to open, and further from the tools.
  - Settings → UV: far from where the tools are used.
- **Keeping results the same:**
  - **every setting that changes what a recorded edit does is stored in the edit** (chosen):
    - `Operation::Unwrap` now carries `uv::UnwrapOptions` whole: strips, how square, method, one-way, where cuts hide;
    - `RelaxUvs` carries its steps.

    Changing a setting later never changes earlier work, and a project opens the same everywhere. Suggest seams and Pack already record their results (seam marks and each island's move), so their settings needn't be stored.
  - **Mending is the project's** (chosen): it runs inside every modelling edit as the shape is worked out, so its settings can't sit on one edit. They're saved with the project (file format 35), and changed with `Command::SetMending` as one undo step. The shape cache works every shape out again when they change (`EvalContext` now carries them).
  - the app's settings for mending: two people opening one project would see different UVs.
- **Never random:**
  - Nothing in the UV tools uses chance or the clock.
  - The hash maps they use (whose order changes from run to run) are only looked up, never walked in order. Every tie is settled by element ids.
  - Tests run each tool several times with fresh maps and compare the results bit for bit.
- **Hide cuts:**
  - **underneath and at the back** (standard), **underneath**, **at the back**, or **anywhere** (the shortest cuts, every edge weighed alike) (chosen). "Underneath" and "the back" are the object's own −Z and +Y.
  - the same choice applies when Unwrap cuts a closed piece open, so the two tools agree.
- **Pack's fit:**
  - **Fast** (half the standard grid), **Standard** (as fine as half the padding, 128 to 512 cells across) or **Best** (1,024 cells) (chosen).

## Decision

1. `uv::seams`: `SuggestOptions { corner_angle, hard_edges, hide }`, `HideSeams`, `suggest_with`. `disk_cuts` takes `hide`.
2. `uv::unwrap::UnwrapOptions` gains `strip_angle` (0 to `MAX_STRIP_ANGLE`, 60°) and `hide`, with `is_plain` and `is_real`. `MAX_RELAX_STEPS` (1,000).
3. `uv::mend`: `MendOptions { on, uneven, size_off }` (limits `LIMITS`, 1.5 to 20) and `mend_with`.
4. `scene`:
   - `Operation::Unwrap { faces, options, anchors }` and `RelaxUvs { faces, steps, anchors }`;
   - `Scene::mending` and `Command::SetMending` / `Edit::SetMending`;
   - `EvalContext { mending }`; the cache rebuilds an object whose context changed.
5. File format 35: `ProjectV35` adds the mending settings. `OperationV35` stores unwraps with every choice and relaxes with their steps. Edits older versions can store are stored as they stored them.
6. `ui`:
   - `UvSettings` gains `strip_angle`, `seam_corner_angle`, `seam_hard_edges`, `seam_hiding` (`SeamHiding`), `relax_steps` and `pack_quality` (`PackQuality`);
   - `workbench::automation` holds the five submenus;
   - Suggest seams, the bake check's first-UVs fix, Unwrap, Relax and Pack all read them.

## Consequences

- No keys or mouse actions.
- The UV menu's two old switches and the Unwrap method submenu (milestone 1) moved into UV → Unwrapping.
- Milestone 4's per-piece settings will lay a piece's own choices over these.
