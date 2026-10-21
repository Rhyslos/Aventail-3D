# 0121. Rework before the rest of Phase 8

- **Status:** Accepted
- **Date:** 2026-10-21
- **Follows:** [0119](0119-roadmap-after-the-uvs.md) (the roadmap after the UVs), [0120](0120-last-edit-ends-when-the-selection-changes.md) (the Last edit)
- **Replaces part of:** [0119](0119-roadmap-after-the-uvs.md) (Phase 9's place and contents; symmetry and the Mirror modifier)

## Problem

The project lead asked to pause and check what had been built and why: "if we can't defend a design choice, we rework it", before 21 more UV milestones, advanced modelling and texture painting. A review looked at three things:

- the app, used hands-on as a new user would;
- every menu, key and label in the code;
- the decision records, the manual and the quality checks.

**Its findings.** The core holds up: the edit stack, the UV engine and safe files. What gets in the way is the surface:

- **The selection is hard to see.** A selected edge is drawn thinner and duller than the unselected hard edges and seams.
- **Nothing can be looked up.** There is no command search and no link to the manual.
- **Labels mean several things.** Edit → Delete deletes whole objects even in Points mode, and "Bake", "Cut" and "Join" each mean several things.
- **Too many concepts.** There are six modes, about ten ideas about edit history, and settings in many homes.
- **Some keys do harm.** Ctrl+D throws away changes in the save question, and Esc finishes a slit instead of cancelling it.

The quality checks had gaps too:

- **The project lead's gate checks** for Phases 4 to 7 never ran.
- **The automated tests** check each feature's specification, not what a person does across several steps. That is how the Last edit bug (0120) got through.

The review is the project lead's doc *Aventail 3D design and quality review*.

## Options

1. Carry on with Phase 8 and fix things in the first-use phase afterwards. This builds 21 more milestones on a surface that is already hard to learn, each adding concepts and words.
2. **Pause Phase 8 now and do the rework first, as Phase 9, with the planned first-use work folded in (chosen, the project lead's answer).**

## Decision

- **Order:** Phase 8 pauses at 7 of 28. Phase 9 becomes "Rework and first use" and comes now, with ten milestones (DESIGN.md §10). Its gate is a newcomer test: 2–3 people new to the app finish a fixed task list without help. Then Phase 8 continues.
- **The project lead's answers:**
  - He runs the Phase 4–7 gate checks himself from a step-by-step list (the doc *Gate checks for Phases 4 to 7*).
  - "Why is this here?" stays a switch, so it isn't in the way.
  - Object mode goes. Pivot editing becomes an "Edit pivot" switch on the gizmo, leaving five modes. He agreed to change Object mode, and the switch was chosen over renaming it "Pivot" mode.
  - Symmetry becomes a modifier: one mirror per object, at the object's own centre by default. This replaces both the live mirror and the separate Mirror modifier planned in 0119.
  - A key audit is offered before any key changes. **Ctrl always adds to the selection and Shift always removes**: fixed.
  - One development file format until the alpha (his OK, 2026-10-21): the format simply grows during development, his existing projects are converted once, and versioning starts properly at the alpha. This is part of milestone 10.

## Consequences

- Phase numbers don't change. Phase 9 simply comes before the rest of Phase 8.
- Every rework milestone that adds or changes keys or mouse actions still asks first (decision 0028).
- From this phase on, pull requests carry a newcomer checklist: does this add a concept, a mode or a new meaning for a key? Is it on by default? Can a newcomer find it?
