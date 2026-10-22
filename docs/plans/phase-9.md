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

## Where the work stands (2026-10-22, end of the first session)

Branch `rework/phase-9`, pushed; not merged and no pull request yet. All tests, clippy and fmt pass.

**Built:**
- Milestone 2: selection drawn clearest and on top, High contrast setting, Select only visible on as standard, one meaning per label, keys that don't bite (Alt + N for Don't save, Esc cancels a slit, Esc closes the Keys window, no ring over windows), a source check for stray backslashes and "arrives later" text.
- Milestone 3: command search (Space, and the palette's search button), right-click menu on, Help → User manual / Words used in the app / Search / Keys / Developer, View → Show and hide, every placeholder gone.
- Milestone 4: five modes with Edit pivot as a switch, drags in a row stay one edit, one "Bake into shape" verb, no Keep editable, Edit layers tab only when wanted.
- Milestone 5: smoothing per object (Attributes → Smoothing; Mesh → Shading → Smooth by angle), marking edges on an object smoothed by angle keeps its look as marks and switches it to its marks (#114); symmetry belongs to the object (pivot, its axes). The development file format (version 39, CBOR, `project/v39.rs`) stores both.
- Milestone 6 (menus): the Mesh menu in groups (Build, Cut and join, Remove, Shading, Shape and object), Edit without the selection doubles, File without Check for baking (it's in UV), the UV menu in the UV Editor, a UV settings panel beside Attributes in the UV Edit layout (#115), "pieces" everywhere instead of "islands".
- Milestone 7 (started): rotation reads X, Y, Z in the axes' colours.
- Milestone 8: the key audit document *Aventail 3D: key audit*, waiting for the project lead's ticks.

**Left:** milestone 6's one home per setting and a slimmer Coordinates band; milestone 7's sliding palette with names, words with icons and the dropdown arrow on the selection mode button; milestone 9 (safety net); milestone 10 (split the big files, spec and rules digest, English language package, menu-path check in tooltips); the manual, changelog, decision 0126 and the test steps for everything above, then the pull request.
