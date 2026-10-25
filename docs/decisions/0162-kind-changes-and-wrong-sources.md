# 0162. A modifier whose source changes kind is taken off

- **Status:** Accepted (recommended choices: the project lead was asleep, see *Choices to check*)
- **Date:** 2026-10-24
- **Builds on:** [0142](0142-modifier-stack-and-remesh.md) (the modifier stack, its sources, `forget_modifier_sources`), [0144](0144-lattice-cage.md) (a Lattice reads only a cage), [0147](0147-curve-deform-and-shrinkwrap.md) (Bend along curve reads only a curve), [0126](0126-rework-and-first-use.md) (Bake into shape, Apply mirror), [0135](0135-live-booleans.md) (cutters), [0028](0028-ask-about-controls.md) (ask about controls)
- **Replaces part of:** [0144](0144-lattice-cage.md) and [0147](0147-curve-deform-and-shrinkwrap.md) (a file whose Lattice or Bend along curve names an object of the wrong kind is no longer refused as damaged)

## Problem

A review of Phase 10 found that **Bake into shape** or **Apply mirror** on a curve that another shape is bent along turns the curve into a mesh but leaves the shape's **Bend along curve** naming it. A bend can read only a curve, so the scene no longer made sense: the file reader refused the modifier (`ModifierRefusal::BadSource`), and since every save reads its own bytes back, the project could no longer be saved (autosave too). Deleting a source already takes its modifiers off in the same step (0142); changing its kind didn't.

Projects saved before this fix may already hold such a modifier, and the reader refused the whole file.

## Options

When an object's kind changes:
1. **Take off, in the same undo step, every modifier elsewhere that can't read the new kind (chosen),** as deleting does: a Scatter loses that shape, a Lattice or Bend along curve goes. One undo brings back the curve and the bend together.
2. Bake the bend into the bent shape first: keeps the look, but changes another object's base without being asked.
3. Refuse to bake a curve something is bent along: safe, but a surprise with no obvious way out.

A file that already holds such a modifier:
1. **Open it without that modifier, and say so in the status bar (chosen):** nothing else is lost, and the person knows what changed.
2. Keep refusing it as damaged: the whole project is lost for one modifier.

## Decision

- **`Run::edit`** (scene, command.rs): after an `Edit::SetKind` whose kind changes (`std::mem::discriminant`), `forget_unreadable_sources(id)` (command/modifiers.rs) takes out of every other object's modifiers each one that reads `id` and can't read its new kind (`ModifierKind::takes_source`), through `without_sources` as deletion does; and if the new kind has no shape, `id` is taken out of every boolean as a cutter (`forget_cutters`). The edits join the step that changed the kind, so undo and redo keep them together. Every command that changes a kind goes through it: Bake into shape (a curve or an instance into a mesh), Apply mirror, Join. A cage never changes kind (its edits keep it a cage), and nothing reads an instance as a cage or curve.
- **Reading a project** (fileio, `v43::ProjectV43::into_document_noting`): a modifier naming an object that is there but of a kind it can't read is left out, with a note: "Box's Bend along curve modifier was taken off: the object it read is no longer of a kind it can read". A modifier naming itself, a missing object or a loop is still refused as damaged. `fileio::project::from_bytes_noting` and `open_noting` give the notes; the app shows them after "Opened …" in the status bar (`Workbench::opened_noting`). `to_bytes` refuses to hand out bytes that would open with a note, so a save can never lose a modifier quietly.

## Choices to check

Made for the project lead (decision 0028):

- Baking a curve that something is bent along **straightens** the bent shape (the bend is taken off) rather than baking the bend into it or refusing. Bake the bent shape first to keep its bend. Ctrl + Z brings both back.
- A project with such a modifier **opens** with a note in the status bar rather than being refused.
- No new keys or mouse actions.

## Consequences

- A scene always makes sense after any command, so it always saves.
- The manual's Bend along curve page explains both cases.
- Tests: `scene/src/kind_change_tests.rs`, `fileio/src/project/stand_in_tests.rs` (`a_modifier_reading_an_object_of_the_wrong_kind_is_taken_off_with_a_note`), and the updated `v45_tests::damaged_cages_are_refused` and `v48_tests::a_bend_or_wrap_naming_the_wrong_thing_or_a_loop_is_refused_as_damaged`.
