# Phase 9 plan: rework and first use

The working plan for Phase 9 ([decision 0121](../decisions/0121-rework-before-the-rest-of-phase-8.md); the roadmap is in [DESIGN.md](../../DESIGN.md#phase-9--rework-and-first-use-now-before-the-rest-of-phase-8)). The project lead asked for it "in one go" (2026-10-22): one branch, `rework/phase-9`, one pull request, built milestone by milestone as separate commits, with every test step in the shared test document *Aventail 3D: tests for the merged pull requests*.

## Controls chosen (asked 2026-10-22)

| What | Choice |
|---|---|
| Command search | **Space** (free everywhere; animation in Phase 17 will need another play key) |
| Edit pivot switch | **No key**: a button by the gizmo and an entry in the Tools menu (Object mode had no key either) |
| Don't save, in "Save changes?" | **Alt + N** (Ctrl + D is gone: Maya users press it to duplicate) |
| Esc on a slit | **Cancels** it, as Esc does everywhere; **Enter** or **Tab** finish it |
| Esc on the Keys window | Closes it |
| E over a window | Doesn't open the tool ring |
| Key audit | A document first; nothing changes until the project lead ticks each change |
| Fixed | Ctrl adds to the selection, Shift takes away |

## Milestones

1. **Merge and check:** the project lead's gate checks (his document *Gate checks for Phases 4 to 7*). Not code.
2. **Quick fixes:** the selection drawn thickest and brightest, on top, with other overlays dimmed while editing, and a **High contrast** setting; one meaning per label; tooltip menu paths checked by a test; the keys above.
3. **Find and help:** command search (Space); the right-click (quad) menu on by default; Help → User manual; View → Show and hide; test tools in Help → Developer; no "arrives later" placeholders.
4. **Five modes, the edit history shown simply:** Object mode replaced by an Edit pivot switch; one "Bake into shape" verb; no "Keep editable"; drags in a row merged into one edit; the Edit layers tab hidden until a layer is added.
5. **Hard edges and symmetry per object:** smoothing by angle as a per-object setting that hand-marked hard edges add to (#114); symmetry as a per-object Mirror at the object's own centre, moving with it.
6. **Menus, words, settings:** grouped Mesh menu, no doubles between Edit and Select, File keeps to files; a glossary; one home per setting; UV settings in their own panel and the UV menu in the UV Editor (#115); Select only visible on by default; a slimmer Coordinates band.
7. **Labels on everything:** a side palette that slides out with names; words with icons where there's room; a dropdown arrow on the selection mode button; X/Y/Z for rotation.
8. **Key audit:** a document for the project lead.
9. **Safety net:** workflow tests as user stories, random action sequences checked against rules, reference images, the glTF validator and a Godot import in CI, a nightly heavy-mesh check, an emergency copy on a crash, a newcomer checklist in the pull request template.
10. **Tidy for growth:** the big interface files split, a short "what exists now" spec and a digest of the rules in force, English as the one language package, one development file format until the alpha.

**Gate (the newcomer test):** 2–3 people new to the app finish a fixed task list without help, see what is selected on any screen, and no text skips the English package. Then Phase 10 ([decision 0125](../decisions/0125-modelling-before-the-rest-of-the-uvs.md)).
